//! Download-queue commands. Thin wrappers over
//! [`DownloadQueueService`].

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::downloads::{DownloadFilters, DownloadRow};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueDownloadInput {
    pub vod_id: String,
    #[specta(optional)]
    pub priority: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VodIdInput {
    pub vod_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReprioritizeInput {
    pub vod_id: String,
    pub priority: i64,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ListDownloadsInput {
    #[specta(optional)]
    pub filters: Option<DownloadFilters>,
}

#[tauri::command]
#[specta::specta]
pub async fn enqueue_download(
    state: tauri::State<'_, AppState>,
    input: EnqueueDownloadInput,
) -> Result<DownloadRow, AppError> {
    let row = state
        .downloads
        .enqueue(&input.vod_id, input.priority)
        .await?;
    state.downloads_handle.wake_up().await;
    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub async fn pause_download(
    state: tauri::State<'_, AppState>,
    input: VodIdInput,
) -> Result<DownloadRow, AppError> {
    state.downloads.pause(&input.vod_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_download(
    state: tauri::State<'_, AppState>,
    input: VodIdInput,
) -> Result<DownloadRow, AppError> {
    let row = state.downloads.resume(&input.vod_id).await?;
    state.downloads_handle.wake_up().await;
    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_download(
    state: tauri::State<'_, AppState>,
    input: VodIdInput,
) -> Result<DownloadRow, AppError> {
    state.downloads.cancel(&input.vod_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn retry_download(
    state: tauri::State<'_, AppState>,
    input: VodIdInput,
) -> Result<DownloadRow, AppError> {
    let row = state.downloads.retry(&input.vod_id).await?;
    state.downloads_handle.wake_up().await;
    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub async fn reprioritize_download(
    state: tauri::State<'_, AppState>,
    input: ReprioritizeInput,
) -> Result<DownloadRow, AppError> {
    let row = state
        .downloads
        .reprioritize(&input.vod_id, input.priority)
        .await?;
    state.downloads_handle.wake_up().await;
    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub async fn list_downloads(
    state: tauri::State<'_, AppState>,
    input: ListDownloadsInput,
) -> Result<Vec<DownloadRow>, AppError> {
    state
        .downloads
        .list(&input.filters.unwrap_or_default())
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_download(
    state: tauri::State<'_, AppState>,
    input: VodIdInput,
) -> Result<DownloadRow, AppError> {
    state
        .downloads
        .get(&input.vod_id)
        .await?
        .ok_or(AppError::NotFound)
}
