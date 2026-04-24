//! Autostart commands (Phase 5 housekeeping).
//!
//! Persists `start_at_login` in the DB and applies the change to the
//! OS (LaunchAgent / Run registry key / XDG autostart) via the
//! autostart service. The frontend Settings page gets `os_enabled`
//! back so it can surface any divergence caused by the user toggling
//! the setting outside Sightline.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::warn;

use crate::AppState;
use crate::error::AppError;
use crate::services::autostart::{AutostartService, AutostartStatus};
use crate::services::settings::SettingsPatch;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetAutostartInput {
    pub enabled: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn get_autostart_status(
    state: tauri::State<'_, AppState>,
) -> Result<AutostartStatus, AppError> {
    let svc = autostart_service(&state);
    let os_enabled = svc.is_os_enabled().await.unwrap_or_else(|e| {
        warn!(error = ?e, "autostart probe failed; reporting false");
        false
    });
    let db_enabled = state.settings.get().await?.start_at_login;
    Ok(AutostartStatus {
        os_enabled,
        db_enabled,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn set_autostart(
    state: tauri::State<'_, AppState>,
    input: SetAutostartInput,
) -> Result<AutostartStatus, AppError> {
    let svc = autostart_service(&state);
    let os_enabled = svc.set_os_enabled(input.enabled).await?;
    state
        .settings
        .update(SettingsPatch {
            start_at_login: Some(os_enabled),
            ..Default::default()
        })
        .await?;
    Ok(AutostartStatus {
        os_enabled,
        db_enabled: os_enabled,
    })
}

fn autostart_service(state: &AppState) -> AutostartService<tauri::Wry> {
    AutostartService::new(state.app_handle.clone(), Arc::clone(&state.settings))
}
