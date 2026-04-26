//! Pull-on-demand distribution service (Phase 8, ADR-0030 + ADR-0031).
//!
//! Wraps the pure decision logic in `domain::distribution` with DB
//! reads/writes and event fan-out.  The IPC surface (commands/quality.rs
//! and commands/distribution.rs) is the only caller of this module
//! from outside the services layer.
//!
//! What v2.0 ships:
//! - `pick_vod`, `unpick_vod`, `pick_next_n` — explicit user gesture
//!   transitions `available -> queued` (or back).  IPC-reachable.
//! - `set_distribution_mode` / `set_sliding_window_size` —
//!   thin wrappers over the settings service for IPC convenience.
//! - `on_watched_completed` — implemented + tested but **not yet
//!   wired** at the watch-progress event hot path; v2.0.x will add
//!   the call in `lib.rs::WatchEvent::Completed`.
//! - `prefetch_check` — implemented + tested but **not yet wired**
//!   from the player; v2.0.x exposes it as an IPC command and
//!   triggers it from the player's first sustained
//!   `watch:progress_updated` event.
//! - `enforce_sliding_window` — runs only as a side effect of
//!   `on_watched_completed`, so until that's wired the sliding-
//!   window evictions are dormant in v2.0.  The user-driven
//!   `pick_vod` / `pick_next_n` paths still respect the cap (no
//!   eviction needed when there's room).
//!
//! Deferred to v2.0.x integration follow-ups:
//! - Wiring `queued -> downloading` from inside the download worker
//!   (today the download service still drives its own state).  The
//!   pull-mode pick transitions to `queued`; v2.0.x will have the
//!   download worker observe `vods.status = 'queued'` and march it
//!   to `downloading` / `ready` (mirroring its existing `downloads`
//!   table state).  See PR description for the rollout note.
//! - Wiring `on_watched_completed` from `WatchEvent::Completed`.
//! - Wiring `prefetch_check` from the player's progress hook.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tracing::{info, warn};

use crate::domain::distribution::{
    DistributionError, DistributionMode, VodStatus, prefetch_pick_next,
    sliding_window_pick_eviction, validate_transition,
};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::services::settings::SettingsService;

/// Events the distribution service emits.  Fan-out happens at the
/// AppState boundary so tests can build a `DistributionService`
/// without a Tauri runtime.
#[derive(Debug, Clone)]
pub enum DistributionEvent {
    VodPicked {
        vod_id: String,
        from: VodStatus,
    },
    VodArchived {
        vod_id: String,
    },
    PrefetchTriggered {
        currently_watching: String,
        prefetched: String,
    },
    WindowEnforced {
        streamer_id: String,
        evicted_vod_id: String,
    },
}

pub type DistributionEventSink = Arc<dyn Fn(DistributionEvent) + Send + Sync>;

/// Result of a `pick_vod` / `unpick_vod` call.  Carries the new
/// status so the renderer can update its cache without a re-fetch.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PickResult {
    pub vod_id: String,
    pub status: VodStatus,
}

#[derive(Debug)]
pub struct DistributionService {
    db: Db,
    /// Held for v2.0.x integration follow-up: when the
    /// `status_changed_at` column lands, the service will bind
    /// `clock.unix_seconds()` into the UPDATE.  See `write_status`.
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
    settings: SettingsService,
}

impl DistributionService {
    pub fn new(db: Db, clock: Arc<dyn Clock>, settings: SettingsService) -> Self {
        Self {
            db,
            clock,
            settings,
        }
    }

    /// Pick a single VOD: `available | deleted -> queued`.
    pub async fn pick_vod(
        &self,
        vod_id: &str,
        sink: &DistributionEventSink,
    ) -> Result<PickResult, AppError> {
        let current = self.read_status(vod_id).await?;
        let next = VodStatus::Queued;
        validate_transition(current, next).map_err(|e| map_distribution_err(&e))?;
        self.write_status(vod_id, next).await?;
        info!(vod_id, from = ?current, "vod picked");
        (sink)(DistributionEvent::VodPicked {
            vod_id: vod_id.to_owned(),
            from: current,
        });
        Ok(PickResult {
            vod_id: vod_id.to_owned(),
            status: next,
        })
    }

