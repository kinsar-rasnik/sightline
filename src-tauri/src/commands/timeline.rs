//! Timeline commands — Phase 4.
//!
//! All handlers are ≤20 lines; the heavy lifting lives in
//! `services::timeline_indexer`. Rebuild progress is fan-out via the
//! tray-aware AppHandle; list/stats/co-streams are synchronous queries.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter;

use crate::AppState;
use crate::domain::timeline::{CoStream, Interval};
use crate::error::AppError;
use crate::services::events::{
    EV_TIMELINE_INDEX_REBUILDING, EV_TIMELINE_INDEX_REBUILT, TimelineIndexRebuildingEvent,
    TimelineIndexRebuiltEvent,
};
use crate::services::timeline_indexer::{
    IndexerEvent, IndexerEventSink, TimelineFilters, TimelineStats,
};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ListTimelineInput {
    #[specta(optional)]
    pub filters: Option<TimelineFilters>,
}

#[tauri::command]
#[specta::specta]
pub async fn list_timeline(
    state: tauri::State<'_, AppState>,
    input: ListTimelineInput,
) -> Result<Vec<Interval>, AppError> {
    state.timeline.list(input.filters.unwrap_or_default()).await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetCoStreamsInput {
    pub vod_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn get_co_streams(
    state: tauri::State<'_, AppState>,
    input: GetCoStreamsInput,
) -> Result<Vec<CoStream>, AppError> {
    state.timeline.co_streams_of(&input.vod_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_timeline_stats(
    state: tauri::State<'_, AppState>,
) -> Result<TimelineStats, AppError> {
    state.timeline.stats().await
}

#[tauri::command]
#[specta::specta]
pub async fn rebuild_timeline_index(
    state: tauri::State<'_, AppState>,
) -> Result<TimelineStats, AppError> {
    let handle = state.app_handle.clone();
    let sink: IndexerEventSink = Arc::new(move |ev| match ev {
        IndexerEvent::Rebuilding { processed, total } => {
            let progress = if total == 0 {
                1.0
            } else {
                (processed as f64) / (total as f64)
            };
            let _ = handle.emit(
                EV_TIMELINE_INDEX_REBUILDING,
                TimelineIndexRebuildingEvent {
                    progress,
                    processed,
                    total,
                },
            );
        }
        IndexerEvent::Rebuilt { total } => {
            let _ = handle.emit(
                EV_TIMELINE_INDEX_REBUILT,
                TimelineIndexRebuiltEvent { total },
            );
        }
    });
    state.timeline.rebuild_all(sink).await?;
    state.timeline.stats().await
}
