//! Sightline — library root.
//!
//! Layering: `commands` (thin IPC surface) → `services` (orchestration)
//! → `domain` (pure types) / `infra` (DB, HTTP, sidecars). No back-edges.

pub mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod ipc;
pub mod services;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, Manager};
use tracing::{debug, error, info, warn};

use crate::infra::clock::{Clock, SystemClock};
use crate::infra::db::Db;
use crate::infra::keychain::{Credentials, OsKeychainCredentials};
use crate::infra::twitch::auth::TwitchAuthenticator;
use crate::infra::twitch::gql::GqlClient;
use crate::infra::twitch::helix::HelixClient;
use crate::services::credentials::CredentialsService;
use crate::services::downloads::{
    DownloadEvent, DownloadEventSink, DownloadQueueHandle, DownloadQueueService,
};
use crate::services::events::{
    CredentialsChangedEvent, DownloadCompletedEvent, DownloadFailedEvent, DownloadProgressEvent,
    DownloadStateChangedEvent, EV_CREDENTIALS_CHANGED, EV_DOWNLOAD_COMPLETED, EV_DOWNLOAD_FAILED,
    EV_DOWNLOAD_PROGRESS, EV_DOWNLOAD_STATE_CHANGED, EV_LIBRARY_MIGRATING,
    EV_LIBRARY_MIGRATION_COMPLETED, EV_LIBRARY_MIGRATION_FAILED, EV_POLL_FINISHED, EV_POLL_STARTED,
    EV_STREAMER_ADDED, EV_STREAMER_REMOVED, EV_VOD_INGESTED, EV_VOD_UPDATED, LibraryMigratingEvent,
    LibraryMigrationCompletedEvent, LibraryMigrationFailedEvent, PollFinishedEvent,
    PollStartedEvent, StreamerAddedEvent, StreamerRemovedEvent, VodIngestedEvent, VodUpdatedEvent,
};
use crate::services::ingest::{IngestEvent, IngestService};
use crate::services::library_migrator::{
    LibraryMigrationEvent, LibraryMigratorService, MigrationSink,
};
use crate::services::media_assets::MediaAssetsService;
use crate::services::notifications::NotificationService;
use crate::services::poller::{PollerEvent, PollerHandle, PollerService};
use crate::services::settings::SettingsService;
use crate::services::shortcuts::ShortcutsService;
use crate::services::storage::StorageService;
use crate::services::streamers::StreamerService;
use crate::services::timeline_indexer::TimelineIndexerService;
use crate::services::vods::VodReadService;

/// Shared application state. One instance is constructed during
/// `setup` and managed by Tauri; each command handler pulls the
/// services it needs off it via `tauri::State<'_, AppState>`.
pub struct AppState {
    pub started_at: i64,
    pub db: Db,
    pub credentials: Arc<CredentialsService>,
    pub streamers: Arc<StreamerService>,
    pub settings: Arc<SettingsService>,
    pub vods: Arc<VodReadService>,
    pub poller_handle: PollerHandle,
    // --- Phase 3 ---
    pub downloads: Arc<DownloadQueueService>,
    pub downloads_handle: DownloadQueueHandle,
    pub library_migrator: Arc<LibraryMigratorService>,
    pub library_migration_sink: MigrationSink,
    pub storage: Arc<StorageService>,
    // --- Phase 4 ---
    pub timeline: Arc<TimelineIndexerService>,
    pub shortcuts: Arc<ShortcutsService>,
    pub notifications: Arc<NotificationService>,
    // --- Phase 5 ---
    pub media_assets: Arc<MediaAssetsService>,
    /// Sync mirror of `app_settings.window_close_behavior` so
    /// `on_window_event` can read the current preference without
    /// touching the async settings service. 0 = hide, 1 = quit.
    /// Updated by `cmd_set_window_close_behavior`.
    pub close_behavior: Arc<AtomicU8>,
    pub app_handle: tauri::AppHandle,
}

/// Encode a [`crate::services::settings::WindowCloseBehavior`] as a u8
/// for atomic storage on `AppState`.
pub fn encode_close_behavior(b: crate::services::settings::WindowCloseBehavior) -> u8 {
    match b {
        crate::services::settings::WindowCloseBehavior::Hide => 0,
        crate::services::settings::WindowCloseBehavior::Quit => 1,
    }
}

