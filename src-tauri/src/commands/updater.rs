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
/// "View release" button on the update banner.  Defence-in-depth:
/// the URL is parsed via [`url::Url::parse`] so a string like
/// `https://\x00evil.com` (which would pass a naive prefix check
/// but truncate at the OS-layer `open` call) is rejected.  We also
/// enforce a `github.com` host allow-list since the only sanctioned
/// caller is the updater service whose data flows from
/// `https://api.github.com/repos/.../releases/latest`.
#[tauri::command]
#[specta::specta]
pub async fn open_release_url(input: OpenReleaseUrlInput) -> Result<(), AppError> {
    let parsed = url::Url::parse(&input.url).map_err(|e| AppError::InvalidInput {
        detail: format!("release URL parse: {e}"),
    })?;
    if parsed.scheme() != "https" {
        return Err(AppError::InvalidInput {
            detail: "release URL must be https".into(),
        });
    }
    let host = parsed.host_str().unwrap_or("");
    if !(host == "github.com" || host.ends_with(".github.com")) {
        return Err(AppError::InvalidInput {
            detail: format!("release URL must be on github.com (got {host})"),
        });
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AppError::InvalidInput {
            detail: "release URL must not carry credentials".into(),
        });
    }
    opener::open(parsed.as_str()).map_err(|e| AppError::Io {
        detail: format!("opener: {e}"),
    })
}
