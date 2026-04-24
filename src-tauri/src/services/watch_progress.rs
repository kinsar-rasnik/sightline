//! Watch-progress service (Phase 5 / ADR-0018).
//!
//! * Persists position / state / stats to `watch_progress` (0008).
//! * Emits `watch:progress_updated`, `watch:state_changed`,
//!   `watch:completed` through an event sink (same pattern as the
//!   poller/downloads service sinks).
//! * Rounds writes to 0.5 s resolution to cap write amplification
//!   at the player's ~4 Hz `timeupdate` rate.
//!
//! The service is intentionally small — the domain layer owns the
//! state machine, interval merger, and pre-roll math. We just wire
//! those into SQLite and broadcast the transitions.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tracing::debug;

use crate::domain::watch_progress::{
    ProgressSettings, UpdateContext, WatchState, on_mark_unwatched, on_mark_watched,
    round_to_half_second, transition_on_update, watched_fraction,
};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;

/// Public snapshot of a VOD's watch progress — serialised to the
/// frontend via IPC and included in the Continue Watching row.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchProgressRow {
    pub vod_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub watched_fraction: f64,
    pub state: WatchState,
    pub first_watched_at: Option<i64>,
    pub last_watched_at: i64,
    pub last_session_duration_seconds: f64,
    pub total_watch_seconds: f64,
}

/// Aggregate stats used by the later "hours watched this streamer"
/// summaries. Defined now so the command + binding types are stable.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchStats {
    pub total_watch_seconds: f64,
    pub completed_count: i64,
    pub in_progress_count: i64,
}

/// Items surfaced in the Continue Watching row. `remaining_seconds`
/// is a derived convenience so the frontend doesn't have to recompute
/// it from the stored position.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWatchingEntry {
    pub vod_id: String,
    pub title: String,
    pub streamer_display_name: String,
    pub streamer_login: String,
    pub duration_seconds: f64,
    pub position_seconds: f64,
    pub remaining_seconds: f64,
    pub watched_fraction: f64,
    pub thumbnail_url: Option<String>,
    pub last_watched_at: i64,
}

/// Events emitted by the service. The commands layer fans these onto
/// the Tauri event bus (`watch:progress_updated`, `watch:state_changed`,
/// `watch:completed`).
#[derive(Debug, Clone)]
pub enum WatchEvent {
    Updated {
        vod_id: String,
        position_seconds: f64,
        state: WatchState,
    },
    StateChanged {
        vod_id: String,
        from: WatchState,
        to: WatchState,
    },
    Completed {
        vod_id: String,
    },
}

pub type WatchEventSink = Arc<dyn Fn(WatchEvent) + Send + Sync>;

#[derive(Debug)]
pub struct WatchProgressService {
    db: Db,
    clock: Arc<dyn Clock>,
}

impl WatchProgressService {
    pub fn new(db: Db, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }

    /// Read the stored row for a VOD. Returns `None` when the user
    /// has never opened the player for this VOD.
    pub async fn get(&self, vod_id: &str) -> Result<Option<WatchProgressRow>, AppError> {
        let row = sqlx::query(
            "SELECT vod_id, position_seconds, duration_seconds, watched_fraction,
                    state, first_watched_at, last_watched_at,
                    last_session_duration_seconds, total_watch_seconds
             FROM watch_progress WHERE vod_id = ?",
        )
        .bind(vod_id)
        .fetch_optional(self.db.pool())
        .await?;
        row.as_ref().map(row_to_watch_progress).transpose()
    }

    /// Persist a fresh `timeupdate`. The settings input controls the
    /// completion threshold; in production it's derived from the
    /// live `AppSettings`, in tests we inject a fixed value.
    pub async fn update(
        &self,
        vod_id: &str,
        position_seconds: f64,
        duration_seconds: f64,
        settings: ProgressSettings,
        sink: Option<&WatchEventSink>,
    ) -> Result<WatchProgressRow, AppError> {
        let rounded = round_to_half_second(position_seconds.max(0.0));
        let duration = duration_seconds.max(0.0);
        let now = self.clock.unix_seconds();
        let previous = self.get(vod_id).await?;
        let current_state = previous
            .as_ref()
            .map(|p| p.state)
            .unwrap_or(WatchState::Unwatched);
        let new_state = transition_on_update(UpdateContext {
            current: current_state,
            position_seconds: rounded,
            duration_seconds: duration,
            settings,
        });

        let first_watched_at = previous
            .as_ref()
            .and_then(|p| p.first_watched_at)
            .unwrap_or(now);
        // Retain the prior total_watch_seconds — the service layer's
        // caller (the player session manager) owns the interval merge
        // and tells us the cumulative value via a dedicated update.
        let prior_total = previous
            .as_ref()
            .map(|p| p.total_watch_seconds)
            .unwrap_or(0.0);

        sqlx::query(
            "INSERT INTO watch_progress (
                vod_id, position_seconds, duration_seconds, state,
                first_watched_at, last_watched_at,
                last_session_duration_seconds, total_watch_seconds
             )
             VALUES (?, ?, ?, ?, ?, ?, 0, ?)
             ON CONFLICT(vod_id) DO UPDATE SET
                position_seconds = excluded.position_seconds,
                duration_seconds = excluded.duration_seconds,
                state = excluded.state,
                last_watched_at = excluded.last_watched_at,
                first_watched_at = COALESCE(watch_progress.first_watched_at,
                                            excluded.first_watched_at)",
        )
        .bind(vod_id)
        .bind(rounded)
        .bind(duration)
        .bind(new_state.as_db_str())
        .bind(first_watched_at)
        .bind(now)
        .bind(prior_total)
        .execute(self.db.pool())
        .await?;

