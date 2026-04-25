//! Update-checker IPC commands (Phase 7, ADR-0026).
//!
//! Three thin wrappers — manual force check, status read, skip-version
//! write — all returning `Result<T, AppError>`.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::updater::{UpdateInfo, UpdateStatus};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CheckForUpdateInput {
    /// Skip the once-per-24h gate.  The Settings UI's "Check now"
    /// button passes `true`; the scheduled tick uses `false`.
    pub force: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn check_for_update(
    state: tauri::State<'_, AppState>,
    input: CheckForUpdateInput,
) -> Result<Option<UpdateInfo>, AppError> {
    state.updater.check_for_update(input.force).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_update_status(
    state: tauri::State<'_, AppState>,
) -> Result<UpdateStatus, AppError> {
    state.updater.get_status().await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SkipUpdateVersionInput {
    /// Empty string clears any prior skip.
    pub version: String,
}

#[tauri::command]
#[specta::specta]
pub async fn skip_update_version(
    state: tauri::State<'_, AppState>,
    input: SkipUpdateVersionInput,
) -> Result<(), AppError> {
    state.updater.skip_version(input.version).await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenReleaseUrlInput {
    pub url: String,
}

/// Open an http(s) URL in the user's default browser.  Used by the
/// "View release" button on the update banner; any other URL is
/// rejected so a malicious release body can't trick the renderer
/// into spawning a `file:///` or `javascript:` URL.
#[tauri::command]
#[specta::specta]
pub async fn open_release_url(input: OpenReleaseUrlInput) -> Result<(), AppError> {
    let lowered = input.url.to_ascii_lowercase();
    if !lowered.starts_with("https://") {
        return Err(AppError::InvalidInput {
            detail: "release URL must be https".into(),
        });
    }
    opener::open(&input.url).map_err(|e| AppError::Io {
        detail: format!("opener: {e}"),
    })
}
