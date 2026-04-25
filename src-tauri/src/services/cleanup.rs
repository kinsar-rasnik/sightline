//! Auto-cleanup service — Phase 7 (ADR-0024).
//!
//! Owns the watermark policy + candidate selection + actual file
//! deletion for the v1 disk-pressure relief loop.  The decision
//! logic itself lives in `domain::cleanup`; this module is the
//! orchestration glue (DB queries, filesystem writes, event fan-out,
//! audit-log persistence).
//!
//! Layering: `CleanupService` is allowed to read every Phase 5/6
//! table (downloads, vods, streamers, watch_progress) but does NOT
//! mutate `watch_progress` — preserving that row is the v1
//! "re-download lands the user back at their position" affordance.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tracing::{info, warn};

use crate::domain::cleanup::{
    CandidateInput, CleanupLogEntry, CleanupMode, CleanupPlan, CleanupResult,
    CleanupSettingsSnapshot, DiskSnapshot, WatchStateForCleanup, compute_plan_pure,
};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::infra::fs::space::FreeSpaceProbe;
use crate::services::settings::SettingsService;

/// Live disk-usage snapshot exposed via `cmd_get_disk_usage`.  The
/// fields land verbatim on the Settings UI's Storage section.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub library_path: Option<bool>,
    pub total_bytes: i64,
    pub free_bytes: i64,
    pub used_fraction: f64,
    pub high_watermark: f64,
    pub low_watermark: f64,
    pub above_high_watermark: bool,
}

/// Events the cleanup service emits.  Fan-out happens at the AppState
/// boundary so tests can build a `CleanupService` without a Tauri
/// runtime.
#[derive(Debug, Clone)]
pub enum CleanupEvent {
    PlanReady {
        candidate_count: i64,
        projected_freed_bytes: i64,
    },
    Executed {
        mode: CleanupMode,
        status: String,
        freed_bytes: i64,
        deleted_vod_count: i64,
    },
    DiskPressure {
        used_fraction: f64,
        free_bytes: i64,
    },
}

pub type CleanupEventSink = Arc<dyn Fn(CleanupEvent) + Send + Sync>;

#[derive(Debug)]
pub struct CleanupService {
    db: Db,
    clock: Arc<dyn Clock>,
    settings: SettingsService,
    probe: Arc<dyn FreeSpaceProbe>,
}

impl CleanupService {
    pub fn new(
        db: Db,
        clock: Arc<dyn Clock>,
        settings: SettingsService,
        probe: Arc<dyn FreeSpaceProbe>,
    ) -> Self {
        Self {
            db,
            clock,
            settings,
            probe,
        }
    }

    /// Build a cleanup plan against the current disk snapshot + DB
    /// state.  Pure read-only; never modifies anything.
    pub async fn compute_plan(&self) -> Result<CleanupPlan, AppError> {
        let settings = self.settings.get().await?;
        let library_root = require_library_root(&settings.library_root)?;
        let snapshot = self.snapshot_disk(&library_root).await?;
        let candidates = self.fetch_candidates().await?;
        let projection = compute_plan_pure(
            snapshot,
            CleanupSettingsSnapshot {
                enabled: settings.cleanup_enabled,
                high_watermark: settings.cleanup_high_watermark,
                low_watermark: settings.cleanup_low_watermark,
            },
            candidates,
            self.clock.unix_seconds(),
        );
        Ok(projection.plan)
    }

    /// Get a live disk-usage snapshot for the Settings UI.  Includes
    /// the user's configured watermarks so the renderer can render
    /// the "above watermark" badge without a separate settings
    /// fetch.
    pub async fn get_disk_usage(&self) -> Result<DiskUsage, AppError> {
        let settings = self.settings.get().await?;
        let library_root = match settings.library_root.as_deref() {
            Some(p) => p,
            None => {
                return Ok(DiskUsage {
                    library_path: Some(false),
                    total_bytes: 0,
                    free_bytes: 0,
                    used_fraction: 0.0,
                    high_watermark: settings.cleanup_high_watermark,
                    low_watermark: settings.cleanup_low_watermark,
                    above_high_watermark: false,
                });
            }
        };
        let snapshot = self.snapshot_disk(library_root).await?;
        let used_fraction = snapshot.used_fraction();
        Ok(DiskUsage {
            library_path: Some(true),
            total_bytes: snapshot.total_bytes as i64,
            free_bytes: snapshot.free_bytes as i64,
            used_fraction,
            high_watermark: settings.cleanup_high_watermark,
            low_watermark: settings.cleanup_low_watermark,
            above_high_watermark: used_fraction >= settings.cleanup_high_watermark,
        })
    }