        let row = self.get(vod_id).await?.ok_or_else(|| AppError::Internal {
            detail: "watch_progress row disappeared after upsert".into(),
        })?;

        if let Some(sink) = sink {
            sink(WatchEvent::Updated {
                vod_id: vod_id.to_owned(),
                position_seconds: rounded,
                state: new_state,
            });
            if current_state != new_state {
                sink(WatchEvent::StateChanged {
                    vod_id: vod_id.to_owned(),
                    from: current_state,
                    to: new_state,
                });
                if new_state == WatchState::Completed {
                    sink(WatchEvent::Completed {
                        vod_id: vod_id.to_owned(),
                    });
                }
            }
        }
        Ok(row)
    }

    /// Mark a VOD as watched. Sets position to the duration and
    /// flips state to `ManuallyWatched`; emits `state_changed`.
    pub async fn mark_watched(
        &self,
        vod_id: &str,
        duration_seconds: f64,
        sink: Option<&WatchEventSink>,
    ) -> Result<WatchProgressRow, AppError> {
        let (state, pos) = on_mark_watched(duration_seconds);
        self.set_state(vod_id, state, pos, duration_seconds, sink)
            .await
    }

    /// Mark a VOD as unwatched. Resets position + state.
    pub async fn mark_unwatched(
        &self,
        vod_id: &str,
        sink: Option<&WatchEventSink>,
    ) -> Result<WatchProgressRow, AppError> {
        let (state, pos) = on_mark_unwatched();
        // We need the stored duration to keep the row consistent;
        // fall back to 0 if the user never opened the VOD.
        let duration = self
            .get(vod_id)
            .await?
            .map(|p| p.duration_seconds)
            .unwrap_or(0.0);
        self.set_state(vod_id, state, pos, duration, sink).await
    }

    /// Bump `total_watch_seconds` by the given delta. Called by the
    /// player session manager after merging the latest tick into its
    /// IntervalSet — the delta is `new_total - old_total` so we never
    /// decrement the stored value below what's already there.
    pub async fn add_watch_seconds(
        &self,
        vod_id: &str,
        delta_seconds: f64,
    ) -> Result<(), AppError> {
        if delta_seconds <= 0.0 {
            return Ok(());
        }
        sqlx::query(
            "UPDATE watch_progress
             SET total_watch_seconds = total_watch_seconds + ?,
                 last_session_duration_seconds = last_session_duration_seconds + ?
             WHERE vod_id = ?",
        )
        .bind(delta_seconds)
        .bind(delta_seconds)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Top-12-ish "Continue Watching" entries. Joins vods + streamers
    /// so the frontend doesn't need a second round-trip for thumbnail
    /// URLs or names. `limit` clamped to 1..=24.
    pub async fn list_continue_watching(
        &self,
        limit: i64,
    ) -> Result<Vec<ContinueWatchingEntry>, AppError> {
        let limit = limit.clamp(1, 24);
        let rows = sqlx::query(
            "SELECT w.vod_id, w.position_seconds, w.duration_seconds, w.watched_fraction,
                    w.last_watched_at,
                    v.title, v.thumbnail_url,
                    s.display_name, s.login
             FROM watch_progress w
             JOIN vods v      ON v.twitch_video_id = w.vod_id
             JOIN streamers s ON s.twitch_user_id  = v.twitch_user_id
             WHERE w.state = 'in_progress'
             ORDER BY w.last_watched_at DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        rows.into_iter()
            .map(|r| {
                let duration: f64 = r.try_get(2)?;
                let position: f64 = r.try_get(1)?;
                let remaining = (duration - position).max(0.0);
                Ok(ContinueWatchingEntry {
                    vod_id: r.try_get(0)?,
                    position_seconds: position,
                    duration_seconds: duration,
                    watched_fraction: r.try_get(3)?,
                    last_watched_at: r.try_get(4)?,
                    title: r.try_get(5)?,
                    thumbnail_url: r.try_get(6)?,
                    streamer_display_name: r.try_get(7)?,
                    streamer_login: r.try_get(8)?,
                    remaining_seconds: remaining,
                })
            })
            .collect()
    }

    /// Aggregate stats, optionally scoped to a streamer.
    pub async fn stats(&self, streamer_id: Option<&str>) -> Result<WatchStats, AppError> {
        let (total, completed_count, in_progress_count): (f64, i64, i64) =
            if let Some(id) = streamer_id {
                let r = sqlx::query(
                    "SELECT COALESCE(SUM(w.total_watch_seconds), 0),
                            COUNT(CASE WHEN w.state IN ('completed','manually_watched') THEN 1 END),
                            COUNT(CASE WHEN w.state = 'in_progress' THEN 1 END)
                     FROM watch_progress w
                     JOIN vods v ON v.twitch_video_id = w.vod_id
                     WHERE v.twitch_user_id = ?",
                )
                .bind(id)
                .fetch_one(self.db.pool())
                .await?;
                (r.try_get(0)?, r.try_get(1)?, r.try_get(2)?)
            } else {
                let r = sqlx::query(
                    "SELECT COALESCE(SUM(total_watch_seconds), 0),
                            COUNT(CASE WHEN state IN ('completed','manually_watched') THEN 1 END),
                            COUNT(CASE WHEN state = 'in_progress' THEN 1 END)
                     FROM watch_progress",
                )
                .fetch_one(self.db.pool())
                .await?;
                (r.try_get(0)?, r.try_get(1)?, r.try_get(2)?)
            };
        Ok(WatchStats {
            total_watch_seconds: total,
            completed_count,
            in_progress_count,
        })
    }

    async fn set_state(
        &self,
        vod_id: &str,
        state: WatchState,
        position_seconds: f64,
        duration_seconds: f64,
        sink: Option<&WatchEventSink>,
    ) -> Result<WatchProgressRow, AppError> {
        let previous = self.get(vod_id).await?;
        let current_state = previous
            .as_ref()
            .map(|p| p.state)
            .unwrap_or(WatchState::Unwatched);
        let now = self.clock.unix_seconds();
        let first_watched_at = previous.as_ref().and_then(|p| p.first_watched_at);
        let prior_total = previous
            .as_ref()
            .map(|p| p.total_watch_seconds)
            .unwrap_or(0.0);
        let rounded = round_to_half_second(position_seconds.max(0.0));

        sqlx::query(
            "INSERT INTO watch_progress (
                vod_id, position_seconds, duration_seconds, state,
                first_watched_at, last_watched_at,
                last_session_duration_seconds, total_watch_seconds
             ) VALUES (?, ?, ?, ?, ?, ?, 0, ?)
             ON CONFLICT(vod_id) DO UPDATE SET
                position_seconds = excluded.position_seconds,
                duration_seconds = excluded.duration_seconds,
                state = excluded.state,
                last_watched_at = excluded.last_watched_at",
        )
        .bind(vod_id)
        .bind(rounded)
        .bind(duration_seconds.max(0.0))
        .bind(state.as_db_str())
        .bind(first_watched_at.unwrap_or(now))
        .bind(now)
        .bind(prior_total)
        .execute(self.db.pool())
        .await?;

        let row = self.get(vod_id).await?.ok_or_else(|| AppError::Internal {
            detail: "watch_progress row disappeared after set_state".into(),
        })?;

        if let Some(sink) = sink
            && current_state != state
        {
            sink(WatchEvent::StateChanged {
                vod_id: vod_id.to_owned(),
                from: current_state,
                to: state,
            });
            if state == WatchState::Completed {
                sink(WatchEvent::Completed {
                    vod_id: vod_id.to_owned(),
                });
            }
        }
        debug!(vod_id = %vod_id, ?state, "watch progress state set");
        Ok(row)
    }
}

