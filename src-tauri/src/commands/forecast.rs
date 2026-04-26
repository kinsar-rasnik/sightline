//! Storage-forecast IPC commands (v2.0.1, ADR-0032).
//!
//! Two thin wrappers over [`crate::services::forecast`]: per-streamer
//! and global.  Both return rounded GB numbers + watermark-risk
//! indicators ready for the renderer.  Wall-clock reads happen
//! inside the service (via the injected `Clock`), keeping the
//! command thin and the service test-deterministic per the project
//! `rust-backend.md` rule.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::forecast::{ForecastResult, GlobalForecast};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EstimateStreamerFootprintInput {
    pub twitch_user_id: String,
}

/// Forecast a single streamer's storage footprint.  Used by the
/// Streamers → Add dialog to set expectations before the user
/// commits, and by the per-streamer breakdown in Settings →
/// Storage Outlook.
#[tauri::command]
#[specta::specta]
pub async fn estimate_streamer_footprint(
    state: tauri::State<'_, AppState>,
    input: EstimateStreamerFootprintInput,
) -> Result<ForecastResult, AppError> {
    state
        .forecast
        .estimate_streamer_footprint(&input.twitch_user_id)
        .await
}

/// Combined forecast across every active streamer + per-streamer
/// breakdown.  Drives the Settings → Storage Outlook section.
#[tauri::command]
#[specta::specta]
pub async fn estimate_global_footprint(
    state: tauri::State<'_, AppState>,
) -> Result<GlobalForecast, AppError> {
    state.forecast.estimate_global_footprint().await
}