/// Counterpart of [`encode_close_behavior`]. Any value other than 1 is
/// treated as `Hide` so an uninitialised / corrupt atomic value can
/// never cause a surprising quit.
pub fn decode_close_behavior(v: u8) -> crate::services::settings::WindowCloseBehavior {
    match v {
        1 => crate::services::settings::WindowCloseBehavior::Quit,
        _ => crate::services::settings::WindowCloseBehavior::Hide,
    }
}

impl AppState {
    pub fn emit_credentials_changed(&self, configured: bool) {
        let _ = self.app_handle.emit(
            EV_CREDENTIALS_CHANGED,
            CredentialsChangedEvent { configured },
        );
    }

    pub fn emit_streamer_added(&self, twitch_user_id: &str, login: &str) {
        let _ = self.app_handle.emit(
            EV_STREAMER_ADDED,
            StreamerAddedEvent {
                twitch_user_id: twitch_user_id.to_owned(),
                login: login.to_owned(),
            },
        );
    }

    pub fn emit_streamer_removed(&self, twitch_user_id: &str) {
        let _ = self.app_handle.emit(
            EV_STREAMER_REMOVED,
            StreamerRemovedEvent {
                twitch_user_id: twitch_user_id.to_owned(),
            },
        );
    }
}

/// Entry point called by `main.rs`. Separate to make the app testable from
/// integration tests without invoking the GUI.
pub fn run() {
    init_tracing();

    let specta_builder = ipc::ipc_builder();

    // Emit TypeScript bindings on every debug build. Release builds read
    // the committed file instead — see ADR-0007. CI enforces drift with
    // a dedicated test (`tests/ipc_bindings.rs`).
    #[cfg(debug_assertions)]
    if let Err(e) = ipc::export_bindings(&specta_builder) {
        warn!(error = %e, "tauri-specta export skipped");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            // LaunchAgent is the standard macOS approach and works
            // without Developer ID signing (AppleScript would prompt
            // the user on first start — we prefer the silent plist
            // path). Windows uses HKCU\Run and Linux uses XDG
            // autostart; MacosLauncher controls only the macOS variant.
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let started_at = unix_now();
                let db_path = resolve_db_path(&handle)?;
                let db = Db::open(&db_path).await?;
                db.migrate().await?;

                let clock: Arc<dyn Clock> = Arc::new(SystemClock);
                let http = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .map_err(|e| error::AppError::Internal {
                        detail: format!("http client: {e}"),
                    })?;

                let keychain: Arc<dyn Credentials> = Arc::new(OsKeychainCredentials);
                let auth = Arc::new(TwitchAuthenticator::new(
                    http.clone(),
                    clock.clone(),
                    keychain.clone(),
                ));
                let helix = Arc::new(HelixClient::new(http.clone(), auth.clone(), clock.clone()));
                let gql = Arc::new(GqlClient::new(http));

                let settings_svc = Arc::new(SettingsService::new(db.clone(), clock.clone()));
                let credentials_svc = Arc::new(CredentialsService::new(
                    keychain,
                    auth,
                    SettingsService::new(db.clone(), clock.clone()),
                ));
                let streamers_svc = Arc::new(StreamerService::new(
                    db.clone(),
                    helix.clone(),
                    clock.clone(),
                ));
                let vods_svc = Arc::new(VodReadService::new(db.clone()));
                let ingest_svc = Arc::new(IngestService::new(
                    db.clone(),
                    helix,
                    gql,
                    clock.clone(),
                    SettingsService::new(db.clone(), clock.clone()),
                    streamers_svc.clone(),
                ));

                let poller_svc = Arc::new(PollerService::new(
                    db.clone(),
                    clock.clone(),
                    SettingsService::new(db.clone(), clock.clone()),
                    streamers_svc.clone(),
                    ingest_svc,
                ));

                // Phase 3 services.
                use crate::infra::ffmpeg::SharedFfmpeg;
                use crate::infra::ffmpeg::cli::FfmpegCli;
                use crate::infra::fs::space::{FreeSpaceProbe, SystemFreeSpace};
                use crate::infra::fs::staging;
                use crate::infra::throttle::GlobalRate;
                use crate::infra::ytdlp::SharedYtDlp;
                use crate::infra::ytdlp::cli::YtDlpCli;

                let ytdlp_binary = resolve_sidecar(&handle, "yt-dlp")
                    .unwrap_or_else(|| std::path::PathBuf::from("yt-dlp"));
                let ffmpeg_binary = resolve_sidecar(&handle, "ffmpeg")
                    .unwrap_or_else(|| std::path::PathBuf::from("ffmpeg"));
                let ytdlp: SharedYtDlp = Arc::new(YtDlpCli::new(ytdlp_binary));
                let ffmpeg: SharedFfmpeg = Arc::new(FfmpegCli::new(ffmpeg_binary));
                let space_probe: Arc<dyn FreeSpaceProbe> = Arc::new(SystemFreeSpace);
                let rate = Arc::new(GlobalRate::new());
                let default_staging = staging::default_staging_dir();
                // Non-fatal: a missing staging dir is fine at this
                // point, we create it lazily at enqueue time.
                let _ = staging::cleanup_stale(&default_staging).await;

                let downloads_svc = Arc::new(DownloadQueueService::new(
                    db.clone(),
                    clock.clone(),
                    ytdlp,
                    ffmpeg.clone(),
                    space_probe,
                    rate,
                    SettingsService::new(db.clone(), clock.clone()),
                    vods_svc.clone(),
                    default_staging,
                ));

                let library_migrator = Arc::new(LibraryMigratorService::new(
                    db.clone(),
                    clock.clone(),
                    vods_svc.clone(),
                ));
                let storage_svc = Arc::new(StorageService::new(Arc::new(SystemFreeSpace)));

                // Phase 4 services.
                let timeline_svc =
                    Arc::new(TimelineIndexerService::new(db.clone(), clock.clone()));
                let shortcuts_svc = Arc::new(ShortcutsService::new(db.clone()));
                let notifications_svc =
                    Arc::new(NotificationService::new(handle.clone(), clock.clone()));

                // Phase 5: media-asset resolver (shared by the player
                // route + the grid's hover preview) + preview-frame
                // backfill for pre-Phase-5 downloads.
                let media_assets_svc = Arc::new(MediaAssetsService::new(
                    db.clone(),
                    ffmpeg.clone(),
                    SettingsService::new(db.clone(), clock.clone()),
                ));
                {
                    let backfill = media_assets_svc.clone();
                    tokio::spawn(async move {
                        match backfill.backfill_preview_frames().await {
                            Ok(n) if n > 0 => info!(count = n, "preview backfill complete"),
                            Ok(_) => debug!("preview backfill: nothing to do"),
                            Err(e) => warn!(error = ?e, "preview backfill failed"),
                        }
                    });
                }

                // Seed the close-behavior atomic from the persisted
                // setting so `on_window_event` can read it synchronously.
                let initial_settings = settings_svc.get().await.ok();
                let close_behavior = Arc::new(AtomicU8::new(encode_close_behavior(
                    initial_settings
                        .as_ref()
                        .map(|s| s.window_close_behavior)
                        .unwrap_or(crate::services::settings::WindowCloseBehavior::Hide),
                )));

                // Opportunistic backfill: if we have VODs but no intervals,
                // populate the index in the background so the timeline UI
                // renders quickly after upgrade.
                {
                    let timeline = timeline_svc.clone();
                    let handle_backfill = handle.clone();
                    tokio::spawn(async move {
                        if matches!(timeline.is_empty().await, Ok(true)) {
                            let sink: crate::services::timeline_indexer::IndexerEventSink = Arc::new(
                                move |ev| match ev {
                                    crate::services::timeline_indexer::IndexerEvent::Rebuilding { processed, total } => {
                                        let progress = if total == 0 {
                                            1.0
                                        } else {
                                            (processed as f64) / (total as f64)
                                        };
                                        let _ = handle_backfill.emit(
                                            crate::services::events::EV_TIMELINE_INDEX_REBUILDING,
                                            crate::services::events::TimelineIndexRebuildingEvent {
                                                progress,
                                                processed,
                                                total,
                                            },
                                        );
                                    }
                                    crate::services::timeline_indexer::IndexerEvent::Rebuilt { total } => {
                                        let _ = handle_backfill.emit(
                                            crate::services::events::EV_TIMELINE_INDEX_REBUILT,
                                            crate::services::events::TimelineIndexRebuiltEvent { total },
                                        );
                                    }
                                },
                            );
                            let _ = timeline.rebuild_all(sink).await;
                        }
                    });
                }

                // Event sink: dispatch each PollerEvent variant to the
                // matching Tauri topic. Keeping all event construction
                // in one closure makes it trivial to trace the surface
                // the webview actually sees.
                let sink_handle = handle.clone();
                let sink_timeline = timeline_svc.clone();
                let sink = Arc::new(move |ev: PollerEvent| match ev {
                    PollerEvent::PollStarted {
                        twitch_user_id,
                        started_at,
                    } => {
                        let _ = sink_handle.emit(
                            EV_POLL_STARTED,
                            PollStartedEvent {
                                twitch_user_id,
                                started_at,
                            },
                        );
                    }
                    PollerEvent::PollFinished {
                        twitch_user_id,
                        finished_at,
                        vods_new,
                        vods_updated,
                        status,
                    } => {
                        let _ = sink_handle.emit(
                            EV_POLL_FINISHED,
                            PollFinishedEvent {
                                twitch_user_id,
                                finished_at,
                                vods_new,
                                vods_updated,
                                status,
                            },
                        );
                    }
                    PollerEvent::Ingest(IngestEvent::VodIngested {
                        twitch_video_id,
                        twitch_user_id,
                        ingest_status,
                        stream_started_at,
                    }) => {
                        let _ = sink_handle.emit(
                            EV_VOD_INGESTED,
                            VodIngestedEvent {
                                twitch_video_id: twitch_video_id.clone(),
                                twitch_user_id: twitch_user_id.clone(),
                                ingest_status,
                                stream_started_at,
                            },
                        );
                        // Phase 4: keep the timeline index in sync.
                        let timeline = sink_timeline.clone();
                        tokio::spawn(async move {
                            // Resolve duration from vods row. We use a
                            // separate query rather than threading the
                            // value through IngestEvent so the event
                            // surface stays narrow. Any error is
                            // swallowed — the timeline-rebuild command
                            // is the user-facing recovery path.
                            if let Ok(row) = sqlx::query_scalar::<_, i64>(
                                "SELECT duration_seconds FROM vods WHERE twitch_video_id = ?",
                            )
                            .bind(&twitch_video_id)
                            .fetch_one(timeline.pool())
                            .await
                            {
                                let _ = timeline
                                    .upsert_from_vod(
                                        &twitch_video_id,
                                        &twitch_user_id,
                                        stream_started_at,
                                        row,
                                    )
                                    .await;
                            }
                        });
                    }
                    PollerEvent::Ingest(IngestEvent::VodUpdated {
                        twitch_video_id,
                        ingest_status,
                    }) => {
                        let _ = sink_handle.emit(
                            EV_VOD_UPDATED,
                            VodUpdatedEvent {
                                twitch_video_id,
                                ingest_status,
                            },
                        );
                    }
                });

                let spawn = poller_svc.spawn(sink);

                // Download event sink: fan out to Tauri topics.
                let download_sink_handle = handle.clone();
                let download_sink: DownloadEventSink =
                    Arc::new(move |ev: DownloadEvent| match ev {
                        DownloadEvent::StateChanged { vod_id, state } => {
                            let _ = download_sink_handle.emit(
                                EV_DOWNLOAD_STATE_CHANGED,
                                DownloadStateChangedEvent {
                                    vod_id,
                                    state: state.as_db_str().to_owned(),
                                },
                            );
                        }
                        DownloadEvent::Progress { vod_id, progress } => {
                            let _ = download_sink_handle.emit(
                                EV_DOWNLOAD_PROGRESS,
                                DownloadProgressEvent {
                                    vod_id,
                                    progress: progress.progress,
                                    bytes_done: progress.bytes_done as i64,
                                    bytes_total: progress.bytes_total.map(|n| n as i64),
                                    speed_bps: progress.speed_bps.map(|n| n as i64),
                                    eta_seconds: progress.eta_seconds.map(|n| n as i64),
                                },
                            );
                        }
                        DownloadEvent::Completed { vod_id, final_path } => {
                            let _ = download_sink_handle.emit(
                                EV_DOWNLOAD_COMPLETED,
                                DownloadCompletedEvent {
                                    vod_id,
                                    final_path: final_path.display().to_string(),
                                },
                            );
                        }
                        DownloadEvent::Failed { vod_id, reason } => {
                            let _ = download_sink_handle
                                .emit(EV_DOWNLOAD_FAILED, DownloadFailedEvent { vod_id, reason });
                        }
                    });
                let downloads_spawn = downloads_svc.clone().spawn(download_sink);

                let lib_sink_handle = handle.clone();
                let library_migration_sink: MigrationSink =
                    Arc::new(move |ev: LibraryMigrationEvent| match ev {
                        LibraryMigrationEvent::Migrating { id, moved, total } => {
                            let _ = lib_sink_handle.emit(
                                EV_LIBRARY_MIGRATING,
                                LibraryMigratingEvent {
                                    migration_id: id,
                                    moved,
                                    total,
                                },
                            );
                        }
                        LibraryMigrationEvent::Completed { id, moved, errors } => {
                            let _ = lib_sink_handle.emit(
                                EV_LIBRARY_MIGRATION_COMPLETED,
                                LibraryMigrationCompletedEvent {
                                    migration_id: id,
                                    moved,
                                    errors,
                                },
                            );
                        }
                        LibraryMigrationEvent::Failed { id, reason } => {
                            let _ = lib_sink_handle.emit(
                                EV_LIBRARY_MIGRATION_FAILED,
                                LibraryMigrationFailedEvent {
                                    migration_id: id,
                                    reason,
                                },
                            );
                        }
                    });

                handle.manage(AppState {
                    started_at,
                    db,
                    credentials: credentials_svc,
                    streamers: streamers_svc,
                    settings: settings_svc,
                    vods: vods_svc,
                    poller_handle: spawn.handle,
                    downloads: downloads_svc,
                    downloads_handle: downloads_spawn.handle,
                    library_migrator,
                    library_migration_sink,
                    storage: storage_svc,
                    timeline: timeline_svc,
                    shortcuts: shortcuts_svc,
                    notifications: notifications_svc,
                    media_assets: media_assets_svc,
                    close_behavior,
                    app_handle: handle.clone(),
                });

                // Install the tray icon + menu. Failure is non-fatal
                // (headless CI, Linux without StatusNotifierItem), but
                // we log it so it shows up in the support bundle.
                if let Err(e) = install_tray(&handle) {
                    warn!(error = %e, "tray icon install skipped");
                }

                // Phase 5 housekeeping: reconcile the autostart
                // setting against the OS. If the user enabled "Start
                // at login" in a previous session but disabled it in
                // System Settings, we pick up the OS state and update
                // the DB; if the DB says on and the OS says off, we
                // re-register. This runs after `manage` so the
                // reconcile's SettingsService handle is identical to
                // what `cmd_set_autostart` will later see.
                let settings_for_reconcile =
                    handle.state::<AppState>().settings.clone();
                let autostart_svc = crate::services::autostart::AutostartService::new(
                    handle.clone(),
                    settings_for_reconcile,
                );
                tokio::spawn(async move {
                    if let Err(e) = autostart_svc.reconcile().await {
                        warn!(error = ?e, "autostart reconcile failed");
                    }
                });

                handle
                    .emit("app:ready", serde_json::json!({ "startedAt": started_at }))
                    .map_err(|e| error::AppError::Io {
                        detail: e.to_string(),
                    })?;
                info!(started_at, "sightline is ready");
                Ok::<_, error::AppError>(())
            })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Phase 4: default close button → hide window. Tokio
                // services keep running. Explicit Quit (tray menu,
                // Cmd/Ctrl+Q, File > Quit → cmd_request_shutdown) is
                // what actually stops the poller/queue and exits the
                // process.
                //
                // SECURITY: read the behavior from a synchronous atomic
                // mirror rather than `block_on`-ing the async settings
                // service — a `block_on` inside a Tokio worker thread
                // deadlocks the multi-threaded scheduler and prevents
                // the graceful-shutdown path below from running. The
                // atomic is seeded at startup and kept in sync by
                // `cmd_set_window_close_behavior`.
                let behavior = match window.try_state::<AppState>() {
                    Some(state) => decode_close_behavior(state.close_behavior.load(Ordering::Acquire)),
                    None => crate::services::settings::WindowCloseBehavior::Hide,
                };
                match behavior {
                    crate::services::settings::WindowCloseBehavior::Hide => {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                    crate::services::settings::WindowCloseBehavior::Quit => {
                        if let Some(state) = window.try_state::<AppState>() {
                            state.poller_handle.shutdown();
                            state.downloads_handle.shutdown();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!("fatal: tauri runtime exited with error: {e}");
            std::process::exit(1);
        });
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt().with_env_filter(filter).with_target(false).finish();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        warn!("tracing subscriber already initialized");
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Install the platform-native tray icon. Chooses the right icon file
/// for each platform (macOS → template PNG, so it follows menu-bar
/// theme; Linux → 22×22 colour; Windows → 32×32 colour) and falls back
/// to the generic `icon.png` if the expected resource isn't bundled.
fn install_tray(handle: &tauri::AppHandle) -> Result<(), String> {
    use tauri::path::BaseDirectory;
    let is_macos = cfg!(target_os = "macos");
    // On macOS we want a template PNG so the OS can invert it in dark
    // mode; on Linux/Windows we want a colour PNG at a size the system
    // tray renders without blurring.
    let preferred = if is_macos {
        &["icons/tray-template.png", "icons/icon.png"][..]
    } else if cfg!(target_os = "linux") {
        &["icons/tray-22.png", "icons/icon.png"][..]
    } else {
        &["icons/tray-32.png", "icons/icon.png"][..]
    };
    let mut resolved: Option<std::path::PathBuf> = None;
    for candidate in preferred {
        if let Ok(path) = handle.path().resolve(candidate, BaseDirectory::Resource)
            && path.exists()
        {
            resolved = Some(path);
            break;
        }
        // Dev fallback (not yet bundled as resources).
        let repo = std::path::PathBuf::from("src-tauri").join(candidate);
        if repo.exists() {
            resolved = Some(repo);
            break;
        }
    }
    let path = resolved.ok_or_else(|| "no tray icon resource found".to_owned())?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read tray icon {path:?}: {e}"))?;
    crate::services::tray::install(handle, &bytes, is_macos)
        .map_err(|e| format!("tray install: {e}"))
}

/// Resolve a bundled sidecar binary by name. Returns `None` if Tauri's
/// resolver can't find it — callers fall back to the binary on PATH
/// (dev workflow) or surface a Sidecar error to the user.
///
/// Tauri's `bundle.externalBin` convention places binaries at
/// `binaries/<name>-<target-triple>[.exe]`. `TARGET_TRIPLE` is baked in
/// at compile time via `build.rs`, matching the filenames produced by
/// `scripts/bundle-sidecars.sh` (ADR-0013).
fn resolve_sidecar(handle: &tauri::AppHandle, name: &str) -> Option<std::path::PathBuf> {
    use tauri::path::BaseDirectory;
    let triple = env!("TARGET_TRIPLE");
    let ext = if triple.contains("windows") {
        ".exe"
    } else {
        ""
    };
    // Try the canonical bundled path first, then the repo's `src-tauri/binaries`
    // (covers `pnpm tauri dev` before a `tauri build` has copied resources).
    let candidates = [
        format!("binaries/{name}-{triple}{ext}"),
        format!("binaries/{name}{ext}"),
        format!("binaries/{name}"),
    ];
    for candidate in &candidates {
        if let Ok(path) = handle.path().resolve(candidate, BaseDirectory::Resource)
            && path.exists()
        {
            return Some(path);
        }
    }
    // Dev fallback: look relative to the repo.
    let repo_candidate =
        std::path::PathBuf::from("src-tauri/binaries").join(format!("{name}-{triple}{ext}"));
    if repo_candidate.exists() {
        return Some(repo_candidate);
    }
    None
}

/// Resolve the SQLite file path. In Phase 1 we keep it simple and place
/// the file under the OS-native app-data directory. Phase 4 moves this to
/// a user-configurable `library_root`.
fn resolve_db_path(handle: &tauri::AppHandle) -> Result<std::path::PathBuf, error::AppError> {
    let dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| error::AppError::Io {
            detail: format!("resolve app_data_dir: {e}"),
        })?;
    std::fs::create_dir_all(&dir).map_err(|e| error::AppError::Io {
        detail: format!("create app_data_dir: {e}"),
    })?;
    Ok(dir.join("sightline.sqlite"))
}

pub type SharedState = Arc<AppState>;
