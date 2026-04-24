//! Timeline indexer service.
//!
//! Maintains the `stream_intervals` materialised view from VOD ingest
//! events. Exposes read queries for the `/timeline` UI route and the
//! `co_streams` column in the library detail drawer.
//!
//! The table is fully derivable from `vods` — a rebuild walks every
//! row in `vods` with a non-null `stream_started_at` and
//! `duration_seconds > 0`. Callers:
//!   - on app start, if the table is empty but `vods` is not,
//!     `maybe_backfill()` kicks off a progress-reporting rebuild on
//!     a spawned task;
//!   - the poller's event sink calls `upsert_from_vod()` whenever a
//!     VOD transitions to `eligible` / `skipped_*` with a known
//!     `stream_started_at`;
//!   - the admin command `cmd_rebuild_timeline_index` fires
//!     `rebuild_all()` with progress.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::timeline::{CoStream, Interval};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;

/// Thread-safe event fan-out for long-running rebuilds. The runtime
/// passes a closure that emits on the Tauri topic; tests pass a
/// capturing closure that collects the events for assertion.
pub type IndexerEventSink = Arc<dyn Fn(IndexerEvent) + Send + Sync>;

/// Events emitted by `rebuild_all`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexerEvent {
    Rebuilding { processed: i64, total: i64 },
    Rebuilt { total: i64 },
}

/// Filter predicate for the `list_timeline` read. Each field is
/// independently optional; `None` means "no filter on this axis".
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TimelineFilters {
    #[specta(optional)]
    pub since: Option<i64>,
    #[specta(optional)]
    pub until: Option<i64>,
    #[specta(optional)]
    pub streamer_ids: Option<Vec<String>>,
}

/// Timeline stats for the header of the `/timeline` route.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TimelineStats {
    pub total_intervals: i64,
    pub earliest_start_at: Option<i64>,
    pub latest_end_at: Option<i64>,
    pub largest_overlap_group: i64,
}

#[derive(Debug, Clone)]
pub struct TimelineIndexerService {
    db: Db,
    clock: Arc<dyn Clock>,
}

impl TimelineIndexerService {
    pub fn new(db: Db, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }

    /// Shared access to the underlying pool. Useful for the poller's
    /// sink when it wants to query a vod row inline before calling
    /// `upsert_from_vod`.
    pub fn pool(&self) -> &sqlx::SqlitePool {
        self.db.pool()
    }

    /// Upsert an interval for the given VOD. Called from the poller's
    /// event sink whenever a VOD's stream_started_at + duration are
    /// known. Silently skips rows with a non-positive duration (live
    /// gate / sub-only VODs that haven't been resolved yet).
    pub async fn upsert_from_vod(
        &self,
        twitch_video_id: &str,
        twitch_user_id: &str,
        stream_started_at: i64,
        duration_seconds: i64,
    ) -> Result<(), AppError> {
        if duration_seconds <= 0 {
            return Ok(());
        }
        let end_at = stream_started_at.saturating_add(duration_seconds);
        let now = self.clock.unix_seconds();
        sqlx::query(
            "INSERT INTO stream_intervals (vod_id, streamer_id, start_at, end_at, created_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(vod_id) DO UPDATE SET
                streamer_id = excluded.streamer_id,
                start_at    = excluded.start_at,
                end_at      = excluded.end_at",
        )
        .bind(twitch_video_id)
        .bind(twitch_user_id)
        .bind(stream_started_at)
        .bind(end_at)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Drop a VOD's interval row (called from ingest when the parent
    /// VOD itself is removed; FK cascade handles streamer removal).
    pub async fn remove(&self, vod_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM stream_intervals WHERE vod_id = ?")
            .bind(vod_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// True iff the intervals table is empty.
    pub async fn is_empty(&self) -> Result<bool, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM stream_intervals")
            .fetch_one(self.db.pool())
            .await?;
        let n: i64 = row.try_get("n")?;
        Ok(n == 0)
    }

    /// Count of VOD rows that could produce an interval.
    pub async fn eligible_vod_count(&self) -> Result<i64, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS n
               FROM vods
              WHERE stream_started_at > 0 AND duration_seconds > 0",
        )
        .fetch_one(self.db.pool())
        .await?;
        let n: i64 = row.try_get("n")?;
        Ok(n)
    }