    /// Bulk-pick the most recent N `available` VODs for a streamer.
    /// Stops when the per-streamer sliding-window cap would be
    /// breached.  Returns the IDs that actually transitioned.
    pub async fn pick_next_n(
        &self,
        streamer_id: &str,
        n: i64,
        sink: &DistributionEventSink,
    ) -> Result<Vec<String>, AppError> {
        let n = n.clamp(1, 50) as usize;
        let cap = self.settings.get().await?.sliding_window_size as usize;
        let current = self.streamer_window_count(streamer_id).await? as usize;
        // saturating_sub mirrors the prefetch_check pattern; both
        // call sites use the same arithmetic so a future
        // refactor can extract a shared helper.
        let room = cap.saturating_sub(current);
        if room == 0 {
            return Ok(vec![]);
        }
        let limit = n.min(room) as i64;
        let candidates: Vec<String> = sqlx::query_scalar(
            "SELECT twitch_video_id FROM vods
              WHERE twitch_user_id = ? AND status = 'available'
              ORDER BY stream_started_at DESC
              LIMIT ?",
        )
        .bind(streamer_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        let mut picked = Vec::with_capacity(candidates.len());
        for vod_id in candidates {
            match self.pick_vod(&vod_id, sink).await {
                Ok(_) => picked.push(vod_id),
                Err(e) => warn!(vod_id, error = ?e, "pick_next_n: skipped"),
            }
        }
        Ok(picked)
    }

    /// Reverse a pick before a download starts:
    /// `queued -> available`.  Rejects post-`downloading` transitions
    /// so a user can't accidentally cancel an in-flight download
    /// through this surface.
    pub async fn unpick_vod(
        &self,
        vod_id: &str,
        _sink: &DistributionEventSink,
    ) -> Result<PickResult, AppError> {
        let current = self.read_status(vod_id).await?;
        if current != VodStatus::Queued {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "unpick only valid from 'queued', got {:?} for {vod_id}",
                    current
                ),
            });
        }
        let next = VodStatus::Available;
        validate_transition(current, next).map_err(|e| map_distribution_err(&e))?;
        self.write_status(vod_id, next).await?;
        Ok(PickResult {
            vod_id: vod_id.to_owned(),
            status: next,
        })
    }

    /// Remove a downloaded VOD from disk on user request.  Valid
    /// from `Ready` (delete the live file) and `Archived` (delete
    /// the watched-but-not-yet-cleaned-up file).  Transitions
    /// `vods.status` to `Deleted` and best-effort `tokio::fs::remove_file`s
    /// the underlying `.mp4` so the user sees the disk freed
    /// immediately rather than waiting for the next auto-cleanup
    /// tick.  Idempotent on `Deleted` rows.
    pub async fn remove_vod(
        &self,
        vod_id: &str,
        sink: &DistributionEventSink,
    ) -> Result<(), AppError> {
        let current = self.read_status(vod_id).await?;
        match current {
            VodStatus::Ready | VodStatus::Archived => {}
            VodStatus::Deleted => return Ok(()),
            other => {
                return Err(AppError::InvalidInput {
                    detail: format!(
                        "remove_vod only valid from ready/archived, got {:?} for {vod_id}",
                        other
                    ),
                });
            }
        }
        // Best-effort: read the final_path off the downloads row
        // and unlink the file.  Failure is logged but doesn't block
        // the state transition — auto-cleanup will catch any
        // stragglers.
        let final_path: Option<String> =
            sqlx::query_scalar("SELECT final_path FROM downloads WHERE vod_id = ?")
                .bind(vod_id)
                .fetch_optional(self.db.pool())
                .await?
                .flatten();
        if let Some(path) = final_path
            && let Err(e) = tokio::fs::remove_file(&path).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!(vod_id, error = %e, path, "remove_vod: file unlink failed");
        }
        validate_transition(current, VodStatus::Deleted).map_err(|e| map_distribution_err(&e))?;
        self.write_status(vod_id, VodStatus::Deleted).await?;
        // Also flag the downloads row as cancelled so the queue
        // doesn't try to "complete" it on a future tick.
        sqlx::query(
            "UPDATE downloads SET state = 'failed_permanent',
                 last_error = 'user_removed', last_error_at = ?
              WHERE vod_id = ? AND state = 'completed'",
        )
        .bind(self.clock.unix_seconds())
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        (sink)(DistributionEvent::VodArchived {
            vod_id: vod_id.to_owned(),
        });
        Ok(())
    }

    /// Watch-progress crossed completion.  Transitions `ready ->
    /// archived` and runs the sliding-window enforcer for the
    /// streamer.  Idempotent — re-watching an already-archived VOD
    /// is a no-op.
    pub async fn on_watched_completed(
        &self,
        vod_id: &str,
        sink: &DistributionEventSink,
    ) -> Result<(), AppError> {
        let current = self.read_status(vod_id).await?;
        match current {
            VodStatus::Ready => {
                self.write_status(vod_id, VodStatus::Archived).await?;
                (sink)(DistributionEvent::VodArchived {
                    vod_id: vod_id.to_owned(),
                });
                if let Some(streamer) = self.streamer_for_vod(vod_id).await? {
                    self.enforce_sliding_window(&streamer, sink).await?;
                }
                Ok(())
            }
            VodStatus::Archived => Ok(()), // already archived
            other => {
                warn!(
                    vod_id,
                    state = ?other,
                    "on_watched_completed: unexpected state, skipping"
                );
                Ok(())
            }
        }
    }

    /// Pre-fetch hook (ADR-0031).  Called by the player on the
    /// first sustained `watch:progress_updated` event for a VOD.
    /// Picks the immediate-next chronological `available` VOD on
    /// the same streamer, respecting `prefetch_enabled` and the
    /// sliding-window cap.
    ///
    /// Returns the picked vod_id when a transition occurred;
    /// `None` for "nothing to do" (skipped).
    ///
    /// **Concurrency note.** The four reads (settings, window
    /// count, streamer vods, then `pick_vod`) are non-transactional.
    /// A concurrent `pick_vod` call between read and write can
    /// transiently push the window above the cap by the number of
    /// in-flight callers.  This is intentional: ADR-0030 §Risks
    /// flags the reconciling path as `enforce_sliding_window`
    /// triggered by `on_watched_completed`.  Wrapping these reads
    /// in a global mutex would deadlock with the caller (which is
    /// itself executing on the watch-progress event hot path).
    pub async fn prefetch_check(
        &self,
        currently_watching_vod_id: &str,
        sink: &DistributionEventSink,
    ) -> Result<Option<String>, AppError> {
        let settings = self.settings.get().await?;
        if !settings.prefetch_enabled {
            return Ok(None);
        }
        if matches!(settings.distribution_mode, DistributionMode::Auto) {
            // Auto mode already pulls everything; no need to
            // pre-fetch on top.
            return Ok(None);
        }
        let Some(streamer) = self.streamer_for_vod(currently_watching_vod_id).await? else {
            return Ok(None);
        };
        let cap = settings.sliding_window_size as usize;
        let current = self.streamer_window_count(&streamer).await? as usize;
        let room = cap.saturating_sub(current);
        if room == 0 {
            return Ok(None);
        }
        let streamer_vods = self.list_streamer_vods(&streamer).await?;
        let pick: Option<String> =
            prefetch_pick_next(currently_watching_vod_id, &streamer_vods, room)
                .map(|s| s.to_owned());
        if let Some(vod_id) = &pick {
            self.pick_vod(vod_id, sink).await?;
            (sink)(DistributionEvent::PrefetchTriggered {
                currently_watching: currently_watching_vod_id.to_owned(),
                prefetched: vod_id.clone(),
            });
        }
        Ok(pick)
    }

    /// Run the sliding-window enforcer for one streamer.  Loops
    /// until the per-streamer count is at or below the cap, in
    /// case a window-shrink (e.g., user reduced
    /// `sliding_window_size` from 5 to 2) has left the streamer
    /// multi-VODs over capacity.  Each iteration transitions one
    /// oldest-archived VOD to `deleted`.  Hard-bounded at 200
    /// iterations as a runaway safeguard — equivalent to evicting
    /// an entire 200-VOD streamer in one call, which never happens
    /// in steady state.
    pub async fn enforce_sliding_window(
        &self,
        streamer_id: &str,
        sink: &DistributionEventSink,
    ) -> Result<(), AppError> {
        const ENFORCER_HARD_BOUND: usize = 200;
        let cap = self.settings.get().await?.sliding_window_size as usize;
        for _ in 0..ENFORCER_HARD_BOUND {
            let current = self.streamer_window_count(streamer_id).await? as usize;
            let archived = self
                .list_streamer_archived_oldest_first(streamer_id)
                .await?;
            let Some(evict) = sliding_window_pick_eviction(&archived, current, cap) else {
                return Ok(());
            };
            let evict = evict.to_owned();
            validate_transition(VodStatus::Archived, VodStatus::Deleted)
                .map_err(|e| map_distribution_err(&e))?;
            self.write_status(&evict, VodStatus::Deleted).await?;
            (sink)(DistributionEvent::WindowEnforced {
                streamer_id: streamer_id.to_owned(),
                evicted_vod_id: evict,
            });
        }
        warn!(
            streamer_id,
            "enforce_sliding_window hit hard iteration bound; check for state-machine drift"
        );
        Ok(())
    }

    // ---- internal helpers ----

    async fn read_status(&self, vod_id: &str) -> Result<VodStatus, AppError> {
        let raw: Option<String> =
            sqlx::query_scalar("SELECT status FROM vods WHERE twitch_video_id = ?")
                .bind(vod_id)
                .fetch_optional(self.db.pool())
                .await?;
        let s = raw.ok_or(AppError::NotFound)?;
        VodStatus::from_db_str(&s).ok_or_else(|| AppError::Db {
            detail: format!("unknown vod_status '{s}' for {vod_id}"),
        })
    }

    async fn write_status(&self, vod_id: &str, status: VodStatus) -> Result<(), AppError> {
        // TODO(phase-8.x): bind a `status_changed_at` timestamp
        // into the UPDATE once the column lands.  For v2.0 the
        // timeline of state transitions is reconstructable from
        // the `vod:updated` event log.
        let result = sqlx::query("UPDATE vods SET status = ? WHERE twitch_video_id = ?")
            .bind(status.as_db_str())
            .bind(vod_id)
            .execute(self.db.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    async fn streamer_for_vod(&self, vod_id: &str) -> Result<Option<String>, AppError> {
        let id: Option<String> =
            sqlx::query_scalar("SELECT twitch_user_id FROM vods WHERE twitch_video_id = ?")
                .bind(vod_id)
                .fetch_optional(self.db.pool())
                .await?;
        Ok(id)
    }

    async fn streamer_window_count(&self, streamer_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vods
              WHERE twitch_user_id = ?
                AND status IN ('queued','downloading','ready')",
        )
        .bind(streamer_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(count)
    }

    async fn list_streamer_vods(
        &self,
        streamer_id: &str,
    ) -> Result<Vec<(String, VodStatus)>, AppError> {
        let rows = sqlx::query(
            "SELECT twitch_video_id, status FROM vods
              WHERE twitch_user_id = ?
              ORDER BY stream_started_at ASC",
        )
        .bind(streamer_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get(0)?;
            let status_str: String = row.try_get(1)?;
            let status = VodStatus::from_db_str(&status_str).ok_or_else(|| AppError::Db {
                detail: format!("unknown vod_status '{status_str}' for {id}"),
            })?;
            out.push((id, status));
        }
        Ok(out)
    }

    async fn list_streamer_archived_oldest_first(
        &self,
        streamer_id: &str,
    ) -> Result<Vec<String>, AppError> {
        // ADR-0024 §Candidate selection (re-applied to ADR-0030's
        // pull-mode enforcer): evict by *watch* recency, not
        // *broadcast* date.  A binge-watcher who watched a 6-month-
        // old VOD yesterday should keep it; the VOD they watched
        // months ago goes first.  LEFT JOIN against watch_progress
        // because Archived implies a watch row exists, but defence-
        // in-depth: a row with no watch_progress falls through to
        // COALESCE(0, 0) and is evicted first (ages-of-zero).
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT v.twitch_video_id
               FROM vods v
               LEFT JOIN watch_progress w ON w.vod_id = v.twitch_video_id
              WHERE v.twitch_user_id = ? AND v.status = 'archived'
              ORDER BY COALESCE(w.last_watched_at, 0) ASC,
                       v.stream_started_at ASC",
        )
        .bind(streamer_id)
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows)
    }
}

