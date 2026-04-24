//! Media-asset commands (Phase 5 housekeeping + player).
//!
//! The webview reads VOD thumbnails, hover-preview frames, and the
//! video file itself through these handlers. Every path returned has
//! been verified to sit under the configured library root — the
//! security-reviewer flagged path validation as the primary risk on
//! the asset-protocol surface (ADR-0019).

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::media_assets::{VideoSource, VodAssets};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VodAssetsInput {
    pub vod_id: String,
}

/// Fetch the asset bundle for a single VOD. Returns a struct with
/// every `Option<String>` field set to `None` if the VOD has no
/// completed download — that's the expected state for a VOD the user
/// hasn't enqueued yet, not an error.
#[tauri::command]
#[specta::specta]
pub async fn get_vod_assets(
    state: tauri::State<'_, AppState>,
    input: VodAssetsInput,
) -> Result<VodAssets, AppError> {
    state.media_assets.get(&input.vod_id).await
}

/// Force-regenerate the single-frame thumbnail for a VOD. Useful for
/// pre-Phase-5 rows that were downloaded before the preview pipeline
/// existed and the thumbnail is missing. Returns when ffmpeg exits
/// (success or failure — failure surfaces as `AppError::Sidecar`).
#[tauri::command]
#[specta::specta]
pub async fn regenerate_vod_thumbnail(
    state: tauri::State<'_, AppState>,
    input: VodAssetsInput,
) -> Result<(), AppError> {
    state.media_assets.regenerate_thumbnail(&input.vod_id).await
}

/// Return a `VideoSource` narrowed to `ready | missing | partial`.
/// The player uses this as its single choke point — the renderer
/// never builds a filesystem path directly.
#[tauri::command]
#[specta::specta]
pub async fn get_video_source(
    state: tauri::State<'_, AppState>,
    input: VodAssetsInput,
) -> Result<VideoSource, AppError> {
    state.media_assets.get_video_source(&input.vod_id).await
}

/// Remux the downloaded `.mp4` in-place via ffmpeg. The player's
/// "Remux file" recovery action fires this when the `<video>`
/// element can't decode the downloaded file.
#[tauri::command]
#[specta::specta]
pub async fn request_remux(
    state: tauri::State<'_, AppState>,
    input: VodAssetsInput,
) -> Result<(), AppError> {
    state.media_assets.request_remux(&input.vod_id).await
}
