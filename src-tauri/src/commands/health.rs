//! `health` command — round-trip sanity check.

use crate::AppState;
use crate::error::AppError;
use crate::services::health::HealthService;

/// Returns an application health report.
///
/// Used by the frontend on startup to verify that the webview, command
/// bridge, and database are all alive.
#[tauri::command]
#[specta::specta]
pub async fn health(
    state: tauri::State<'_, AppState>,
) -> Result<crate::domain::health::HealthReport, AppError> {
    HealthService::new(&state.db).report(state.started_at).await
}