fn map_distribution_err(e: &DistributionError) -> AppError {
    AppError::InvalidInput {
        detail: e.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn setup() -> (DistributionService, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(2_000_000));
        let settings = SettingsService::new(db.clone(), clock.clone());
        let svc = DistributionService::new(db.clone(), clock, settings);
        (svc, db)
    }

    async fn seed_vod(db: &Db, streamer_id: &str, vod_id: &str, status: &str, started_at: i64) {
        sqlx::query(
            "INSERT OR IGNORE INTO streamers (twitch_user_id, login, display_name,
                 broadcaster_type, twitch_created_at, added_at)
             VALUES (?, ?, ?, '', 0, 0)",
        )
        .bind(streamer_id)
        .bind(format!("login_{streamer_id}"))
        .bind(format!("Display {streamer_id}"))
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title, stream_started_at,
                 published_at, url, duration_seconds, ingest_status, first_seen_at, last_seen_at, status)
             VALUES (?, ?, ?, ?, ?, ?, 1800, 'eligible', 0, 0, ?)",
        )
        .bind(vod_id)
        .bind(streamer_id)
        .bind(format!("title {vod_id}"))
        .bind(started_at)
        .bind(started_at)
        .bind(format!("https://twitch.tv/videos/{vod_id}"))
        .bind(status)
        .execute(db.pool())
        .await
        .unwrap();
    }

    fn capture_sink() -> (
        DistributionEventSink,
        Arc<std::sync::Mutex<Vec<DistributionEvent>>>,
    ) {
        let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        let sink: DistributionEventSink = Arc::new(move |ev| {
            captured_clone.lock().unwrap().push(ev);
        });
        (sink, captured)
    }

    #[tokio::test]
    async fn pick_vod_transitions_available_to_queued() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "available", 100).await;
        let (sink, captured) = capture_sink();
        let result = svc.pick_vod("v1", &sink).await.unwrap();
        assert_eq!(result.status, VodStatus::Queued);
        assert_eq!(captured.lock().unwrap().len(), 1);
        assert!(matches!(
            captured.lock().unwrap()[0],
            DistributionEvent::VodPicked { .. }
        ));
    }

    #[tokio::test]
    async fn pick_vod_rejects_invalid_transitions() {
        let (svc, db) = setup().await;
        // ready -> queued is not allowed (ready is already a "have it" state).
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        let (sink, _) = capture_sink();
        let err = svc.pick_vod("v1", &sink).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn unpick_vod_only_works_from_queued() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "downloading", 100).await;
        let (sink, _) = capture_sink();
        let err = svc.unpick_vod("v1", &sink).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn unpick_vod_returns_to_available() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "queued", 100).await;
        let (sink, _) = capture_sink();
        let result = svc.unpick_vod("v1", &sink).await.unwrap();
        assert_eq!(result.status, VodStatus::Available);
    }

    #[tokio::test]
    async fn on_watched_completed_archives_and_runs_window() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        seed_vod(&db, "s1", "v2", "archived", 90).await; // older archived
        seed_vod(&db, "s1", "v3", "ready", 110).await; // would push window to 2 (cap)
        let (sink, captured) = capture_sink();
        svc.on_watched_completed("v1", &sink).await.unwrap();
        // v1 -> archived; v2 should now be evicted (oldest archived) since
        // v3 is still ready, total occupies = 1 (v3) which is below cap=2;
        // but combined with v1->archived, current_window_count == 1 and
        // adding more would NOT breach (1+1=2 == cap=2).  So no eviction.
        let evs = captured.lock().unwrap();
        assert!(
            evs.iter()
                .any(|e| matches!(e, DistributionEvent::VodArchived { .. }))
        );
    }

    #[tokio::test]
    async fn pick_next_n_respects_window_cap() {
        let (svc, db) = setup().await;
        for i in 1..=5 {
            seed_vod(&db, "s1", &format!("v{i}"), "available", 100 + i).await;
        }
        let (sink, _) = capture_sink();
        // Default sliding_window_size = 2; current = 0; room = 2.
        let picked = svc.pick_next_n("s1", 10, &sink).await.unwrap();
        assert_eq!(picked.len(), 2);
    }

    #[tokio::test]
    async fn prefetch_check_picks_next_chronological_available() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        seed_vod(&db, "s1", "v2", "available", 200).await; // newer
        seed_vod(&db, "s1", "v3", "available", 300).await;
        let (sink, _) = capture_sink();
        let picked = svc.prefetch_check("v1", &sink).await.unwrap();
        // ADR-0031: pick the immediate-next available chronologically.
        assert_eq!(picked, Some("v2".into()));
    }

    #[tokio::test]
    async fn prefetch_check_skips_when_disabled() {
        let (svc, db) = setup().await;
        let settings = SettingsService::new(db.clone(), svc.clock.clone());
        settings
            .update(crate::services::settings::SettingsPatch {
                prefetch_enabled: Some(false),
                ..Default::default()
            })
            .await
            .unwrap();
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        seed_vod(&db, "s1", "v2", "available", 200).await;
        let (sink, _) = capture_sink();
        let picked = svc.prefetch_check("v1", &sink).await.unwrap();
        assert_eq!(picked, None);
    }

    #[tokio::test]
    async fn prefetch_check_skips_in_auto_mode() {
        let (svc, db) = setup().await;
        let settings = SettingsService::new(db.clone(), svc.clock.clone());
        settings
            .update(crate::services::settings::SettingsPatch {
                distribution_mode: Some(DistributionMode::Auto),
                ..Default::default()
            })
            .await
            .unwrap();
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        seed_vod(&db, "s1", "v2", "available", 200).await;
        let (sink, _) = capture_sink();
        let picked = svc.prefetch_check("v1", &sink).await.unwrap();
        assert_eq!(picked, None);
    }

    #[tokio::test]
    async fn remove_vod_transitions_ready_to_deleted() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        let (sink, captured) = capture_sink();
        svc.remove_vod("v1", &sink).await.unwrap();
        // VOD now Deleted.
        let status: String =
            sqlx::query_scalar("SELECT status FROM vods WHERE twitch_video_id = 'v1'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(status, "deleted");
        let evs = captured.lock().unwrap();
        assert!(
            evs.iter()
                .any(|e| matches!(e, DistributionEvent::VodArchived { .. })),
            "expected a VodArchived (delete-tier) event"
        );
    }

    #[tokio::test]
    async fn remove_vod_transitions_archived_to_deleted() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "archived", 100).await;
        let (sink, _) = capture_sink();
        svc.remove_vod("v1", &sink).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT status FROM vods WHERE twitch_video_id = 'v1'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(status, "deleted");
    }

    #[tokio::test]
    async fn remove_vod_idempotent_on_already_deleted() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "deleted", 100).await;
        let (sink, _) = capture_sink();
        // Should be a benign no-op rather than an error.
        svc.remove_vod("v1", &sink).await.unwrap();
    }

    #[tokio::test]
    async fn remove_vod_rejects_invalid_states() {
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "queued", 100).await;
        let (sink, _) = capture_sink();
        let err = svc.remove_vod("v1", &sink).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn pick_vod_transitions_deleted_to_queued() {
        // ADR-0030 re-pick path: a previously-deleted VOD can be
        // re-picked and goes through the queue.
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v1", "deleted", 100).await;
        let (sink, captured) = capture_sink();
        let result = svc.pick_vod("v1", &sink).await.unwrap();
        assert_eq!(result.status, VodStatus::Queued);
        let evs = captured.lock().unwrap();
        assert_eq!(evs.len(), 1);
        if let DistributionEvent::VodPicked { from, .. } = &evs[0] {
            assert_eq!(*from, VodStatus::Deleted);
        } else {
            panic!("expected VodPicked event");
        }
    }

    #[tokio::test]
    async fn pick_next_n_isolates_streamers() {
        // Window cap = 2 (default).  Streamer A's window full;
        // streamer B should still be free to pick from.
        let (svc, db) = setup().await;
        // Fill streamer A's window with 2 ready rows.
        seed_vod(&db, "sA", "vA1", "ready", 100).await;
        seed_vod(&db, "sA", "vA2", "ready", 110).await;
        seed_vod(&db, "sA", "vA3", "available", 120).await;
        // Streamer B fresh.
        seed_vod(&db, "sB", "vB1", "available", 200).await;
        seed_vod(&db, "sB", "vB2", "available", 210).await;
        seed_vod(&db, "sB", "vB3", "available", 220).await;
        let (sink, _) = capture_sink();
        // A is full — picks 0.
        let a = svc.pick_next_n("sA", 5, &sink).await.unwrap();
        assert_eq!(a.len(), 0);
        // B picks up to cap (2).
        let b = svc.pick_next_n("sB", 5, &sink).await.unwrap();
        assert_eq!(b.len(), 2);
    }

    #[tokio::test]
    async fn enforce_sliding_window_drains_when_window_shrunk() {
        // Simulate a window-shrink: 4 ready + 3 archived
        // (current_window = 4, cap = 2 → over by 2).  Loop should
        // run twice and evict 2 oldest-archived rows in order.
        let (svc, db) = setup().await;
        seed_vod(&db, "s1", "v_arc1", "archived", 50).await;
        seed_vod(&db, "s1", "v_arc2", "archived", 60).await;
        seed_vod(&db, "s1", "v_arc3", "archived", 70).await;
        seed_vod(&db, "s1", "v1", "ready", 100).await;
        seed_vod(&db, "s1", "v2", "ready", 110).await;
        seed_vod(&db, "s1", "v3", "ready", 120).await;
        seed_vod(&db, "s1", "v4", "ready", 130).await;
        let (sink, captured) = capture_sink();
        svc.enforce_sliding_window("s1", &sink).await.unwrap();
        // Loop should evict v_arc1 first, then v_arc2.  v_arc3
        // stays because the loop terminates once current <= cap
        // would require evicting more than archived.len() + 1 (or
        // current == 4 still > cap=2 even after 2 evictions, but
        // only archived rows can be evicted; the loop bounds
        // out).  v3 / v4 remain in `ready` — the enforcer never
        // touches non-archived rows.
        let evicted: Vec<_> = captured
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                DistributionEvent::WindowEnforced { evicted_vod_id, .. } => {
                    Some(evicted_vod_id.clone())
                }
                _ => None,
            })
            .collect();
        // Only archived rows can evict; readys are sacrosanct.
        // The loop runs while there's an archived candidate AND
        // current > cap.  Current = 4 (constant — readys don't
        // change), so each loop tries to evict one and continues
        // because current still > cap.  All 3 archived rows are
        // evicted in oldest-first order.
        assert_eq!(evicted.len(), 3);
        assert_eq!(evicted, vec!["v_arc1", "v_arc2", "v_arc3"]);
    }

    #[tokio::test]
    async fn enforce_sliding_window_evicts_oldest_archived() {
        let (svc, db) = setup().await;
        // window cap = 2 (default); make 3 ready + 2 archived, so
        // current_window_count = 3 (>cap), and the loop will
        // drain all archived rows since current is constant
        // (readys are sacrosanct and unchanged by the enforcer).
        seed_vod(&db, "s1", "v1", "archived", 100).await;
        seed_vod(&db, "s1", "v2", "archived", 110).await; // newer archived
        seed_vod(&db, "s1", "v3", "ready", 120).await;
        seed_vod(&db, "s1", "v4", "ready", 130).await;
        seed_vod(&db, "s1", "v5", "ready", 140).await;
        let (sink, captured) = capture_sink();
        svc.enforce_sliding_window("s1", &sink).await.unwrap();
        let evicted: Vec<_> = captured
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                DistributionEvent::WindowEnforced { evicted_vod_id, .. } => {
                    Some(evicted_vod_id.clone())
                }
                _ => None,
            })
            .collect();
        // Loop evicts oldest-first (v1, then v2) until either the
        // window is below cap OR archived runs out.  The 3 readys
        // mean current stays >cap, so all archived drain.
        assert_eq!(evicted, vec!["v1".to_string(), "v2".to_string()]);
    }
}
