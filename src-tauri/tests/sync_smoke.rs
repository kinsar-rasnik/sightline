//! Multi-view sync engine smoke test (Phase 6).
//!
//! Drives the full `services::sync::SyncService` flow against an
//! in-memory SQLite + the live migration set:
//!   open → set_leader → seek → drift_correction → close.
//!
//! Mirrors the mission's specified sequence so a regression in any
//! single step surfaces here even before the unit tests in
//! `services::sync::tests` would.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use sightline_lib::domain::sync::{DriftMeasurement, SyncLayout, SyncStatus};
use sightline_lib::infra::clock::{Clock, FixedClock};
use sightline_lib::infra::db::Db;
use sightline_lib::services::settings::SettingsService;
use sightline_lib::services::sync::{SyncEvent, SyncEventSink, SyncService};

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
async fn open_promote_seek_drift_close_drives_full_event_sequence() {
    let db = Db::open_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    seed_streamer(&db, "u1", "primary").await;
    seed_streamer(&db, "u2", "secondary").await;
    // Two co-streamers, fully overlapping windows.
    seed_vod(&db, "v1", "u1", 1_000, 1_200).await;
    seed_vod(&db, "v2", "u2", 1_100, 1_200).await;

    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000_000));
    let svc = SyncService::new(
        db.clone(),
        clock.clone(),
        SettingsService::new(db.clone(), clock.clone()),
    );
    let (sink, events) = capturing_sink();

    // 1. open → emits StateChanged{Active} + LeaderChanged{0}.
    let session = svc
        .open_session(
            vec!["v1".into(), "v2".into()],
            SyncLayout::Split5050,
            Some(&sink),
        )
        .await
        .unwrap();
    assert_eq!(session.status, SyncStatus::Active);
    assert_eq!(session.leader_pane_index, Some(0));
    assert_eq!(session.panes.len(), 2);

    // 2. set_leader (0 → 1) emits LeaderChanged{1}.
    let updated = svc.set_leader(session.id, 1, Some(&sink)).await.unwrap();
    assert_eq!(updated.leader_pane_index, Some(1));

    // 3. transport seek emits StateChanged{Active}.
    svc.apply_transport(
        session.id,
        sightline_lib::domain::sync::SyncTransportCommand::Seek {
            wall_clock_ts: 1_500,
        },
        Some(&sink),
    )
    .await
    .unwrap();

    // 4. drift correction at 600 ms (above the 250 ms default
    //    threshold) emits DriftCorrected.
    svc.record_drift(
        session.id,
        DriftMeasurement {
            pane_index: 0,
            follower_position_seconds: 200.0,
            expected_position_seconds: 200.6,
            drift_ms: 600.0,
        },
        Some(&sink),
    )
    .await
    .unwrap();

    // 5. close emits StateChanged{Closed} + GroupClosed.
    svc.close_session(session.id, Some(&sink)).await.unwrap();

    let events = events.lock().unwrap();
    let kinds: Vec<&'static str> = events
        .iter()
        .map(|ev| match ev {
            SyncEvent::StateChanged {
                status: SyncStatus::Active,
                ..
            } => "state:active",
            SyncEvent::StateChanged {
                status: SyncStatus::Closed,
                ..
            } => "state:closed",
            SyncEvent::DriftCorrected { .. } => "drift",
            SyncEvent::LeaderChanged { .. } => "leader",
            SyncEvent::MemberOutOfRange { .. } => "out_of_range",
            SyncEvent::GroupClosed { .. } => "group_closed",
        })
        .collect();

    // Order matters: open path emits state:active then leader, then
    // the explicit promote, transport, drift, close.
    assert_eq!(
        kinds,
        vec![
            "state:active",
            "leader",
            "leader",
            "state:active",
            "drift",
            "state:closed",
            "group_closed",
        ],
        "unexpected event sequence: {kinds:?}"
    );
}

#[tokio::test]
async fn out_of_range_report_emits_event() {
    let db = Db::open_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    seed_streamer(&db, "u1", "primary").await;
    seed_streamer(&db, "u2", "secondary").await;
    seed_vod(&db, "v1", "u1", 0, 1_000).await;
    seed_vod(&db, "v2", "u2", 100, 1_000).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000_000));
    let svc = SyncService::new(
        db.clone(),
        clock.clone(),
        SettingsService::new(db.clone(), clock.clone()),
    );

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
            .any(|e| matches!(e, SyncEvent::MemberOutOfRange { pane_index: 1, .. })),
        "expected MemberOutOfRange event for pane 1, got {events:?}"
    );
}

#[tokio::test]
async fn overlap_query_returns_intersection_for_two_streams() {
    let db = Db::open_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    seed_streamer(&db, "u1", "primary").await;
    seed_streamer(&db, "u2", "secondary").await;
    seed_streamer(&db, "u3", "tertiary").await;
    // v1 → u1 100..1100, v2 → u2 500..1500, overlap = 500..1100.
    seed_vod(&db, "v1", "u1", 100, 1_000).await;
    seed_vod(&db, "v2", "u2", 500, 1_000).await;
    // v3 fully outside the v1/v2 overlap → disjoint check.
    seed_vod(&db, "v3", "u3", 5_000, 100).await;

    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000_000));
    let svc = SyncService::new(
        db.clone(),
        clock.clone(),
        SettingsService::new(db.clone(), clock.clone()),
    );
    let result = svc
        .overlap_of(vec!["v1".into(), "v2".into()])
        .await
        .unwrap();
    assert_eq!(result.window.start_at, 500);
    assert_eq!(result.window.end_at, 1_100);
    assert_eq!(result.vod_ids, vec!["v1".to_string(), "v2".to_string()]);
    assert_eq!(result.window.duration_seconds(), 600);

    let disjoint = svc
        .overlap_of(vec!["v1".into(), "v3".into()])
        .await
        .unwrap();
    assert!(!disjoint.window.is_non_empty());
}
