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
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, Manager};
use tracing::{error, info, warn};

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
use crate::services::poller::{PollerEvent, PollerHandle, PollerService};
use crate::services::settings::SettingsService;
use crate::services::storage::StorageService;
use crate::services::streamers::StreamerService;
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
    pub app_handle: tauri::AppHandle,
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
                    ffmpeg,
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

                // Event sink: dispatch each PollerEvent variant to the
                // matching Tauri topic. Keeping all event construction
                // in one closure makes it trivial to trace the surface
                // the webview actually sees.
                let sink_handle = handle.clone();
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
                                twitch_video_id,
                                twitch_user_id,
                                ingest_status,
                                stream_started_at,
                            },
                        );
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
                    app_handle: handle.clone(),
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
            if let tauri::WindowEvent::CloseRequested { .. } = event
                && let Some(state) = window.try_state::<AppState>()
            {
                state.poller_handle.shutdown();
                state.downloads_handle.shutdown();
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
