//! Auto-cleanup IPC commands (Phase 7, ADR-0024).
//!
//! Thin wrappers over `services::cleanup::CleanupService`.  All four
//! commands return Result<T, AppError> so the renderer narrows on
//! `kind` like every other Phase-7 surface.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::cleanup::{CleanupLogEntry, CleanupMode, CleanupPlan, CleanupResult};
use crate::error::AppError;
use crate::services::cleanup::DiskUsage;

#[tauri::command]
#[specta::specta]
pub async fn get_cleanup_plan(state: tauri::State<'_, AppState>) -> Result<CleanupPlan, AppError> {
    state.cleanup.compute_plan().await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCleanupInput {
    pub dry_run: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn execute_cleanup(
    state: tauri::State<'_, AppState>,
    input: ExecuteCleanupInput,
) -> Result<CleanupResult, AppError> {
    let plan = state.cleanup.compute_plan().await?;
    let mode = if input.dry_run {
        CleanupMode::DryRun
    } else {
        CleanupMode::Manual
    };
    state.cleanup.execute_plan(plan, mode).await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupHistoryInput {
    /// Maximum entries to return.  Clamped to `[1, 200]` server-side.
    /// Default 25.
    #[specta(optional)]
    pub limit: Option<i64>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_cleanup_history(
    state: tauri::State<'_, AppState>,
    input: CleanupHistoryInput,
) -> Result<Vec<CleanupLogEntry>, AppError> {
    state.cleanup.list_history(input.limit.unwrap_or(25)).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_disk_usage(state: tauri::State<'_, AppState>) -> Result<DiskUsage, AppError> {
    state.cleanup.get_disk_usage().await
}