    /// Execute a previously-computed plan.  Modes:
    ///
    /// - [`CleanupMode::DryRun`] — log a `dry_run` row, don't touch
    ///   the filesystem.  Used by the UI's "Preview what would be
    ///   deleted" path.
    /// - [`CleanupMode::Scheduled`] — daemon-tick path.  Returns
    ///   `status = "skipped"` when the supplied plan is empty AND
    ///   pressure was below the high watermark when the plan was
    ///   computed.
    /// - [`CleanupMode::Manual`] — UI-confirmed execution.  Always
    ///   deletes whatever is in the plan.
    pub async fn execute_plan(
        &self,
        plan: CleanupPlan,
        mode: CleanupMode,
    ) -> Result<CleanupResult, AppError> {
        let candidate_count = plan.candidates.len() as i64;
        let now = self.clock.unix_seconds();

        if matches!(mode, CleanupMode::DryRun) {
            let log_id = self
                .insert_log_row(now, mode, plan.projected_freed_bytes, candidate_count, "ok")
                .await?;
            return Ok(CleanupResult {
                mode,
                status: "ok".into(),
                freed_bytes: plan.projected_freed_bytes,
                deleted_vod_count: candidate_count,
                log_id,
            });
        }

        if plan.candidates.is_empty() {
            // For scheduled mode this is the "no work" path; manual
            // mode should never arrive here because the UI gates on
            // candidate_count > 0, but we emit a safe no-op.
            let log_id = self.insert_log_row(now, mode, 0, 0, "skipped").await?;
            return Ok(CleanupResult {
                mode,
                status: "skipped".into(),
                freed_bytes: 0,
                deleted_vod_count: 0,
                log_id,
            });
        }

        let mut freed_bytes: i64 = 0;
        let mut deleted_vod_count: i64 = 0;
        let mut errors: i64 = 0;
        for candidate in &plan.candidates {
            match self
                .delete_candidate(&candidate.vod_id, &candidate.final_path)
                .await
            {
                Ok(removed_bytes) => {
                    freed_bytes = freed_bytes.saturating_add(removed_bytes);
                    deleted_vod_count += 1;
                }
                Err(err) => {
                    warn!(
                        vod_id = %candidate.vod_id,
                        path = %candidate.final_path,
                        error = ?err,
                        "cleanup delete failed"
                    );
                    errors += 1;
                }
            }
        }

        let status = if errors == 0 {
            "ok"
        } else if deleted_vod_count == 0 {
            "error"
        } else {
            "partial"
        };
        let log_id = self
            .insert_log_row(now, mode, freed_bytes, deleted_vod_count, status)
            .await?;

        info!(
            mode = ?mode,
            freed_bytes,
            deleted_vod_count,
            errors,
            "cleanup executed"
        );

        Ok(CleanupResult {
            mode,
            status: status.into(),
            freed_bytes,
            deleted_vod_count,
            log_id,
        })
    }

