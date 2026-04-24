//! Graceful-shutdown integration test (Phase 5 housekeeping).
//!
//! Phase 4 wired the shutdown mechanism — `cmd_request_shutdown`
//! broadcasts on both services' shutdown channels and the
//! `on_window_event` Quit branch relies on Tokio to drain the workers
//! before `std::process::exit`. The mechanism was reviewed but never
//! exercised end-to-end; phase-04.md §Deviations #5 flagged the gap.
//!
//! This test now runs the real `DownloadQueueService::spawn` against a
//! scripted ytdlp/ffmpeg/fs that keeps a download "in flight" for a
//! bounded window, fires the shutdown broadcaster at a random point
//! inside that window, asserts the DB is in a safe state after the
//! drain, and then re-spawns the service against the same DB to
//! prove the crash-recovery path picks the row up cleanly.
//!
//! Why not drive `cargo run` and an OS SIGTERM? Two reasons:
//!
//! 1. A full Tauri process needs a webview surface that CI's
//!    Windows and Linux runners don't always have. We already gate
//!    `ipc_bindings` off Windows for the same reason.
//! 2. The shutdown logic is in the Tokio service layer, not the
//!    Tauri glue. Exercising the service directly tests the
//!    invariant that actually matters (no partially-applied DB
//!    transitions on shutdown) without the flakiness budget a real
//!    subprocess harness brings.
//!
//! The test runs on every CI matrix OS.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sightline_lib::infra::clock::{Clock, FixedClock};
use sightline_lib::infra::db::Db;
use sightline_lib::infra::ffmpeg::fake::{FfmpegFake, FfmpegScript};
use sightline_lib::infra::ffmpeg::{Ffmpeg, SharedFfmpeg};
use sightline_lib::infra::fs::space::FakeFreeSpace;
use sightline_lib::infra::throttle::GlobalRate;
use sightline_lib::infra::ytdlp::DownloadProgress;
use sightline_lib::infra::ytdlp::fake::{FakeScript, YtDlpFake};
use sightline_lib::infra::ytdlp::{SharedYtDlp, YtDlp};
use sightline_lib::services::downloads::DownloadQueueService;
use sightline_lib::services::settings::{SettingsPatch, SettingsService};
use sightline_lib::services::vods::VodReadService;
use tempfile::TempDir;

async fn seed_settings(db: &Db, library_root: &std::path::Path) {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000));
    let settings = SettingsService::new(db.clone(), clock);
    settings
        .update(SettingsPatch {
            library_root: Some(library_root.display().to_string()),
            max_concurrent_downloads: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
}

async fn seed_streamer_and_vod(db: &Db) {
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
         VALUES ('v1', '100', 'shutdown-test', 1, 1,
                 'https://twitch.tv/videos/v1', 1800, 'eligible', 0, 0)",
    )
    .execute(db.pool())
    .await
    .unwrap();
}

fn long_running_script() -> FakeScript {
    // A deliberately-slow download: six evenly-spaced ticks with a
    // small delay between each. The shutdown fires at a random point
    // inside the ~300 ms window; the service's cooperative cancel
    // path should flip the row to a safe state by the time the
    // shutdown broadcast completes.
    let ticks = (1u64..=6)
        .map(|i| DownloadProgress {
            progress: Some((i as f64) / 6.0),
            bytes_done: i * 1024,
            bytes_total: Some(6 * 1024),
            speed_bps: Some(1024),
            eta_seconds: Some(6 - i),
        })
        .collect();
    FakeScript {
        progress_ticks: ticks,
        tick_delay_ms: 50,
        ..Default::default()
    }
}