fn row_to_watch_progress(row: &sqlx::sqlite::SqliteRow) -> Result<WatchProgressRow, AppError> {
    let position: f64 = row.try_get(1)?;
    let duration: f64 = row.try_get(2)?;
    let fraction: f64 = row
        .try_get(3)
        .unwrap_or(watched_fraction(position, duration));
    let state_str: String = row.try_get(4)?;
    let state = WatchState::from_db_str(&state_str).ok_or_else(|| AppError::Internal {
        detail: format!("unknown watch_progress state: {state_str}"),
    })?;
    Ok(WatchProgressRow {
        vod_id: row.try_get(0)?,
        position_seconds: position,
        duration_seconds: duration,
        watched_fraction: fraction,
        state,
        first_watched_at: row.try_get(5).ok(),
        last_watched_at: row.try_get(6)?,
        last_session_duration_seconds: row.try_get(7)?,
        total_watch_seconds: row.try_get(8)?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn setup_service() -> (WatchProgressService, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        sqlx::query(
            "INSERT INTO streamers (twitch_user_id, login, display_name,
                 broadcaster_type, twitch_created_at, added_at)
             VALUES ('100', 'sampler', 'Sampler', '', 0, 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title,
                 stream_started_at, published_at, url, duration_seconds,
                 ingest_status, first_seen_at, last_seen_at)
             VALUES ('v1', '100', 'Watch Test', 1_000_000, 1_000_000,
                     'https://twitch.tv/videos/v1', 3600, 'eligible', 0, 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();
        (
            WatchProgressService::new(db.clone(), Arc::new(FixedClock::at(1_000_100))),
            db,
        )
    }

    #[tokio::test]
    async fn first_update_creates_in_progress_row() {
        let (svc, _db) = setup_service().await;
        let row = svc
            .update("v1", 100.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        assert_eq!(row.state, WatchState::InProgress);
        assert_eq!(row.position_seconds, 100.0);
        assert_eq!(row.first_watched_at, Some(1_000_100));
    }

    #[tokio::test]
    async fn crossing_threshold_emits_completed_event() {
        let (svc, _db) = setup_service().await;
        let fired = Arc::new(std::sync::Mutex::new(Vec::<WatchEvent>::new()));
        let cloned = fired.clone();
        let sink: WatchEventSink = Arc::new(move |ev| cloned.lock().unwrap().push(ev));

        svc.update(
            "v1",
            100.0,
            1000.0,
            ProgressSettings::default(),
            Some(&sink),
        )
        .await
        .unwrap();
        svc.update(
            "v1",
            950.0,
            1000.0,
            ProgressSettings::default(),
            Some(&sink),
        )
        .await
        .unwrap();
        let events = fired.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WatchEvent::Completed { .. })),
            "completed event missing from {events:?}"
        );
    }

    #[tokio::test]
    async fn mark_watched_sets_state_and_position() {
        let (svc, _db) = setup_service().await;
        svc.update("v1", 50.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        let row = svc.mark_watched("v1", 3600.0, None).await.unwrap();
        assert_eq!(row.state, WatchState::ManuallyWatched);
        assert_eq!(row.position_seconds, 3600.0);
    }

    #[tokio::test]
    async fn mark_unwatched_resets_row() {
        let (svc, _db) = setup_service().await;
        svc.update("v1", 3400.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        let row = svc.mark_unwatched("v1", None).await.unwrap();
        assert_eq!(row.state, WatchState::Unwatched);
        assert_eq!(row.position_seconds, 0.0);
    }

    #[tokio::test]
    async fn continue_watching_lists_in_progress_sorted() {
        let (svc, db) = setup_service().await;
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title,
                 stream_started_at, published_at, url, duration_seconds,
                 ingest_status, first_seen_at, last_seen_at)
             VALUES ('v2', '100', 'second', 1_000_010, 1_000_010,
                     'u', 1800, 'eligible', 0, 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();
        svc.update("v1", 10.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        // Force a newer last_watched_at for v2 so the sort is
        // deterministic under the FixedClock.
        sqlx::query("UPDATE watch_progress SET last_watched_at = 1_000_500 WHERE vod_id = 'v1'")
            .execute(db.pool())
            .await
            .unwrap();
        svc.update("v2", 5.0, 1800.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        sqlx::query("UPDATE watch_progress SET last_watched_at = 1_000_600 WHERE vod_id = 'v2'")
            .execute(db.pool())
            .await
            .unwrap();
        let entries = svc.list_continue_watching(10).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].vod_id, "v2");
        assert_eq!(entries[1].vod_id, "v1");
    }

    #[tokio::test]
    async fn add_watch_seconds_accumulates() {
        let (svc, _db) = setup_service().await;
        svc.update("v1", 10.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        svc.add_watch_seconds("v1", 5.0).await.unwrap();
        svc.add_watch_seconds("v1", 3.0).await.unwrap();
        svc.add_watch_seconds("v1", -1.0).await.unwrap(); // ignored
        let row = svc.get("v1").await.unwrap().unwrap();
        assert_eq!(row.total_watch_seconds, 8.0);
    }

    #[tokio::test]
    async fn stats_counts_completed_and_in_progress() {
        let (svc, _db) = setup_service().await;
        svc.update("v1", 10.0, 3600.0, ProgressSettings::default(), None)
            .await
            .unwrap();
        let stats = svc.stats(None).await.unwrap();
        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.in_progress_count, 1);
        svc.mark_watched("v1", 3600.0, None).await.unwrap();
        let stats = svc.stats(None).await.unwrap();
        assert_eq!(stats.completed_count, 1);
        assert_eq!(stats.in_progress_count, 0);
    }
}
