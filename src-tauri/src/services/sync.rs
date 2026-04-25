//! Multi-view sync engine service (Phase 6 / ADR-0021..0023).
//!
//! Owns the `sync_sessions` + `sync_session_panes` tables.  The
//! frontend opens a session, the service persists the *definition*,
//! and the live frame-by-frame state (pane currentTime, drift
//! corrections) lives in the renderer.  Transport commands fan to a
//! `SyncEventSink`; the lib.rs wiring turns those into Tauri events.
//!
//! Pure math (overlap windows, expected follower position) lives in
//! [`crate::domain::sync`]; this module is the I/O layer.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tracing::debug;

use crate::domain::sync::{
    DriftMeasurement, MemberRange, OverlapWindow, PaneIndex, SyncLayout, SyncMember, SyncSession,
    SyncSessionId, SyncStatus, SyncTransportCommand, compute_overlap,
};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::services::settings::{AppSettings, SettingsService};

/// Events published by the service.  The lib.rs sink fans these onto
/// the matching Tauri topics defined in `services::events`.  All five
/// topic names are listed in ADR-0021..0023.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    /// Session opened or status otherwise changed (e.g. closed).
    StateChanged {
        session_id: SyncSessionId,
        status: SyncStatus,
    },
    /// Frontend reported a drift > threshold; the loop corrected.
    DriftCorrected {
        session_id: SyncSessionId,
        pane_index: PaneIndex,
        drift_ms: f64,
        corrected_to_seconds: f64,
    },
    /// Leader pane was changed (open path or explicit promotion).
    LeaderChanged {
        session_id: SyncSessionId,
        leader_pane_index: PaneIndex,
    },
    /// A follower pane fell out of the leader's wall-clock range.
    MemberOutOfRange {
        session_id: SyncSessionId,
        pane_index: PaneIndex,
    },
    /// Session was closed (explicit close or page unmount).
    GroupClosed { session_id: SyncSessionId },
}

pub type SyncEventSink = Arc<dyn Fn(SyncEvent) + Send + Sync>;

#[derive(Debug)]
pub struct SyncService {
    db: Db,
    clock: Arc<dyn Clock>,
    settings: SettingsService,
}

/// Per-pane VOD wall-clock range as the service reads it from `vods`.
/// Used to compute overlap windows + out-of-range events without a
/// round-trip back to the frontend.
#[derive(Debug, Clone, Copy)]
struct VodRange {
    stream_started_at: i64,
    duration_seconds: i64,
}

/// Snapshot exposed via `cmd_get_overlap` and `cmd_open_sync_group`'s
/// pre-validation.  Wraps [`OverlapWindow`] with the input vod_ids
/// echoed back so the renderer can correlate without an extra query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlapResult {
    pub vod_ids: Vec<String>,
    pub window: OverlapWindow,
}

impl SyncService {
    pub fn new(db: Db, clock: Arc<dyn Clock>, settings: SettingsService) -> Self {
        Self {
            db,
            clock,
            settings,
        }
    }

