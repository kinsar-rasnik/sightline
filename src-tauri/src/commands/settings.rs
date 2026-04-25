//! Settings commands. `update_settings` accepts a partial patch; `get`
//! always returns the full `AppSettings` including the credentials
//! status summary.

use tauri::Manager;

use crate::AppState;
use crate::error::AppError;
use crate::services::settings::{AppSettings, SettingsPatch};

#[tauri::command]
#[specta::specta]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, AppError> {
    state.settings.get().await
}

#[tauri::command]
#[specta::specta]
pub async fn update_settings(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    patch: SettingsPatch,
) -> Result<AppSettings, AppError> {
    let drift_invalidate = patch.sync_drift_threshold_ms.is_some();
    let library_root_added = patch.library_root.clone();
    let result = state.settings.update(patch).await?;
    if drift_invalidate {
        // Phase 7 pickup: keep the SyncService drift cache in sync with
        // a fresh settings write so a slider change reflects on the
        // next tick instead of waiting out the cache TTL.
        state.sync.invalidate_drift_cache();
    }
    if let Some(root) = library_root_added
        && !root.is_empty()
        && let Err(e) = app.asset_protocol_scope().allow_directory(&root, true)
    {
        // Phase 7 (ADR-0027): extend the asset-protocol allow-list to
        // cover the new library root so the player + grid can serve
        // files from it without restarting the app.  We never remove
        // a previously-allowed root mid-session; a window reload
        // would be a cleaner reset and is the documented escape hatch.
        tracing::warn!(
            error = %e,
            root = %root,
            "asset protocol allow_directory failed on update"
        );
    }
    Ok(result)
}
