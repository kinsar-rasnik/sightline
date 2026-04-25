//! Multi-view sync commands (Phase 6 / ADR-0021..0023).
//!
//! Thin wrappers over `services::sync::SyncService`.  Each handler
//! deserialises the input, fires the service method with the wired
//! event sink, and serialises the result.  Heavy lifting lives in
//! the service.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::sync::{
    DriftMeasurement, PaneIndex, SyncLayout, SyncSession, SyncSessionId, SyncTransportCommand,
};
use crate::error::AppError;
use crate::services::sync::OverlapResult;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenSyncGroupInput {
    pub vod_ids: Vec<String>,
    pub layout: SyncLayout,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSessionIdInput {
    pub session_id: SyncSessionId,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetSyncLeaderInput {
    pub session_id: SyncSessionId,
    pub pane_index: PaneIndex,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSeekInput {
    pub session_id: SyncSessionId,
    pub wall_clock_ts: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSetSpeedInput {
    pub session_id: SyncSessionId,
    pub speed: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetOverlapInput {
    pub vod_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordSyncDriftInput {
    pub session_id: SyncSessionId,
    pub measurement: DriftMeasurement,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportSyncOutOfRangeInput {
    pub session_id: SyncSessionId,
    pub pane_index: PaneIndex,
}

#[tauri::command]
#[specta::specta]
pub async fn open_sync_group(
    state: tauri::State<'_, AppState>,
    input: OpenSyncGroupInput,
) -> Result<SyncSession, AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .open_session(input.vod_ids, input.layout, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn close_sync_group(
    state: tauri::State<'_, AppState>,
    input: SyncSessionIdInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .close_session(input.session_id, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_sync_group(
    state: tauri::State<'_, AppState>,
    input: SyncSessionIdInput,
) -> Result<SyncSession, AppError> {
    state.sync.get_session(input.session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_sync_leader(
    state: tauri::State<'_, AppState>,
    input: SetSyncLeaderInput,
) -> Result<SyncSession, AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .set_leader(input.session_id, input.pane_index, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_seek(
    state: tauri::State<'_, AppState>,
    input: SyncSeekInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .apply_transport(
            input.session_id,
            SyncTransportCommand::Seek {
                wall_clock_ts: input.wall_clock_ts,
            },
            Some(&sink),
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_play(
    state: tauri::State<'_, AppState>,
    input: SyncSessionIdInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .apply_transport(input.session_id, SyncTransportCommand::Play, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_pause(
    state: tauri::State<'_, AppState>,
    input: SyncSessionIdInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .apply_transport(input.session_id, SyncTransportCommand::Pause, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_set_speed(
    state: tauri::State<'_, AppState>,
    input: SyncSetSpeedInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .apply_transport(
            input.session_id,
            SyncTransportCommand::SetSpeed { speed: input.speed },
            Some(&sink),
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_overlap(
    state: tauri::State<'_, AppState>,
    input: GetOverlapInput,
) -> Result<OverlapResult, AppError> {
    state.sync.overlap_of(input.vod_ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn record_sync_drift(
    state: tauri::State<'_, AppState>,
    input: RecordSyncDriftInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .record_drift(input.session_id, input.measurement, Some(&sink))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn report_sync_out_of_range(
    state: tauri::State<'_, AppState>,
    input: ReportSyncOutOfRangeInput,
) -> Result<(), AppError> {
    let sink = state.sync_sink.clone();
    state
        .sync
        .report_out_of_range(input.session_id, input.pane_index, Some(&sink))
        .await
}