    /// Open a session with the given pane → vod_id mapping and the
    /// chosen layout.  Returns the persisted session row with the
    /// default leader assigned per `sync_default_leader`.  The renderer
    /// uses the returned `SyncSession::id` for every subsequent IPC
    /// call.
    ///
    /// v1 enforces exactly two distinct VODs.  Reusing the same VOD
    /// twice is a v2 concern (PiP layouts that repeat the same source
    /// at two zoom levels) — rejected here so the unique partial
    /// index can't fail at write time.
    pub async fn open_session(
        &self,
        vod_ids: Vec<String>,
        layout: SyncLayout,
        sink: Option<&SyncEventSink>,
    ) -> Result<SyncSession, AppError> {
        if vod_ids.len() != 2 {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "multi-view v1 requires exactly 2 panes, got {}",
                    vod_ids.len()
                ),
            });
        }
        if vod_ids[0] == vod_ids[1] {
            return Err(AppError::InvalidInput {
                detail: "multi-view panes must reference distinct VODs".into(),
            });
        }
        // Validate every VOD exists before we touch sync_sessions —
        // the FK on sync_session_panes would otherwise produce a
        // generic `Db { detail: "FOREIGN KEY constraint failed" }`,
        // which is harder to surface in the UI.
        for id in &vod_ids {
            self.lookup_vod_range(id).await?;
        }
        let settings = self.settings.get().await?;
        let leader_index = default_leader_index(&settings);
        let now = self.clock.unix_seconds();

        let mut tx = self.db.pool().begin().await?;
        let row = sqlx::query(
            "INSERT INTO sync_sessions (created_at, layout, leader_pane_index, status)
             VALUES (?, ?, ?, 'active')
             RETURNING id",
        )
        .bind(now)
        .bind(layout.as_db_str())
        .bind(leader_index)
        .fetch_one(&mut *tx)
        .await?;
        let session_id: SyncSessionId = row.try_get("id")?;
        for (idx, vod_id) in vod_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO sync_session_panes
                    (session_id, pane_index, vod_id, volume, muted, joined_at)
                 VALUES (?, ?, ?, 1.0, 0, ?)",
            )
            .bind(session_id)
            .bind(idx as i64)
            .bind(vod_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let session = self.get_session(session_id).await?;
        if let Some(sink) = sink {
            sink(SyncEvent::StateChanged {
                session_id,
                status: SyncStatus::Active,
            });
            sink(SyncEvent::LeaderChanged {
                session_id,
                leader_pane_index: leader_index,
            });
        }
        Ok(session)
    }

    /// Close a session.  Idempotent — closing an already-closed session
    /// is a no-op that emits the `GroupClosed` event regardless so the
    /// frontend's local state can converge.
    pub async fn close_session(
        &self,
        session_id: SyncSessionId,
        sink: Option<&SyncEventSink>,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let result = sqlx::query(
            "UPDATE sync_sessions
                SET status = 'closed', closed_at = ?
              WHERE id = ? AND status = 'active'",
        )
        .bind(now)
        .bind(session_id)
        .execute(self.db.pool())
        .await?;
        if let Some(sink) = sink {
            if result.rows_affected() > 0 {
                sink(SyncEvent::StateChanged {
                    session_id,
                    status: SyncStatus::Closed,
                });
            }
            sink(SyncEvent::GroupClosed { session_id });
        }
        debug!(session_id, "sync session closed");
        Ok(())
    }

    /// Read a session's current shape (panes + leader + status).
    /// Errors with `NotFound` if the session id is unknown.
    pub async fn get_session(&self, session_id: SyncSessionId) -> Result<SyncSession, AppError> {
        let row = sqlx::query(
            "SELECT id, created_at, closed_at, layout, leader_pane_index, status
               FROM sync_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(self.db.pool())
        .await?;
        let row = row.ok_or(AppError::NotFound)?;

        let layout_str: String = row.try_get("layout")?;
        let layout = SyncLayout::from_db_str(&layout_str).ok_or_else(|| AppError::Internal {
            detail: format!("unknown sync layout '{layout_str}' in row {session_id}"),
        })?;
        let status_str: String = row.try_get("status")?;
        let status = SyncStatus::from_db_str(&status_str).ok_or_else(|| AppError::Internal {
            detail: format!("unknown sync status '{status_str}' in row {session_id}"),
        })?;
        let leader_pane_index: Option<i64> = row.try_get("leader_pane_index")?;

        let pane_rows = sqlx::query(
            "SELECT pane_index, vod_id, volume, muted, joined_at
               FROM sync_session_panes
              WHERE session_id = ?
              ORDER BY pane_index ASC",
        )
        .bind(session_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut panes = Vec::with_capacity(pane_rows.len());
        for r in pane_rows {
            let muted_int: i64 = r.try_get("muted")?;
            panes.push(SyncMember {
                pane_index: r.try_get("pane_index")?,
                vod_id: r.try_get("vod_id")?,
                volume: r.try_get("volume")?,
                muted: muted_int != 0,
                joined_at: r.try_get("joined_at")?,
            });
        }
        Ok(SyncSession {
            id: row.try_get("id")?,
            created_at: row.try_get("created_at")?,
            closed_at: row.try_get("closed_at")?,
            layout,
            leader_pane_index,
            status,
            panes,
        })
    }

    /// Promote a pane to leader.  Errors with `InvalidInput` if the
    /// pane index isn't a member of the session.  Emits
    /// `LeaderChanged`; if the new leader is the same as the current
    /// leader, this is a no-op (no event fired).
    pub async fn set_leader(
        &self,
        session_id: SyncSessionId,
        pane_index: PaneIndex,
        sink: Option<&SyncEventSink>,
    ) -> Result<SyncSession, AppError> {
        let session = self.get_session(session_id).await?;
        if session.status != SyncStatus::Active {
            return Err(AppError::InvalidInput {
                detail: "cannot change leader on a closed session".into(),
            });
        }
        if !session.panes.iter().any(|p| p.pane_index == pane_index) {
            return Err(AppError::InvalidInput {
                detail: format!("pane {pane_index} is not a member of session {session_id}"),
            });
        }
        if session.leader_pane_index == Some(pane_index) {
            return Ok(session);
        }
        sqlx::query("UPDATE sync_sessions SET leader_pane_index = ? WHERE id = ?")
            .bind(pane_index)
            .bind(session_id)
            .execute(self.db.pool())
            .await?;
        if let Some(sink) = sink {
            sink(SyncEvent::LeaderChanged {
                session_id,
                leader_pane_index: pane_index,
            });
        }
        self.get_session(session_id).await
    }

    /// Apply a group-wide transport command.  v1's transport is
    /// stateless on the backend — the renderer fans the play/pause/
    /// seek/speed call to its `<video>` elements; this method only
    /// validates that the session is in a state where the command
    /// makes sense.  Crucially we do *not* emit `StateChanged` from
    /// here: a play/pause/seek/speed change does not change the
    /// session's lifecycle status, and emitting `StateChanged{Active}`
    /// repeatedly would mislead any observer that re-fetches the
    /// session row on that event.  Lifecycle events fire from open /
    /// close.
    pub async fn apply_transport(
        &self,
        session_id: SyncSessionId,
        command: SyncTransportCommand,
        _sink: Option<&SyncEventSink>,
    ) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if session.status != SyncStatus::Active {
            return Err(AppError::InvalidInput {
                detail: "cannot apply transport to a closed session".into(),
            });
        }
        if let SyncTransportCommand::SetSpeed { speed } = command
            && !(0.25..=4.0).contains(&speed)
        {
            return Err(AppError::InvalidInput {
                detail: format!("speed {speed} out of supported range [0.25, 4.0]"),
            });
        }
        // Discriminant-only debug log — never the user-supplied
        // payload values, to avoid surfacing renderer-controlled data
        // in any future log sink.
        let kind = match command {
            SyncTransportCommand::Play => "play",
            SyncTransportCommand::Pause => "pause",
            SyncTransportCommand::Seek { .. } => "seek",
            SyncTransportCommand::SetSpeed { .. } => "set_speed",
        };
        debug!(session_id, kind, "sync transport applied");
        Ok(())
    }

    /// Record a drift measurement.  Above-threshold reports emit
    /// `DriftCorrected` (the renderer does the actual seek; the
    /// service only fans the event for observability).  Sub-threshold
    /// reports are dropped — they're noise for the UI.
    ///
    /// Validates: session exists + active, `pane_index` is a member,
    /// `drift_ms` is finite (NaN / Infinity are rejected before the
    /// threshold comparison since `f64::abs(NaN) < threshold` is
    /// `false`, which would otherwise silently emit a phantom event
    /// with `corrected_to_seconds: NaN`).
    pub async fn record_drift(
        &self,
        session_id: SyncSessionId,
        measurement: DriftMeasurement,
        sink: Option<&SyncEventSink>,
    ) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if session.status != SyncStatus::Active {
            return Err(AppError::InvalidInput {
                detail: "cannot record drift on a closed session".into(),
            });
        }
        if !session
            .panes
            .iter()
            .any(|p| p.pane_index == measurement.pane_index)
        {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "pane {} is not a member of session {session_id}",
                    measurement.pane_index
                ),
            });
        }
        if !measurement.drift_ms.is_finite()
            || !measurement.expected_position_seconds.is_finite()
            || !measurement.follower_position_seconds.is_finite()
        {
            return Err(AppError::InvalidInput {
                detail: "drift measurement must contain finite numbers".into(),
            });
        }
        let settings = self.settings.get().await?;
        let threshold = settings.sync_drift_threshold_ms;
        if measurement.drift_ms.abs() < threshold {
            return Ok(());
        }
        if let Some(sink) = sink {
            sink(SyncEvent::DriftCorrected {
                session_id,
                pane_index: measurement.pane_index,
                drift_ms: measurement.drift_ms,
                corrected_to_seconds: measurement.expected_position_seconds,
            });
        }
        Ok(())
    }

    /// Fan a pre-debounced "out of range" report from the renderer.
    /// Validates: session exists + active and `pane_index` is a
    /// member, so the event payload can be trusted by any future
    /// consumer that uses `pane_index` for indexing.
    pub async fn report_out_of_range(
        &self,
        session_id: SyncSessionId,
        pane_index: PaneIndex,
        sink: Option<&SyncEventSink>,
    ) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if session.status != SyncStatus::Active {
            return Err(AppError::InvalidInput {
                detail: "cannot report out-of-range on a closed session".into(),
            });
        }
        if !session.panes.iter().any(|p| p.pane_index == pane_index) {
            return Err(AppError::InvalidInput {
                detail: format!("pane {pane_index} is not a member of session {session_id}"),
            });
        }
        if let Some(sink) = sink {
            sink(SyncEvent::MemberOutOfRange {
                session_id,
                pane_index,
            });
        }
        Ok(())
    }

    /// Compute the wall-clock overlap of the given VODs.  Used by the
    /// frontend to bound the seek slider before opening a session
    /// (preview) and after opening (canonical bound).
    ///
    /// v1 caps `vod_ids` at 2 entries to mirror `open_session`'s
    /// constraint and to bound the per-VOD lookup cost.  A v2
    /// expansion to N panes would relax this with an explicit cap.
    pub async fn overlap_of(&self, vod_ids: Vec<String>) -> Result<OverlapResult, AppError> {
        if vod_ids.is_empty() {
            return Err(AppError::InvalidInput {
                detail: "overlap requires at least one vod_id".into(),
            });
        }
        if vod_ids.len() > 2 {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "overlap_of supports up to 2 VODs in v1, got {}",
                    vod_ids.len()
                ),
            });
        }
        let mut ranges = Vec::with_capacity(vod_ids.len());
        for id in &vod_ids {
            let r = self.lookup_vod_range(id).await?;
            ranges.push(MemberRange {
                stream_started_at: r.stream_started_at,
                duration_seconds: r.duration_seconds,
            });
        }
        let window = compute_overlap(&ranges);
        Ok(OverlapResult { vod_ids, window })
    }

    async fn lookup_vod_range(&self, vod_id: &str) -> Result<VodRange, AppError> {
        let row = sqlx::query(
            "SELECT stream_started_at, duration_seconds
               FROM vods WHERE twitch_video_id = ?",
        )
        .bind(vod_id)
        .fetch_optional(self.db.pool())
        .await?;
        let row = row.ok_or_else(|| AppError::InvalidInput {
            detail: format!("vod_id '{vod_id}' not found"),
        })?;
        Ok(VodRange {
            stream_started_at: row.try_get("stream_started_at")?,
            duration_seconds: row.try_get("duration_seconds")?,
        })
    }
}