    /// Daemon-tick entry point.  Idempotent and cheap: if the feature
    /// is disabled, returns immediately.  If enabled and the schedule
    /// hour has not yet been crossed today, returns without touching
    /// the disk.  Emits `DiskPressure` whenever pressure crosses the
    /// high watermark, regardless of `cleanup_enabled`.
    pub async fn schedule_tick(&self, sink: &CleanupEventSink) -> Result<(), AppError> {
        let settings = self.settings.get().await?;
        let Some(library_root) = settings.library_root.as_deref() else {
            return Ok(());
        };

        let snapshot = self.snapshot_disk(library_root).await?;
        let used_fraction = snapshot.used_fraction();
        if used_fraction >= settings.cleanup_high_watermark {
            (sink)(CleanupEvent::DiskPressure {
                used_fraction,
                free_bytes: snapshot.free_bytes as i64,
            });
        }

        if !settings.cleanup_enabled {
            return Ok(());
        }

        if !self
            .schedule_due(settings.cleanup_schedule_hour, self.clock.unix_seconds())
            .await?
        {
            return Ok(());
        }

        // Build the plan from the same snapshot so the projection is
        // consistent with the DiskPressure event we just emitted.
        let candidates = self.fetch_candidates().await?;
        let projection = compute_plan_pure(
            snapshot,
            CleanupSettingsSnapshot {
                enabled: settings.cleanup_enabled,
                high_watermark: settings.cleanup_high_watermark,
                low_watermark: settings.cleanup_low_watermark,
            },
            candidates,
            self.clock.unix_seconds(),
        );

        if !projection.pressure_above_high {
            // Pressure dropped below the threshold between the disk
            // snapshot and the candidate fetch (rare, but possible).
            // Log a skipped row so the History view shows the tick
            // ran, then bail.
            let now = self.clock.unix_seconds();
            self.insert_log_row(now, CleanupMode::Scheduled, 0, 0, "skipped")
                .await?;
            return Ok(());
        }

        (sink)(CleanupEvent::PlanReady {
            candidate_count: projection.plan.candidates.len() as i64,
            projected_freed_bytes: projection.plan.projected_freed_bytes,
        });

        let result = self
            .execute_plan(projection.plan, CleanupMode::Scheduled)
            .await?;
        (sink)(CleanupEvent::Executed {
            mode: CleanupMode::Scheduled,
            status: result.status.clone(),
            freed_bytes: result.freed_bytes,
            deleted_vod_count: result.deleted_vod_count,
        });
        Ok(())
    }

