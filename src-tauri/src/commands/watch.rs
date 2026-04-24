//! Watch-progress commands (Phase 5).

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::watch_progress::ProgressSettings;
use crate::error::AppError;
use crate::services::watch_progress::{ContinueWatchingEntry, WatchProgressRow, WatchStats};

/// Distinct from `crate::commands::downloads::VodIdInput` (downloads)
/// and `crate::commands::media::VodAssetsInput` so tauri-specta
/// doesn't collide on the type-name when emitting the bindings. The
/// wire shape is identical.
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WatchVodIdInput {
    pub vod_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWatchProgressInput {
    pub vod_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ListContinueWatchingInput {
    #[specta(optional)]
    pub limit: Option<i64>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct GetWatchStatsInput {
    #[specta(optional)]
    pub streamer_id: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_watch_progress(
    state: tauri::State<'_, AppState>,
    input: WatchVodIdInput,
) -> Result<Option<WatchProgressRow>, AppError> {
    state.watch_progress.get(&input.vod_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_watch_progress(
    state: tauri::State<'_, AppState>,
    input: UpdateWatchProgressInput,
) -> Result<WatchProgressRow, AppError> {
    let sink = state.watch_progress_sink.clone();
    // We derive ProgressSettings from the stored AppSettings in the
    // caller's session; for now use the default because the settings
    // row doesn't yet carry the per-user completion threshold (that
    // lands with the Settings UI work). Mission spec makes this a
    // 70–100 % configurable range, which the default of 0.9 satisfies
    // out-of-the-box.
    state
        .watch_progress
        .update(
            &input.vod_id,
            input.position_seconds,
            input.duration_seconds,
            ProgressSettings::default(),
            Some(&sink),
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn mark_watched(
    state: tauri::State<'_, AppState>,
    input: WatchVodIdInput,
) -> Result<WatchProgressRow, AppError> {
    let sink = state.watch_progress_sink.clone();
    // We need the duration — fall back to the stored row's value or
    // 0 if the user hasn't played this VOD yet (unusual, but possible
    // if they click "Mark watched" from the grid without opening it).
    let duration = state
        .watch_progress
        .get(&input.vod_id)
        .await?
        .map(|p| p.duration_seconds)
        .unwrap_or(0.0);
    state
        .watch_progress
        .mark_watched(&input.vod_id, duration, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn mark_unwatched(
    state: tauri::State<'_, AppState>,
    input: WatchVodIdInput,
) -> Result<WatchProgressRow, AppError> {
    let sink = state.watch_progress_sink.clone();
    state
        .watch_progress
        .mark_unwatched(&input.vod_id, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_continue_watching(
    state: tauri::State<'_, AppState>,
    input: ListContinueWatchingInput,
) -> Result<Vec<ContinueWatchingEntry>, AppError> {
    state
        .watch_progress
        .list_continue_watching(input.limit.unwrap_or(12))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_watch_stats(
    state: tauri::State<'_, AppState>,
    input: GetWatchStatsInput,
) -> Result<WatchStats, AppError> {
    state
        .watch_progress
        .stats(input.streamer_id.as_deref())
        .await
}
