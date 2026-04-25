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

// --- Phase 4: timeline + tray + shutdown + favorites ---
pub const EV_TIMELINE_INDEX_REBUILDING: &str = "timeline:index_rebuilding";
pub const EV_TIMELINE_INDEX_REBUILT: &str = "timeline:index_rebuilt";
pub const EV_STREAMER_FAVORITED: &str = "streamer:favorited";
pub const EV_STREAMER_UNFAVORITED: &str = "streamer:unfavorited";
pub const EV_APP_TRAY_ACTION: &str = "app:tray_action";
pub const EV_APP_SHUTDOWN_REQUESTED: &str = "app:shutdown_requested";

// --- Phase 5: watch progress ---
pub const EV_WATCH_PROGRESS_UPDATED: &str = "watch:progress_updated";
pub const EV_WATCH_STATE_CHANGED: &str = "watch:state_changed";
pub const EV_WATCH_COMPLETED: &str = "watch:completed";

// --- Phase 6: multi-view sync engine ---
pub const EV_SYNC_STATE_CHANGED: &str = "sync:state_changed";
pub const EV_SYNC_DRIFT_CORRECTED: &str = "sync:drift_corrected";
pub const EV_SYNC_LEADER_CHANGED: &str = "sync:leader_changed";
pub const EV_SYNC_MEMBER_OUT_OF_RANGE: &str = "sync:member_out_of_range";
pub const EV_SYNC_GROUP_CLOSED: &str = "sync:group_closed";

// --- Phase 7: auto-cleanup + updater ---
pub const EV_CLEANUP_PLAN_READY: &str = "cleanup:plan_ready";
pub const EV_CLEANUP_EXECUTED: &str = "cleanup:executed";
pub const EV_CLEANUP_DISK_PRESSURE: &str = "cleanup:disk_pressure";
pub const EV_UPDATER_UPDATE_AVAILABLE: &str = "updater:update_available";
pub const EV_UPDATER_CHECK_FAILED: &str = "updater:check_failed";

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

// --- Phase 4 event payloads ---

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineIndexRebuildingEvent {
    /// Fraction complete in [0.0, 1.0].
    pub progress: f64,
    pub processed: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineIndexRebuiltEvent {
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreamerFavoritedEvent {
    pub twitch_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreamerUnfavoritedEvent {
    pub twitch_user_id: String,
}

/// Tray menu → webview coordination signal. The tray handler emits a
/// specific `kind` string; the frontend narrows on it to route UI
/// state (e.g. focus the Downloads route, surface a pause toast).
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppTrayActionEvent {
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppShutdownRequestedEvent {
    /// Deadline in unix-seconds — the services flush by this time.
    pub deadline_at: i64,
}

// --- Phase 5 event payloads ---

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchProgressUpdatedEvent {
    pub vod_id: String,
    pub position_seconds: f64,
    /// Mirrors `WatchState` db strings.
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WatchStateChangedEvent {
    pub vod_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WatchCompletedEvent {
    pub vod_id: String,
}

// --- Phase 6 event payloads ---

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStateChangedEvent {
    pub session_id: i64,
    /// Mirrors `SyncStatus` db strings (`active` | `closed`).
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncDriftCorrectedEvent {
    pub session_id: i64,
    pub pane_index: i64,
    pub drift_ms: f64,
    pub corrected_to_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncLeaderChangedEvent {
    pub session_id: i64,
    pub leader_pane_index: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncMemberOutOfRangeEvent {
    pub session_id: i64,
    pub pane_index: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncGroupClosedEvent {
    pub session_id: i64,
}

// --- Phase 7 event payloads ---

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlanReadyEvent {
    pub candidate_count: i64,
    pub projected_freed_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecutedEvent {
    pub mode: String,
    pub status: String,
    pub freed_bytes: i64,
    pub deleted_vod_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupDiskPressureEvent {
    pub used_fraction: f64,
    pub free_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterUpdateAvailableEvent {
    pub version: String,
    pub release_url: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterCheckFailedEvent {
    pub reason: String,
}