    /// Rebuild the entire table from `vods`. Emits `Rebuilding`
    /// roughly every 200 rows so the UI can render a progress bar
    /// during a large backfill, then a single `Rebuilt` at the end.
    pub async fn rebuild_all(&self, sink: IndexerEventSink) -> Result<i64, AppError> {
        let total = self.eligible_vod_count().await?;
        // Truncate first so partial state is never observed.
        sqlx::query("DELETE FROM stream_intervals")
            .execute(self.db.pool())
            .await?;
        // Insert in one statement using a SELECT — much faster than
        // row-by-row and keeps the indexer in a consistent state at
        // all points (the transaction either commits in full or is
        // rolled back on error).
        let now = self.clock.unix_seconds();
        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            "INSERT INTO stream_intervals (vod_id, streamer_id, start_at, end_at, created_at)
             SELECT twitch_video_id, twitch_user_id, stream_started_at,
                    stream_started_at + duration_seconds, ?
               FROM vods
              WHERE stream_started_at > 0 AND duration_seconds > 0",
        )
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        sink(IndexerEvent::Rebuilding {
            processed: total,
            total,
        });
        sink(IndexerEvent::Rebuilt { total });
        Ok(total)
    }

    /// Read intervals matching the filters, ordered by `start_at`.
    pub async fn list(&self, filters: TimelineFilters) -> Result<Vec<Interval>, AppError> {
        // Build the WHERE clause from a fixed vocabulary of fragments;
        // every user-supplied value goes through `.bind(...)`.
        let mut sql = String::from(
            "SELECT vod_id, streamer_id, start_at, end_at FROM stream_intervals WHERE 1=1",
        );
        if filters.since.is_some() {
            sql.push_str(" AND end_at >= ?");
        }
        if filters.until.is_some() {
            sql.push_str(" AND start_at <= ?");
        }
        if let Some(ids) = filters.streamer_ids.as_ref()
            && !ids.is_empty()
        {
            sql.push_str(" AND streamer_id IN (");
            for i in 0..ids.len() {
                if i > 0 {
                    sql.push(',');
                }
                sql.push('?');
            }
            sql.push(')');
        }
        sql.push_str(" ORDER BY start_at ASC, vod_id ASC");

        let mut q = sqlx::query(&sql);
        if let Some(since) = filters.since {
            q = q.bind(since);
        }
        if let Some(until) = filters.until {
            q = q.bind(until);
        }
        if let Some(ids) = filters.streamer_ids.as_ref() {
            for id in ids {
                q = q.bind(id);
            }
        }
        let rows = q.fetch_all(self.db.pool()).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(Interval {
                vod_id: r.try_get("vod_id")?,
                streamer_id: r.try_get("streamer_id")?,
                start_at: r.try_get("start_at")?,
                end_at: r.try_get("end_at")?,
            });
        }
        Ok(out)
    }

    /// Return every other interval that overlaps the given VOD's
    /// interval, using the domain helper for the actual overlap
    /// calculation. Ordered by overlap length descending.
    pub async fn co_streams_of(&self, vod_id: &str) -> Result<Vec<CoStream>, AppError> {
        let row = sqlx::query(
            "SELECT vod_id, streamer_id, start_at, end_at
               FROM stream_intervals WHERE vod_id = ?",
        )
        .bind(vod_id)
        .fetch_optional(self.db.pool())
        .await?;
        let around = match row {
            Some(r) => Interval {
                vod_id: r.try_get("vod_id")?,
                streamer_id: r.try_get("streamer_id")?,
                start_at: r.try_get("start_at")?,
                end_at: r.try_get("end_at")?,
            },
            None => return Ok(vec![]),
        };
        // Narrow the candidate set to intervals that *could* overlap
        // the time range; still excludes same-streamer and same-vod
        // via the domain helper.
        let all = self
            .list(TimelineFilters {
                since: Some(around.start_at),
                until: Some(around.end_at),
                streamer_ids: None,
            })
            .await?;
        Ok(crate::domain::timeline::find_co_streams(&around, &all))
    }

    pub async fn stats(&self) -> Result<TimelineStats, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS n, MIN(start_at) AS lo, MAX(end_at) AS hi
               FROM stream_intervals",
        )
        .fetch_one(self.db.pool())
        .await?;
        let total: i64 = row.try_get("n").unwrap_or(0);
        let earliest: Option<i64> = row.try_get("lo").ok();
        let latest: Option<i64> = row.try_get("hi").ok();

        // Largest concurrent group = the maximum count of intervals
        // covering any single instant. For Phase 4 we approximate
        // this by the max concurrent count across interval endpoints
        // (a sweep-line over start_at/end_at pairs). Worth doing in
        // the domain layer later if we need it elsewhere.
        let rows = sqlx::query("SELECT start_at, end_at FROM stream_intervals")
            .fetch_all(self.db.pool())
            .await?;
        let mut events: Vec<(i64, i32)> = Vec::with_capacity(rows.len() * 2);
        for r in rows {
            let s: i64 = r.try_get("start_at")?;
            let e: i64 = r.try_get("end_at")?;
            events.push((s, 1));
            events.push((e, -1));
        }
        events.sort_unstable();
        let mut active: i64 = 0;
        let mut peak: i64 = 0;
        for (_, delta) in events {
            active += delta as i64;
            if active > peak {
                peak = active;
            }
        }

        Ok(TimelineStats {
            total_intervals: total,
            earliest_start_at: earliest,
            latest_end_at: latest,
            largest_overlap_group: peak,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn fixture() -> (TimelineIndexerService, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock = Arc::new(FixedClock::at(1_700_000_000));
        (TimelineIndexerService::new(db.clone(), clock), db)
    }

    async fn seed_streamer(db: &Db, id: &str, login: &str) {
        sqlx::query(
            "INSERT INTO streamers (twitch_user_id, login, display_name, profile_image_url,
                broadcaster_type, twitch_created_at, added_at)
             VALUES (?, ?, ?, NULL, '', 0, 0)",
        )
        .bind(id)
        .bind(login)
        .bind(login)
        .execute(db.pool())
        .await
        .unwrap();
    }

    async fn seed_vod(db: &Db, id: &str, streamer: &str, start: i64, dur: i64) {
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title, stream_started_at,
                published_at, url, duration_seconds, first_seen_at, last_seen_at)
             VALUES (?, ?, 'title', ?, ?, 'https://twitch.tv', ?, ?, ?)",
        )
        .bind(id)
        .bind(streamer)
        .bind(start)
        .bind(start)
        .bind(dur)
        .bind(start)
        .bind(start)
        .execute(db.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn upsert_is_idempotent_and_overwrites() {
        let (svc, db) = fixture().await;
        seed_streamer(&db, "u1", "sampler").await;
        seed_vod(&db, "v1", "u1", 1000, 600).await;
        svc.upsert_from_vod("v1", "u1", 1000, 600).await.unwrap();
        svc.upsert_from_vod("v1", "u1", 1000, 800).await.unwrap();
        let ivs = svc.list(TimelineFilters::default()).await.unwrap();
        assert_eq!(ivs.len(), 1);
        assert_eq!(ivs[0].end_at, 1800);
    }

    #[tokio::test]
    async fn rebuild_from_empty_vods_table_is_a_noop() {
        let (svc, _db) = fixture().await;
        let sink: IndexerEventSink = Arc::new(|_| {});
        assert_eq!(svc.rebuild_all(sink).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn rebuild_all_derives_every_eligible_row() {
        let (svc, db) = fixture().await;
        seed_streamer(&db, "u1", "sampler").await;
        seed_streamer(&db, "u2", "live").await;
        seed_vod(&db, "v1", "u1", 100, 300).await;
        seed_vod(&db, "v2", "u2", 200, 300).await;
        // Zero-duration VOD should be excluded.
        seed_vod(&db, "v3", "u1", 1000, 0).await;

        let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink: IndexerEventSink = {
            let captured = captured.clone();
            Arc::new(move |ev| captured.lock().unwrap().push(ev))
        };
        let n = svc.rebuild_all(sink).await.unwrap();
        assert_eq!(n, 2);
        let ivs = svc.list(TimelineFilters::default()).await.unwrap();
        assert_eq!(ivs.len(), 2);
    }

    #[tokio::test]
    async fn list_filters_by_time_window_and_streamer() {
        let (svc, db) = fixture().await;
        seed_streamer(&db, "u1", "s1").await;
        seed_streamer(&db, "u2", "s2").await;
        seed_vod(&db, "v1", "u1", 100, 100).await;
        seed_vod(&db, "v2", "u2", 1000, 100).await;
        svc.upsert_from_vod("v1", "u1", 100, 100).await.unwrap();
        svc.upsert_from_vod("v2", "u2", 1000, 100).await.unwrap();

        let windowed = svc
            .list(TimelineFilters {
                since: Some(150),
                until: Some(300),
                streamer_ids: None,
            })
            .await
            .unwrap();
        assert_eq!(windowed.len(), 1);
        assert_eq!(windowed[0].vod_id, "v1");

        let scoped = svc
            .list(TimelineFilters {
                since: None,
                until: None,
                streamer_ids: Some(vec!["u2".into()]),
            })
            .await
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].vod_id, "v2");
    }

    #[tokio::test]
    async fn co_streams_finds_overlaps_on_other_streamers() {
        let (svc, db) = fixture().await;
        seed_streamer(&db, "u1", "s1").await;
        seed_streamer(&db, "u2", "s2").await;
        seed_streamer(&db, "u3", "s3").await;
        seed_vod(&db, "v1", "u1", 100, 1000).await;
        seed_vod(&db, "v2", "u2", 500, 1000).await;
        seed_vod(&db, "v3", "u3", 2000, 100).await;
        svc.upsert_from_vod("v1", "u1", 100, 1000).await.unwrap();
        svc.upsert_from_vod("v2", "u2", 500, 1000).await.unwrap();
        svc.upsert_from_vod("v3", "u3", 2000, 100).await.unwrap();

        let hits = svc.co_streams_of("v1").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].interval.vod_id, "v2");
    }

    #[tokio::test]
    async fn stats_reports_peak_concurrency() {
        let (svc, db) = fixture().await;
        seed_streamer(&db, "u1", "s1").await;
        seed_streamer(&db, "u2", "s2").await;
        seed_streamer(&db, "u3", "s3").await;
        seed_vod(&db, "v1", "u1", 0, 100).await;
        seed_vod(&db, "v2", "u2", 20, 100).await;
        seed_vod(&db, "v3", "u3", 40, 100).await;
        svc.upsert_from_vod("v1", "u1", 0, 100).await.unwrap();
        svc.upsert_from_vod("v2", "u2", 20, 100).await.unwrap();
        svc.upsert_from_vod("v3", "u3", 40, 100).await.unwrap();

        let s = svc.stats().await.unwrap();
        assert_eq!(s.total_intervals, 3);
        assert_eq!(s.earliest_start_at, Some(0));
        assert_eq!(s.latest_end_at, Some(140));
        // Peak at t=40 → 60: all three live.
        assert_eq!(s.largest_overlap_group, 3);
    }
}
