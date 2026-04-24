//! Poll-control commands.

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::AppState;
use crate::domain::streamer::StreamerSummary;
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TriggerPollInput {
    /// If omitted, the poller re-evaluates every due streamer on its next tick.
    pub twitch_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PollStatusRow {
    pub streamer: StreamerSummary,
    pub last_poll: Option<LastPollSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LastPollSummary {
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub vods_new: i64,
    pub vods_updated: i64,
    pub status: String,
}

#[tauri::command]
#[specta::specta]
pub async fn trigger_poll(
    state: tauri::State<'_, AppState>,
    input: TriggerPollInput,
) -> Result<(), AppError> {
    state.poller_handle.trigger(input.twitch_user_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_poll_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PollStatusRow>, AppError> {
    let streamers = state.streamers.list_active().await?;
    let mut out = Vec::with_capacity(streamers.len());
    for s in streamers {
        let last = last_poll_for(&state, &s.streamer.twitch_user_id).await?;
        out.push(PollStatusRow {
            streamer: s,
            last_poll: last,
        });
    }
    Ok(out)
}

async fn last_poll_for(
    state: &tauri::State<'_, AppState>,
    twitch_user_id: &str,
) -> Result<Option<LastPollSummary>, AppError> {
    let row = sqlx::query(
        "SELECT started_at, finished_at, vods_new, vods_updated, status
         FROM poll_log
         WHERE twitch_user_id = ?
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(twitch_user_id)
    .fetch_optional(state.db.pool())
    .await?;

    match row {
        None => Ok(None),
        Some(r) => Ok(Some(LastPollSummary {
            started_at: r.try_get(0)?,
            finished_at: r.try_get(1)?,
            vods_new: r.try_get(2)?,
            vods_updated: r.try_get(3)?,
            status: r.try_get(4)?,
        })),
    }
}
