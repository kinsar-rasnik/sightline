//! Pull-on-demand distribution IPC commands (Phase 8, ADR-0030 + ADR-0031).
//!
//! Five thin wrappers over [`crate::services::distribution`].  The
//! event emission happens at the AppState boundary in `lib.rs`.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::distribution::DistributionMode;
use crate::error::AppError;
use crate::services::distribution::PickResult;
use crate::services::settings::SettingsPatch;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PickVodInput {
    pub vod_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn pick_vod(
    state: tauri::State<'_, AppState>,
    input: PickVodInput,
) -> Result<PickResult, AppError> {
    state
        .distribution
        .pick_vod(&input.vod_id, &state.distribution_sink)
        .await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PickNextNInput {
    pub streamer_id: String,
    pub n: i64,
}

#[tauri::command]
#[specta::specta]
pub async fn pick_next_n(
    state: tauri::State<'_, AppState>,
    input: PickNextNInput,
) -> Result<Vec<String>, AppError> {
    state
        .distribution
        .pick_next_n(&input.streamer_id, input.n, &state.distribution_sink)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn unpick_vod(
    state: tauri::State<'_, AppState>,
    input: PickVodInput,
) -> Result<PickResult, AppError> {
    state
        .distribution
        .unpick_vod(&input.vod_id, &state.distribution_sink)
        .await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetDistributionModeInput {
    pub mode: DistributionMode,
}

#[tauri::command]
#[specta::specta]
pub async fn set_distribution_mode(
    state: tauri::State<'_, AppState>,
    input: SetDistributionModeInput,
) -> Result<DistributionMode, AppError> {
    let updated = state
        .settings
        .update(SettingsPatch {
            distribution_mode: Some(input.mode),
            ..Default::default()
        })
        .await?;
    Ok(updated.distribution_mode)
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetSlidingWindowSizeInput {
    pub size: i64,
}

#[tauri::command]
#[specta::specta]
pub async fn set_sliding_window_size(
    state: tauri::State<'_, AppState>,
    input: SetSlidingWindowSizeInput,
) -> Result<i64, AppError> {
    let updated = state
        .settings
        .update(SettingsPatch {
            sliding_window_size: Some(input.size),
            ..Default::default()
        })
        .await?;
    Ok(updated.sliding_window_size)
}
