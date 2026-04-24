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
use crate::services::events::{
    CredentialsChangedEvent, EV_CREDENTIALS_CHANGED, EV_POLL_FINISHED, EV_POLL_STARTED,
    EV_STREAMER_ADDED, EV_STREAMER_REMOVED, EV_VOD_INGESTED, EV_VOD_UPDATED, PollFinishedEvent,
    PollStartedEvent, StreamerAddedEvent, StreamerRemovedEvent, VodIngestedEvent, VodUpdatedEvent,
};
use crate::services::ingest::{IngestEvent, IngestService};
use crate::services::poller::{PollerEvent, PollerHandle, PollerService};
use crate::services::settings::SettingsService;
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
                handle.manage(AppState {
                    started_at,
                    db,
                    credentials: credentials_svc,
                    streamers: streamers_svc,
                    settings: settings_svc,
                    vods: vods_svc,
                    poller_handle: spawn.handle,
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