/// Resolve the leader-pane-index from `app_settings.sync_default_leader`.
/// `'first-opened'` (the only v1 vocabulary entry) maps to pane index 0.
fn default_leader_index(settings: &AppSettings) -> PaneIndex {
    match settings.sync_default_leader.as_str() {
        "first-opened" => 0,
        // Forward-compatibility: a future 'longest' or other strategy
        // would land here.  For now, an unknown value falls back to
        // pane 0 rather than failing — the column-level CHECK already
        // gates writes, so a non-vocabulary value can only enter via a
        // direct DB edit, which the user has already opted into.
        _ => 0,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn fixture() -> (SyncService, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000_000));
        seed_streamer(&db, "u1", "primary").await;
        seed_streamer(&db, "u2", "secondary").await;
        seed_streamer(&db, "u3", "tertiary").await;
        // v1 = stream u1 1000..2200, v2 = stream u2 1100..2300, overlap 1100..2200.
        seed_vod(&db, "v1", "u1", 1000, 1200).await;
        seed_vod(&db, "v2", "u2", 1100, 1200).await;
        seed_vod(&db, "v3", "u3", 5000, 600).await; // disjoint with v1/v2
        let settings = SettingsService::new(db.clone(), clock.clone());
        (SyncService::new(db.clone(), clock, settings), db)
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
                published_at, url, duration_seconds, ingest_status, first_seen_at, last_seen_at)
             VALUES (?, ?, 'title', ?, ?, 'https://twitch.tv', ?, 'eligible', ?, ?)",
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

    fn capturing_sink() -> (SyncEventSink, Arc<std::sync::Mutex<Vec<SyncEvent>>>) {
        let store = Arc::new(std::sync::Mutex::new(Vec::new()));
        let cloned = store.clone();
        let sink: SyncEventSink = Arc::new(move |ev| cloned.lock().unwrap().push(ev));
        (sink, store)
    }

    #[tokio::test]
    async fn open_session_persists_panes_and_emits_events() {
        let (svc, _db) = fixture().await;
        let (sink, events) = capturing_sink();
        let session = svc
            .open_session(
                vec!["v1".into(), "v2".into()],
                SyncLayout::Split5050,
                Some(&sink),
            )
            .await
            .unwrap();
        assert_eq!(session.panes.len(), 2);
        assert_eq!(session.panes[0].vod_id, "v1");
        assert_eq!(session.panes[1].vod_id, "v2");
        assert_eq!(session.leader_pane_index, Some(0));
        assert_eq!(session.status, SyncStatus::Active);
        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::StateChanged {
                status: SyncStatus::Active,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::LeaderChanged {
                leader_pane_index: 0,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn open_session_rejects_one_pane() {
        let (svc, _db) = fixture().await;
        let err = svc
            .open_session(vec!["v1".into()], SyncLayout::Split5050, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn open_session_rejects_duplicate_vod() {
        let (svc, _db) = fixture().await;
        let err = svc
            .open_session(vec!["v1".into(), "v1".into()], SyncLayout::Split5050, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn open_session_rejects_unknown_vod() {
        let (svc, _db) = fixture().await;
        let err = svc
            .open_session(
                vec!["v1".into(), "ghost".into()],
                SyncLayout::Split5050,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn close_session_is_idempotent_and_emits_group_closed() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        svc.close_session(session.id, Some(&sink)).await.unwrap();
        svc.close_session(session.id, Some(&sink)).await.unwrap();
        let events = events.lock().unwrap();
        // Only one StateChanged{Closed}; both calls emit GroupClosed.
        let state_changes = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SyncEvent::StateChanged {
                        status: SyncStatus::Closed,
                        ..
                    }
                )
            })
            .count();
        let closed = events
            .iter()
            .filter(|e| matches!(e, SyncEvent::GroupClosed { .. }))
            .count();
        assert_eq!(state_changes, 1);
        assert_eq!(closed, 2);
    }

    #[tokio::test]
    async fn set_leader_to_existing_pane_is_a_noop() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        let updated = svc.set_leader(session.id, 0, Some(&sink)).await.unwrap();
        assert_eq!(updated.leader_pane_index, Some(0));
        let events = events.lock().unwrap();
        assert!(events.is_empty(), "no events expected; got {events:?}");
    }

    #[tokio::test]
    async fn set_leader_to_other_pane_emits_leader_changed() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        let updated = svc.set_leader(session.id, 1, Some(&sink)).await.unwrap();
        assert_eq!(updated.leader_pane_index, Some(1));
        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::LeaderChanged {
                leader_pane_index: 1,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn set_leader_rejects_unknown_pane() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let err = svc.set_leader(session.id, 5, None).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn set_leader_rejects_closed_session() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        svc.close_session(session.id, None).await.unwrap();
        let err = svc.set_leader(session.id, 1, None).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn record_drift_rejects_closed_session() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        svc.close_session(session.id, None).await.unwrap();
        let err = svc
            .record_drift(
                session.id,
                DriftMeasurement {
                    pane_index: 1,
                    follower_position_seconds: 10.0,
                    expected_position_seconds: 10.6,
                    drift_ms: 600.0,
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn record_drift_rejects_unknown_pane() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let err = svc
            .record_drift(
                session.id,
                DriftMeasurement {
                    pane_index: 5,
                    follower_position_seconds: 10.0,
                    expected_position_seconds: 10.6,
                    drift_ms: 600.0,
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn record_drift_rejects_non_finite_values() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let err = svc
            .record_drift(
                session.id,
                DriftMeasurement {
                    pane_index: 1,
                    follower_position_seconds: 10.0,
                    expected_position_seconds: 10.0,
                    drift_ms: f64::NAN,
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
        let err = svc
            .record_drift(
                session.id,
                DriftMeasurement {
                    pane_index: 1,
                    follower_position_seconds: f64::INFINITY,
                    expected_position_seconds: 10.0,
                    drift_ms: 600.0,
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn report_out_of_range_rejects_closed_session() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        svc.close_session(session.id, None).await.unwrap();
        let err = svc
            .report_out_of_range(session.id, 1, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn report_out_of_range_rejects_unknown_pane() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let err = svc
            .report_out_of_range(session.id, 9, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn overlap_rejects_too_many_vods() {
        let (svc, _db) = fixture().await;
        let err = svc
            .overlap_of(vec!["v1".into(), "v2".into(), "v3".into()])
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn overlap_rejects_empty_input() {
        let (svc, _db) = fixture().await;
        let err = svc.overlap_of(vec![]).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn record_drift_below_threshold_is_dropped() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        svc.record_drift(
            session.id,
            DriftMeasurement {
                pane_index: 1,
                follower_position_seconds: 10.0,
                expected_position_seconds: 10.05,
                drift_ms: 50.0,
            },
            Some(&sink),
        )
        .await
        .unwrap();
        let events = events.lock().unwrap();
        assert!(
            events.is_empty(),
            "drift below 250 ms threshold should not emit; got {events:?}"
        );
    }

    #[tokio::test]
    async fn record_drift_above_threshold_emits_drift_corrected() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        svc.record_drift(
            session.id,
            DriftMeasurement {
                pane_index: 1,
                follower_position_seconds: 10.0,
                expected_position_seconds: 10.5,
                drift_ms: 500.0,
            },
            Some(&sink),
        )
        .await
        .unwrap();
        let events = events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SyncEvent::DriftCorrected { pane_index: 1, .. }))
        );
    }

    #[tokio::test]
    async fn apply_transport_rejects_closed_session() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        svc.close_session(session.id, None).await.unwrap();
        let err = svc
            .apply_transport(session.id, SyncTransportCommand::Play, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn apply_transport_rejects_speed_outside_range() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let err = svc
            .apply_transport(
                session.id,
                SyncTransportCommand::SetSpeed { speed: 10.0 },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn report_out_of_range_emits_event() {
        let (svc, _db) = fixture().await;
        let session = svc
            .open_session(vec!["v1".into(), "v2".into()], SyncLayout::Split5050, None)
            .await
            .unwrap();
        let (sink, events) = capturing_sink();
        svc.report_out_of_range(session.id, 1, Some(&sink))
            .await
            .unwrap();
        let events = events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SyncEvent::MemberOutOfRange { pane_index: 1, .. }))
        );
    }

    #[tokio::test]
    async fn overlap_of_two_overlapping_vods() {
        let (svc, _db) = fixture().await;
        let result = svc
            .overlap_of(vec!["v1".into(), "v2".into()])
            .await
            .unwrap();
        assert_eq!(result.window.start_at, 1100);
        assert_eq!(result.window.end_at, 2200);
        assert!(result.window.is_non_empty());
    }

    #[tokio::test]
    async fn overlap_of_disjoint_vods_is_empty() {
        let (svc, _db) = fixture().await;
        let result = svc
            .overlap_of(vec!["v1".into(), "v3".into()])
            .await
            .unwrap();
        assert!(!result.window.is_non_empty());
    }

    #[tokio::test]
    async fn overlap_rejects_unknown_vod() {
        let (svc, _db) = fixture().await;
        let err = svc
            .overlap_of(vec!["v1".into(), "ghost".into()])
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn get_session_returns_not_found_for_unknown_id() {
        let (svc, _db) = fixture().await;
        let err = svc.get_session(9999).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }
}
