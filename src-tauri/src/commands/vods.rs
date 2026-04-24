//! VOD read commands.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::vods::{ListVodsInput, VodWithChapters};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetVodInput {
    pub twitch_video_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn list_vods(
    state: tauri::State<'_, AppState>,
    input: ListVodsInput,
) -> Result<Vec<VodWithChapters>, AppError> {
    state.vods.list(&input).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_vod(
    state: tauri::State<'_, AppState>,
    input: GetVodInput,
) -> Result<VodWithChapters, AppError> {
    state.vods.get(&input.twitch_video_id).await
}