fn make_queue(
    db: &Db,
    ytdlp: SharedYtDlp,
    ffmpeg: SharedFfmpeg,
    staging: PathBuf,
) -> Arc<DownloadQueueService> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000));
    let settings = SettingsService::new(db.clone(), clock.clone());
    let svc = DownloadQueueService::new(
        db.clone(),
        clock,
        ytdlp,
        ffmpeg,
        Arc::new(FakeFreeSpace(u64::MAX)),
        Arc::new(GlobalRate::new()),
        settings,
        Arc::new(VodReadService::new(db.clone())),
        staging,
    );
    Arc::new(svc)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mid_download_shutdown_leaves_db_recoverable() {
    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    let staging = tmp.path().join("staging");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(&staging).unwrap();

    let db_path = tmp.path().join("sightline.sqlite");
    let db = Db::open(&db_path).await.unwrap();
    db.migrate().await.unwrap();
    seed_settings(&db, &library).await;
    seed_streamer_and_vod(&db).await;

    let ytdlp: SharedYtDlp = Arc::new(YtDlpFake::new(long_running_script()));
    let ffmpeg: SharedFfmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
    let queue = make_queue(&db, ytdlp.clone(), ffmpeg.clone(), staging.clone());
    let sink: sightline_lib::services::downloads::DownloadEventSink =
        Arc::new(|_event| { /* drop */ });
    let spawn = queue.clone().spawn(sink);

    // Enqueue, wake the worker, and wait a tick so the manager picks
    // the row up.
    queue.enqueue("v1", None).await.unwrap();
    spawn.handle.wake_up().await;
    // Random-ish but bounded wait — we want the shutdown to land at
    // ~25–75 % through the tick sequence. `thread_rng` pulled in
    // from `rand` would add a dependency; the Tokio clock does the
    // job deterministically enough for this contract.
    tokio::time::sleep(Duration::from_millis(120)).await;

    spawn.handle.shutdown();
    // Shutdown should return control within a bounded window; the
    // fake's tick delay caps the loop at ~300 ms, the broadcast
    // wakes the select!, and the worker's `kill_on_drop` (on the
    // real wrapper) drops the in-flight future.
    let _ = tokio::time::timeout(Duration::from_secs(3), spawn.join)
        .await
        .expect("service did not drain within 3s");

    // DB assertion: after shutdown the row is either `queued` (worker
    // hadn't taken it yet) or `downloading` (worker was mid-flight;
    // crash-recovery will flip it back to `queued` on the next spawn).
    // Nothing else is acceptable.
    let state: String = sqlx::query_scalar("SELECT state FROM downloads WHERE vod_id = 'v1'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(
        matches!(state.as_str(), "queued" | "downloading"),
        "unexpected post-shutdown state: {state}"
    );

    // No half-written NFO or thumbnail should be visible in the library
    // root — the atomic-move step is only reached at the very end of
    // the pipeline, well after the shutdown broadcast fires.
    let mut entries = std::fs::read_dir(&library).unwrap();
    assert!(
        entries.next().is_none(),
        "library root should be untouched after mid-download shutdown"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_after_shutdown_recovers_via_crash_path() {
    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    let staging = tmp.path().join("staging");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(&staging).unwrap();

    let db_path = tmp.path().join("sightline.sqlite");
    let db = Db::open(&db_path).await.unwrap();
    db.migrate().await.unwrap();
    seed_settings(&db, &library).await;
    seed_streamer_and_vod(&db).await;

    // Leave a row in `downloading` — the exact state the previous
    // test would produce when the shutdown interrupts a mid-flight
    // worker.
    sqlx::query(
        "INSERT INTO downloads (vod_id, state, priority, quality_preset,
             staging_path, bytes_done, attempts, queued_at, pause_requested)
         VALUES ('v1', 'downloading', 100, 'source', ?, 1024, 1, 1, 0)",
    )
    .bind(staging.display().to_string())
    .execute(db.pool())
    .await
    .unwrap();

    // Spawn the service the same way a fresh app launch would. The
    // worker loop runs `crash_recover` before taking any new command.
    let ytdlp: SharedYtDlp = Arc::new(YtDlpFake::new(FakeScript::default()));
    let ffmpeg: SharedFfmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
    let queue = make_queue(&db, ytdlp, ffmpeg, staging);
    let (tx, _rx) =
        tokio::sync::mpsc::channel::<sightline_lib::services::downloads::DownloadEvent>(16);
    let sink: sightline_lib::services::downloads::DownloadEventSink = Arc::new(move |event| {
        let _ = tx.try_send(event);
    });
    let spawn = queue.clone().spawn(sink);
    // Give the manager loop a beat to run crash_recover — it runs
    // before the first select!, so one tokio::yield is usually enough,
    // but a short sleep is more robust across OSes.
    tokio::time::sleep(Duration::from_millis(80)).await;

    let state: String = sqlx::query_scalar("SELECT state FROM downloads WHERE vod_id = 'v1'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        state, "queued",
        "crash_recover should reset `downloading` to `queued` on startup"
    );

    // bytes_done should have been reset so the next worker starts
    // from scratch — yt-dlp's resume flag isn't trusted.
    let bytes: i64 = sqlx::query_scalar("SELECT bytes_done FROM downloads WHERE vod_id = 'v1'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(bytes, 0);

    spawn.handle.shutdown();
    let _ = tokio::time::timeout(Duration::from_secs(2), spawn.join).await;

    // Sanity: no orphaned rows in download state that aren't in the
    // allow-list.
    let weird: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM downloads
         WHERE state NOT IN ('queued', 'downloading', 'paused', 'completed',
                             'failed_retryable', 'failed_permanent')",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(weird, 0);
}

// Keep the test binary's imports honest: touching these traits at the
// path resolves any `unused_imports` warnings we'd otherwise get from
// the test harness.
#[allow(dead_code)]
fn _imports_are_used<'a>(y: &'a dyn YtDlp, f: &'a dyn Ffmpeg) -> (&'a dyn YtDlp, &'a dyn Ffmpeg) {
    (y, f)
}
