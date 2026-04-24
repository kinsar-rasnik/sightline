//! Streamer-management commands.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::streamer::StreamerSummary;
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddStreamerInput {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveStreamerInput {
    pub twitch_user_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn add_streamer(
    state: tauri::State<'_, AppState>,
    input: AddStreamerInput,
) -> Result<StreamerSummary, AppError> {
    let summary = state.streamers.add(&input.login).await?;
    state.emit_streamer_added(&summary.streamer.twitch_user_id, &summary.streamer.login);
    Ok(summary)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_streamer(
    state: tauri::State<'_, AppState>,
    input: RemoveStreamerInput,
) -> Result<(), AppError> {
    state.streamers.remove(&input.twitch_user_id).await?;
    state.emit_streamer_removed(&input.twitch_user_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn list_streamers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StreamerSummary>, AppError> {
    state.streamers.list_active().await
}
