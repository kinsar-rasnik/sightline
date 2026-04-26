//! IPC binding builder.
//!
//! A single source of truth for the command + event surface exposed to the
//! webview. `ipc_builder()` is consumed by:
//!
//! * [`crate::run`] — mounts it on the Tauri builder and (in debug builds)
//!   regenerates `src/ipc/bindings.ts`.
//! * `tests/ipc_bindings.rs` — regenerates the file and lets CI diff it
//!   against the committed copy to detect drift.
//!
//! See ADR-0007 for the rationale.

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use tauri::Wry;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands;

/// Build the tauri-specta `Builder` with every command and event the app
/// exposes. The same instance is used at runtime (as the `invoke_handler`
/// source) and offline (to regenerate bindings).
pub fn ipc_builder() -> Builder<Wry> {
    use crate::services::autostart::AutostartStatus;
    use crate::services::events::{
        AppReadyEvent, AppShutdownRequestedEvent, AppTrayActionEvent, CleanupDiskPressureEvent,
        CleanupExecutedEvent, CleanupPlanReadyEvent, CredentialsChangedEvent,
        DistributionPrefetchTriggeredEvent, DistributionVodArchivedEvent,
        DistributionVodPickedEvent, DistributionWindowEnforcedEvent, DownloadCompletedEvent,
        DownloadFailedEvent, DownloadProgressEvent, DownloadStateChangedEvent,
        LibraryMigratingEvent, LibraryMigrationCompletedEvent, LibraryMigrationFailedEvent,
        PollFinishedEvent, PollStartedEvent, StorageLowDiskWarningEvent, StreamerAddedEvent,
        StreamerFavoritedEvent, StreamerRemovedEvent, StreamerUnfavoritedEvent,
        SyncDriftCorrectedEvent, SyncGroupClosedEvent, SyncLeaderChangedEvent,
        SyncMemberOutOfRangeEvent, SyncStateChangedEvent, TimelineIndexRebuildingEvent,
        TimelineIndexRebuiltEvent, UpdaterCheckFailedEvent, UpdaterUpdateAvailableEvent,
        VodIngestedEvent, VodUpdatedEvent, WatchCompletedEvent, WatchProgressUpdatedEvent,
        WatchStateChangedEvent,
    };
    use crate::services::notifications::NotificationPayload;

    Builder::<Wry>::new()
        .commands(collect_commands![
            commands::health::health,
            commands::credentials::set_twitch_credentials,
            commands::credentials::get_twitch_credentials_status,
            commands::credentials::clear_twitch_credentials,
            commands::streamers::add_streamer,
            commands::streamers::remove_streamer,
            commands::streamers::list_streamers,
            commands::vods::list_vods,
            commands::vods::get_vod,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::poll::trigger_poll,
            commands::poll::get_poll_status,
            // Phase 3
            commands::downloads::enqueue_download,
            commands::downloads::pause_download,
            commands::downloads::resume_download,
            commands::downloads::cancel_download,
            commands::downloads::retry_download,
            commands::downloads::reprioritize_download,
            commands::downloads::list_downloads,
            commands::downloads::get_download,
            commands::storage::get_staging_info,
            commands::storage::get_library_info,
            commands::storage::migrate_library,
            commands::storage::get_migration_status,
            // Phase 4
            commands::timeline::list_timeline,
            commands::timeline::get_co_streams,
            commands::timeline::get_timeline_stats,
            commands::timeline::rebuild_timeline_index,
            commands::app::get_app_summary,
            commands::app::pause_all_downloads,
            commands::app::resume_all_downloads,
            commands::app::set_window_close_behavior,
            commands::app::toggle_streamer_favorite,
            commands::app::request_shutdown,
            commands::app::emit_tray_action,
            commands::app::list_shortcuts,
            commands::app::set_shortcut,
            commands::app::reset_shortcuts,
            // Phase 5
            commands::media::get_vod_assets,
            commands::media::regenerate_vod_thumbnail,
            commands::media::get_video_source,
            commands::media::request_remux,
            commands::watch::get_watch_progress,
            commands::watch::update_watch_progress,
            commands::watch::mark_watched,
            commands::watch::mark_unwatched,
            commands::watch::list_continue_watching,
            commands::watch::get_watch_stats,
            commands::autostart::get_autostart_status,
            commands::autostart::set_autostart,
            // Phase 6: multi-view sync engine
            commands::sync::open_sync_group,
            commands::sync::close_sync_group,
            commands::sync::get_sync_group,
            commands::sync::set_sync_leader,
            commands::sync::sync_seek,
            commands::sync::sync_play,
            commands::sync::sync_pause,
            commands::sync::sync_set_speed,
            commands::sync::get_overlap,
            commands::sync::record_sync_drift,
            commands::sync::report_sync_out_of_range,
            // Phase 7: auto-cleanup
            commands::cleanup::get_cleanup_plan,
            commands::cleanup::execute_cleanup,
            commands::cleanup::get_cleanup_history,
            commands::cleanup::get_disk_usage,
            // Phase 7: update checker
            commands::updater::check_for_update,
            commands::updater::get_update_status,
            commands::updater::skip_update_version,
            commands::updater::open_release_url,
            // Phase 8: quality pipeline
            commands::quality::get_encoder_capability,
            commands::quality::redetect_encoders,
            commands::quality::set_video_quality_profile,
            // Phase 8: pull-on-demand distribution
            commands::distribution::pick_vod,
            commands::distribution::pick_next_n,
            commands::distribution::unpick_vod,
            commands::distribution::set_distribution_mode,
            commands::distribution::set_sliding_window_size,
        ])
        .events(collect_events![])
        // Register the event payload shapes so the frontend gets their TS
        // types even though nothing returns them from a `#[tauri::command]`.
        // See ADR-0007 on the event-vs-command boundary.
        .typ::<AppReadyEvent>()
        .typ::<CredentialsChangedEvent>()
        .typ::<StreamerAddedEvent>()
        .typ::<StreamerRemovedEvent>()
        .typ::<VodIngestedEvent>()
        .typ::<VodUpdatedEvent>()
        .typ::<PollStartedEvent>()
        .typ::<PollFinishedEvent>()
        // Phase 3
        .typ::<DownloadStateChangedEvent>()
        .typ::<DownloadProgressEvent>()
        .typ::<DownloadCompletedEvent>()
        .typ::<DownloadFailedEvent>()
        .typ::<LibraryMigratingEvent>()
        .typ::<LibraryMigrationCompletedEvent>()
        .typ::<LibraryMigrationFailedEvent>()
        .typ::<StorageLowDiskWarningEvent>()
        // Phase 4
        .typ::<TimelineIndexRebuildingEvent>()
        .typ::<TimelineIndexRebuiltEvent>()
        .typ::<StreamerFavoritedEvent>()
        .typ::<StreamerUnfavoritedEvent>()
        .typ::<AppTrayActionEvent>()
        .typ::<AppShutdownRequestedEvent>()
        .typ::<NotificationPayload>()
        // Phase 5
        .typ::<AutostartStatus>()
        .typ::<WatchProgressUpdatedEvent>()
        .typ::<WatchStateChangedEvent>()
        .typ::<WatchCompletedEvent>()
        // Phase 6
        .typ::<SyncStateChangedEvent>()
        .typ::<SyncDriftCorrectedEvent>()
        .typ::<SyncLeaderChangedEvent>()
        .typ::<SyncMemberOutOfRangeEvent>()
        .typ::<SyncGroupClosedEvent>()
        // Phase 7
        .typ::<CleanupPlanReadyEvent>()
        .typ::<CleanupExecutedEvent>()
        .typ::<CleanupDiskPressureEvent>()
        .typ::<UpdaterUpdateAvailableEvent>()
        .typ::<UpdaterCheckFailedEvent>()
        // Phase 8
        .typ::<DistributionVodPickedEvent>()
        .typ::<DistributionVodArchivedEvent>()
        .typ::<DistributionPrefetchTriggeredEvent>()
        .typ::<DistributionWindowEnforcedEvent>()
}

/// Target path of the generated TS bindings, relative to the workspace
/// root. Computed from `CARGO_MANIFEST_DIR` so the test and the runtime
/// export always resolve to the same place.
pub fn bindings_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // `src-tauri/` → `../src/ipc/bindings.ts`.
    manifest
        .parent()
        .unwrap_or(manifest)
        .join("src")
        .join("ipc")
        .join("bindings.ts")
}

/// Export the Builder to the canonical bindings file. Called from the
/// debug-mode `setup` hook and from the drift test.
pub fn export_bindings(builder: &Builder<Wry>) -> Result<(), String> {
    let config = Typescript::default().header(TS_HEADER);

    builder
        .export(config, bindings_path())
        .map_err(|e| format!("tauri-specta export: {e}"))
}

const TS_HEADER: &str = "// AUTO-GENERATED — DO NOT EDIT.\n\
// Regenerated on every `pnpm tauri dev` (debug) and via\n\
// `cargo test --test ipc_bindings` (CI drift check). See\n\
// docs/adr/0007-ipc-typegen.md.\n\
\n\
/* eslint-disable */\n";
