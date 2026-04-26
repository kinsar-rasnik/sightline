//! Pull-on-demand distribution IPC commands (Phase 8, ADR-0030 + ADR-0031).
//!
//! Six thin wrappers over [`crate::services::distribution`].  The
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

/// Input for [`prefetch_check`].  The currently-watching VOD ID is
/// the only signal — `prefetch_check` derives the streamer + the
/// chronologically-next `available` candidate from the database.
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchCheckInput {
    pub vod_id: String,
}

/// Outcome of a `prefetch_check`.  `triggered = true` means a
/// candidate was transitioned `available -> queued`; `prefetched_vod_id`
/// carries the VOD ID for the renderer's optimistic update.  Both
/// fields are `false` / `None` when the check no-ops (settings,
/// auto mode, full window, no candidate, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchCheckResult {
    pub triggered: bool,
    pub prefetched_vod_id: Option<String>,
}

/// ADR-0031 hook: invoked from the player when the active VOD's
/// watch progress crosses the threshold or the remaining time falls
/// below the look-ahead window.  The frontend hook
/// (`src/features/player/use-prefetch-hook.ts`) maintains a
/// module-scoped `Set<vodId>` so this command fires at most once
/// per VOD per app session — re-mounting the player on the same
/// VOD does not re-trigger.  Returns whether a pre-fetch actually
/// fired so the renderer can show a non-blocking confirmation.
#[tauri::command]
#[specta::specta]
pub async fn prefetch_check(
    state: tauri::State<'_, AppState>,
    input: PrefetchCheckInput,
) -> Result<PrefetchCheckResult, AppError> {
    let pick = state
        .distribution
        .prefetch_check(&input.vod_id, &state.distribution_sink)
        .await?;
    Ok(PrefetchCheckResult {
        triggered: pick.is_some(),
        prefetched_vod_id: pick,
    })
}
