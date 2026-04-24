//! Tauri event payload shapes.
//!
//! Services construct the payloads; the `emit!` helpers below fan them
//! out through a `tauri::AppHandle` to the webview. Keeping the shapes
//! in the services layer (rather than `commands`) means tests can
//! construct them without a running Tauri runtime.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Event bus topic names. Central constants so misspellings surface at
/// compile time rather than at runtime.
pub const EV_APP_READY: &str = "app:ready";
pub const EV_CREDENTIALS_CHANGED: &str = "credentials:changed";
pub const EV_STREAMER_ADDED: &str = "streamer:added";
pub const EV_STREAMER_REMOVED: &str = "streamer:removed";
pub const EV_VOD_INGESTED: &str = "vod:ingested";
pub const EV_VOD_UPDATED: &str = "vod:updated";
pub const EV_POLL_STARTED: &str = "poll:started";
pub const EV_POLL_FINISHED: &str = "poll:finished";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppReadyEvent {
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsChangedEvent {
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreamerAddedEvent {
    pub twitch_user_id: String,
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreamerRemovedEvent {
    pub twitch_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VodIngestedEvent {
    pub twitch_video_id: String,
    pub twitch_user_id: String,
    /// Mirrors `vods.ingest_status`.
    pub ingest_status: String,
    pub stream_started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VodUpdatedEvent {
    pub twitch_video_id: String,
    pub ingest_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PollStartedEvent {
    pub twitch_user_id: String,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PollFinishedEvent {
    pub twitch_user_id: String,
    pub finished_at: i64,
    pub vods_new: i64,
    pub vods_updated: i64,
    pub status: String,
}
