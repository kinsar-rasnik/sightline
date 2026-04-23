//! Sightline — library root.
//!
//! Layering: `commands` (thin IPC surface) → `services` (orchestration)
//! → `domain` (pure types) / `infra` (DB, HTTP, sidecars). No back-edges.

pub mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, Manager};
use tracing::{info, warn};

/// Shared application state.
///
/// One instance is constructed during setup and cloned (via `Arc`) into
/// every command handler via Tauri's state management.
pub struct AppState {
    /// Unix seconds UTC at which the process started.
    pub started_at: i64,
    /// SQLite pool, initialized after migrations run.
    pub db: infra::db::Db,
}

/// Entry point called by `main.rs`. Separate to make the app testable from
/// integration tests without invoking the GUI.
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let started_at = unix_now();
            let handle = app.handle().clone();

            // Initialize the DB on the async runtime that Tauri created.
            tauri::async_runtime::block_on(async move {
                let db_path = resolve_db_path(&handle)?;
                let db = infra::db::Db::open(&db_path).await?;
                db.migrate().await?;
                handle.manage(AppState { started_at, db });
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
        .invoke_handler(tauri::generate_handler![commands::health::health])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fatal: tauri runtime exited with error: {e}");
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

// Re-export for integration tests.
pub use infra::db::Db;

pub type SharedState = Arc<AppState>;
