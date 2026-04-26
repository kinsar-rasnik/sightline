//! Quality-pipeline IPC commands (Phase 8, ADR-0028).
//!
//! Three thin wrappers over [`crate::services::encoder_detection`] and
//! [`crate::services::settings`].  All commands return
//! `Result<T, AppError>` so the renderer narrows on `kind` like every
//! other Phase-8 surface.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::quality::{EncoderCapability, VideoQualityProfile};
use crate::error::AppError;
use crate::services::settings::SettingsPatch;

/// Persisted encoder-capability snapshot (or `None` if detection has
/// never run).  Returns the raw `EncoderCapability` blob — the
/// renderer renders the per-encoder-kind labels.
#[tauri::command]
#[specta::specta]
pub async fn get_encoder_capability(
    state: tauri::State<'_, AppState>,
) -> Result<Option<EncoderCapability>, AppError> {
    Ok(state.settings.get().await?.encoder_capability)
}

/// Force a fresh detection probe (`ffmpeg -encoders` + 2-second test
/// encode).  Persists the new capability and returns it.  Triggered
/// by the Settings UI's "Re-detect" button.
#[tauri::command]
#[specta::specta]
pub async fn redetect_encoders(
    state: tauri::State<'_, AppState>,
) -> Result<EncoderCapability, AppError> {
    state.encoder_detection.detect_and_persist().await
}

/// Persist the chosen quality profile.  Wraps `update_settings` for
/// the typed-from-the-renderer "user picked a profile" gesture; the
/// generic `update_settings` command also accepts the field through
/// `SettingsPatch.video_quality_profile`, but a dedicated command
/// keeps the IPC surface readable on the Settings UI side.
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetVideoQualityProfileInput {
    pub profile: VideoQualityProfile,
}

#[tauri::command]
#[specta::specta]
pub async fn set_video_quality_profile(
    state: tauri::State<'_, AppState>,
    input: SetVideoQualityProfileInput,
) -> Result<VideoQualityProfile, AppError> {
    let updated = state
        .settings
        .update(SettingsPatch {
            video_quality_profile: Some(input.profile),
            ..Default::default()
        })
        .await?;
    Ok(updated.video_quality_profile)
}
