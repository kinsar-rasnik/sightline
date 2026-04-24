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

// --- Phase 3: downloads + library + storage events. ---
pub const EV_DOWNLOAD_STATE_CHANGED: &str = "download:state_changed";
pub const EV_DOWNLOAD_PROGRESS: &str = "download:progress";
pub const EV_DOWNLOAD_COMPLETED: &str = "download:completed";
pub const EV_DOWNLOAD_FAILED: &str = "download:failed";

pub const EV_LIBRARY_MIGRATING: &str = "library:migrating";
pub const EV_LIBRARY_MIGRATION_COMPLETED: &str = "library:migration_completed";
pub const EV_LIBRARY_MIGRATION_FAILED: &str = "library:migration_failed";

pub const EV_STORAGE_LOW_DISK_WARNING: &str = "storage:low_disk_warning";

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

// --- Phase 3 event payloads ---

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStateChangedEvent {
    pub vod_id: String,
    /// Matches `DownloadState` wire strings.
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub vod_id: String,
    pub progress: Option<f64>,
    pub bytes_done: i64,
    pub bytes_total: Option<i64>,
    pub speed_bps: Option<i64>,
    pub eta_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCompletedEvent {
    pub vod_id: String,
    pub final_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFailedEvent {
    pub vod_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMigratingEvent {
    pub migration_id: i64,
    pub moved: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMigrationCompletedEvent {
    pub migration_id: i64,
    pub moved: i64,
    pub errors: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMigrationFailedEvent {
    pub migration_id: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageLowDiskWarningEvent {
    pub path: String,
    pub free_bytes: i64,
}
