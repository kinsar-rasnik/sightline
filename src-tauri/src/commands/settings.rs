//! Settings commands. `update_settings` accepts a partial patch; `get`
//! always returns the full `AppSettings` including the credentials
//! status summary.

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
    patch: SettingsPatch,
) -> Result<AppSettings, AppError> {
    state.settings.update(patch).await
}