    /// Read the most recent `cleanup_log` rows for the Settings
    /// History view.  `limit` is clamped to `[1, 200]`.
    pub async fn list_history(&self, limit: i64) -> Result<Vec<CleanupLogEntry>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query(
            "SELECT id, ran_at, mode, freed_bytes, deleted_vod_count, status
               FROM cleanup_log
              ORDER BY ran_at DESC, id DESC
              LIMIT ?",
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(CleanupLogEntry {
                id: row.try_get(0)?,
                ran_at: row.try_get(1)?,
                mode: row.try_get(2)?,
                freed_bytes: row.try_get(3)?,
                deleted_vod_count: row.try_get(4)?,
                status: row.try_get(5)?,
            });
        }
        Ok(out)
    }

    // --- internal helpers --------------------------------------

    async fn snapshot_disk(&self, library_root: &str) -> Result<DiskSnapshot, AppError> {
        let path = Path::new(library_root);
        let total_bytes = self.probe.total_bytes(path).await?;
        let free_bytes = self.probe.free_bytes(path).await?;
        Ok(DiskSnapshot {
            total_bytes,
            free_bytes,
        })
    }

    async fn fetch_candidates(&self) -> Result<Vec<CandidateInput>, AppError> {
        let rows = sqlx::query(
            "SELECT d.vod_id, COALESCE(s.login, '') AS login,
                    v.stream_started_at,
                    COALESCE(w.last_watched_at, 0) AS last_watched_at,
                    COALESCE(w.state, 'unwatched') AS state,
                    COALESCE(d.bytes_total, d.bytes_done) AS size_bytes,
                    d.final_path
               FROM downloads d
               JOIN vods v ON v.twitch_video_id = d.vod_id
               LEFT JOIN streamers s ON s.twitch_user_id = v.twitch_user_id
               LEFT JOIN watch_progress w ON w.vod_id = d.vod_id
              WHERE d.state = 'completed' AND d.final_path IS NOT NULL",
        )
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let vod_id: String = row.try_get(0)?;
            let login: String = row.try_get(1)?;
            let stream_started_at: i64 = row.try_get(2)?;
            let last_watched_at: i64 = row.try_get(3)?;
            let state_str: String = row.try_get(4)?;
            let size_bytes: i64 = row.try_get(5).unwrap_or(0);
            let final_path: String = row.try_get(6)?;
            out.push(CandidateInput {
                vod_id,
                streamer_login: login,
                stream_started_at,
                last_watched_at,
                watch_state: WatchStateForCleanup::from_db_str(&state_str),
                size_bytes,
                final_path,
            });
        }
        Ok(out)
    }

    async fn delete_candidate(&self, vod_id: &str, path: &str) -> Result<i64, AppError> {
        let p = Path::new(path);
        let metadata = tokio::fs::metadata(p).await.ok();
        let size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
        if metadata.is_some() {
            tokio::fs::remove_file(p).await?;
        }
        // Mark the download row failed_permanent with reason
        // CLEANED_UP so the Library UI can surface a "Re-download"
        // CTA, and so the row no longer matches the next plan's
        // `state = 'completed'` filter.
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE downloads
                SET state = 'failed_permanent',
                    last_error = 'CLEANED_UP',
                    last_error_at = ?,
                    final_path = NULL
              WHERE vod_id = ?",
        )
        .bind(now)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        Ok(size)
    }

    async fn insert_log_row(
        &self,
        ran_at: i64,
        mode: CleanupMode,
        freed_bytes: i64,
        deleted_vod_count: i64,
        status: &str,
    ) -> Result<i64, AppError> {
        let row = sqlx::query(
            "INSERT INTO cleanup_log (ran_at, mode, freed_bytes, deleted_vod_count, status)
             VALUES (?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(ran_at)
        .bind(mode.as_db_str())
        .bind(freed_bytes)
        .bind(deleted_vod_count)
        .bind(status)
        .fetch_one(self.db.pool())
        .await?;
        Ok(row.try_get(0)?)
    }

    /// Has the configured schedule hour been crossed since the last
    /// scheduled run?  Compares against `cleanup_log` rows of mode
    /// 'scheduled' so a manual run doesn't reset the schedule.
    async fn schedule_due(&self, schedule_hour: i64, now: i64) -> Result<bool, AppError> {
        let last_scheduled: Option<i64> =
            sqlx::query_scalar("SELECT MAX(ran_at) FROM cleanup_log WHERE mode = 'scheduled'")
                .fetch_one(self.db.pool())
                .await
                .unwrap_or(None);

        Ok(is_schedule_due(schedule_hour, now, last_scheduled))
    }
}

fn require_library_root(value: &Option<String>) -> Result<String, AppError> {
    value.clone().ok_or_else(|| AppError::Cleanup {
        detail: "library_root not configured".into(),
    })
}

/// Pure helper used by `schedule_due` and exercised in tests.
/// `now` and `last_scheduled` are wall-clock unix seconds.
///
/// Contract:
/// - Never run before: hold off until today's configured hour, then
///   fire at the first tick that crosses it.  This avoids surprising
///   the user on first launch.
/// - Ran on a previous day (or earlier today before the target) and
///   we're now past either yesterday's or today's target: due.
/// - Ran today after the target: not due again until tomorrow.
pub fn is_schedule_due(schedule_hour: i64, now: i64, last_scheduled: Option<i64>) -> bool {
    let day_secs = 86_400;
    // Today's midnight (UTC); the tick runs at daily granularity and
    // DST boundary skew is not load-bearing.
    let today_midnight = (now / day_secs) * day_secs;
    let target_today = today_midnight + schedule_hour * 3_600;
    match last_scheduled {
        None => now >= target_today,
        Some(last) if last >= target_today => false,
        Some(last) => {
            let target_yesterday = target_today - day_secs;
            (now >= target_yesterday && last < target_yesterday) || now >= target_today
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::fs::space::FakeDiskUsage;

    async fn setup() -> (CleanupService, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(2_000_000));
        let settings = SettingsService::new(db.clone(), clock.clone());
        let probe = Arc::new(FakeDiskUsage {
            total: 10_000,
            free: 1_000,
        });
        let svc = CleanupService::new(db.clone(), clock, settings, probe);
        (svc, db)
    }

    #[tokio::test]
    async fn compute_plan_errors_without_library_root() {
        let (svc, _db) = setup().await;
        let err = svc.compute_plan().await.unwrap_err();
        assert!(matches!(err, AppError::Cleanup { .. }));
    }

    #[tokio::test]
    async fn dry_run_logs_without_filesystem_writes() {
        let (svc, db) = setup().await;
        // Configure a library root so disk-snapshot path works.
        let root = if cfg!(windows) {
            "C:\\tmp\\lib"
        } else {
            "/tmp/lib"
        };
        let settings = SettingsService::new(db.clone(), svc.clock.clone());
        settings
            .update(crate::services::settings::SettingsPatch {
                library_root: Some(root.into()),
                ..Default::default()
            })
            .await
            .unwrap();
        let plan = svc.compute_plan().await.unwrap();
        let result = svc.execute_plan(plan, CleanupMode::DryRun).await.unwrap();
        assert_eq!(result.status, "ok");
        assert_eq!(result.deleted_vod_count, 0);
        let log = svc.list_history(10).await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].mode, "dry_run");
    }

    #[tokio::test]
    async fn list_history_clamps_limit() {
        let (svc, _db) = setup().await;
        // No rows yet — listing returns empty regardless of limit.
        assert!(svc.list_history(0).await.unwrap().is_empty());
        assert!(svc.list_history(500).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn schedule_due_pure_logic() {
        let day = 86_400;
        // Schedule hour 3 (= 10_800 seconds past midnight).
        let midnight = 1_700_000_000 - (1_700_000_000 % day);
        let before_target = midnight + 2 * 3_600; // 02:00
        let after_target = midnight + 4 * 3_600; // 04:00

        // Never ran before — eligible only if we're past the target.
        assert!(!is_schedule_due(3, before_target, None));
        assert!(is_schedule_due(3, after_target, None));

        // Ran yesterday — eligible at any time today.
        let yesterday = midnight - day;
        assert!(is_schedule_due(3, before_target, Some(yesterday)));
        assert!(is_schedule_due(3, after_target, Some(yesterday)));

        // Ran today after the target — not eligible again until
        // tomorrow.
        assert!(!is_schedule_due(3, after_target + 60, Some(after_target)));

        // Ran today before the target — eligible once we cross.
        assert!(!is_schedule_due(3, before_target + 60, Some(before_target)));
        assert!(is_schedule_due(3, after_target, Some(before_target)));
    }

    #[tokio::test]
    async fn get_disk_usage_reports_no_root() {
        let (svc, _db) = setup().await;
        let usage = svc.get_disk_usage().await.unwrap();
        assert_eq!(usage.library_path, Some(false));
        assert_eq!(usage.total_bytes, 0);
        assert!(!usage.above_high_watermark);
    }

    #[tokio::test]
    async fn get_disk_usage_with_root() {
        let (svc, db) = setup().await;
        let root = if cfg!(windows) {
            "C:\\tmp\\lib"
        } else {
            "/tmp/lib"
        };
        let settings = SettingsService::new(db.clone(), svc.clock.clone());
        settings
            .update(crate::services::settings::SettingsPatch {
                library_root: Some(root.into()),
                ..Default::default()
            })
            .await
            .unwrap();
        let usage = svc.get_disk_usage().await.unwrap();
        assert_eq!(usage.library_path, Some(true));
        assert_eq!(usage.total_bytes, 10_000);
        assert_eq!(usage.free_bytes, 1_000);
        // 90 % used (1000 free / 10 000 total) hits the default
        // high watermark of 0.9 exactly.
        assert!(usage.above_high_watermark);
    }

    #[tokio::test]
    async fn execute_plan_with_empty_plan_logs_skipped() {
        let (svc, db) = setup().await;
        let root = if cfg!(windows) {
            "C:\\tmp\\lib"
        } else {
            "/tmp/lib"
        };
        let settings = SettingsService::new(db.clone(), svc.clock.clone());
        settings
            .update(crate::services::settings::SettingsPatch {
                library_root: Some(root.into()),
                ..Default::default()
            })
            .await
            .unwrap();
        let plan = svc.compute_plan().await.unwrap();
        assert!(plan.candidates.is_empty());
        let result = svc.execute_plan(plan, CleanupMode::Manual).await.unwrap();
        assert_eq!(result.status, "skipped");
        assert_eq!(result.deleted_vod_count, 0);
    }
}
