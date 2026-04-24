// Hand-written thin wrapper over the generated tauri-specta bindings.
// Source of truth: `src-tauri/src/commands/**` + `src-tauri/src/ipc.rs`.
// See docs/adr/0007-ipc-typegen.md.
//
// The generator emits `Result<T, AppError>`-style commands
// (`{ status: "ok" } | { status: "error" }`). This file unwraps them into
// throw-style Promises so hooks and components can use idiomatic
// `await command(...)` calls, while keeping the discriminated `AppError`
// union intact via the `IpcError` class.

import {
  commands as generatedCommands,
  type AddStreamerInput,
  type AppError,
  type AppSettings,
  type AppReadyEvent,
  type AppShutdownRequestedEvent,
  type AppSummary,
  type AppTrayActionEvent,
  type AutostartStatus,
  type Chapter,
  type CoStream,
  type CredentialsChangedEvent,
  type CredentialsStatus,
  type DownloadCompletedEvent,
  type DownloadFailedEvent,
  type DownloadFilters,
  type DownloadProgressEvent,
  type DownloadRow,
  type DownloadState,
  type DownloadStateChangedEvent,
  type EnqueueDownloadInput,
  type GetCoStreamsInput,
  type GetVodInput,
  type HealthReport,
  type Interval,
  type LibraryInfo,
  type LibraryLayoutKind,
  type LibraryMigratingEvent,
  type LibraryMigrationCompletedEvent,
  type LibraryMigrationFailedEvent,
  type ListDownloadsInput,
  type ListTimelineInput,
  type ListVodsInput,
  type MigrateLibraryInput,
  type MigrateLibraryOutput,
  type MigrationIdInput,
  type MigrationRow,
  type NotificationCategory,
  type NotificationPayload,
  type PollFinishedEvent,
  type PollStartedEvent,
  type PollStatusRow,
  type QualityPreset,
  type RemoveStreamerInput,
  type ReprioritizeInput,
  type SetAutostartInput,
  type SetShortcutInput,
  type SetTwitchCredentialsInput,
  type SetWindowCloseBehaviorInput,
  type SettingsPatch,
  type Shortcut,
  type StagingInfo,
  type StorageLowDiskWarningEvent,
  type StreamerAddedEvent,
  type StreamerFavoritedEvent,
  type StreamerRemovedEvent,
  type StreamerUnfavoritedEvent,
  type StreamerSummary,
  type TimelineFilters,
  type TimelineIndexRebuildingEvent,
  type TimelineIndexRebuiltEvent,
  type TimelineStats,
  type ToggleFavoriteInput,
  type TrayActionInput,
  type TrayActionKind,
  type TriggerPollInput,
  type ContinueWatchingEntry,
  type GetWatchStatsInput,
  type ListContinueWatchingInput,
  type UpdateWatchProgressInput,
  type VideoSource,
  type VideoSourceState,
  type VodAssets,
  type VodAssetsInput,
  type VodIdInput,
  type VodIngestedEvent,
  type VodUpdatedEvent,
  type VodWithChapters,
  type WatchCompletedEvent,
  type WatchProgressRow,
  type WatchProgressUpdatedEvent,
  type WatchState,
  type WatchStateChangedEvent,
  type WatchStats,
  type WatchVodIdInput,
  type WindowCloseBehavior,
} from "@/ipc/bindings";

export type {
  AddStreamerInput,
  AppError,
  AppSettings,
  AppReadyEvent,
  AppShutdownRequestedEvent,
  AppSummary,
  AppTrayActionEvent,
  AutostartStatus,
  Chapter,
  CoStream,
  CredentialsChangedEvent,
  CredentialsStatus,
  DownloadCompletedEvent,
  DownloadFailedEvent,
  DownloadFilters,
  DownloadProgressEvent,
  DownloadRow,
  DownloadState,
  DownloadStateChangedEvent,
  EnqueueDownloadInput,
  GetCoStreamsInput,
  GetVodInput,
  HealthReport,
  Interval,
  LibraryInfo,
  LibraryLayoutKind,
  LibraryMigratingEvent,
  LibraryMigrationCompletedEvent,
  LibraryMigrationFailedEvent,
  ListDownloadsInput,
  ListTimelineInput,
  ListVodsInput,
  MigrateLibraryInput,
  MigrateLibraryOutput,
  MigrationIdInput,
  MigrationRow,
  NotificationCategory,
  NotificationPayload,
  PollFinishedEvent,
  PollStartedEvent,
  PollStatusRow,
  QualityPreset,
  RemoveStreamerInput,
  ReprioritizeInput,
  SetAutostartInput,
  SetShortcutInput,
  SetTwitchCredentialsInput,
  SetWindowCloseBehaviorInput,
  SettingsPatch,
  Shortcut,
  StagingInfo,
  StorageLowDiskWarningEvent,
  StreamerAddedEvent,
  StreamerFavoritedEvent,
  StreamerRemovedEvent,
  StreamerUnfavoritedEvent,
  StreamerSummary,
  TimelineFilters,
  TimelineIndexRebuildingEvent,
  TimelineIndexRebuiltEvent,
  TimelineStats,
  ToggleFavoriteInput,
  TrayActionInput,
  TrayActionKind,
  TriggerPollInput,
  ContinueWatchingEntry,
  GetWatchStatsInput,
  ListContinueWatchingInput,
  UpdateWatchProgressInput,
  VideoSource,
  VideoSourceState,
  VodAssets,
  VodAssetsInput,
  VodIdInput,
  VodIngestedEvent,
  VodUpdatedEvent,
  VodWithChapters,
  WatchCompletedEvent,
  WatchProgressRow,
  WatchProgressUpdatedEvent,
  WatchState,
  WatchStateChangedEvent,
  WatchStats,
  WatchVodIdInput,
  WindowCloseBehavior,
};

/**
 * Wraps a typed `AppError` so React hooks can `throw` on failure while
 * still exposing the full discriminated union to callers.
 */
export class IpcError extends Error {
  readonly appError: AppError;
  constructor(appError: AppError) {
    const detail = "detail" in appError ? appError.detail : appError.kind;
    super(`${appError.kind}: ${detail}`);
    this.appError = appError;
    this.name = "IpcError";
  }
}

type IpcResult<T> =
  | { status: "ok"; data: T }
  | { status: "error"; error: AppError };

function unwrap<T>(result: IpcResult<T>): T {
  if (result.status === "ok") return result.data;
  throw new IpcError(result.error);
}

/** Event topic strings. Central list so typos surface at compile time. */
export const events = {
  appReady: "app:ready",
  credentialsChanged: "credentials:changed",
  streamerAdded: "streamer:added",
  streamerRemoved: "streamer:removed",
  streamerFavorited: "streamer:favorited",
  streamerUnfavorited: "streamer:unfavorited",
  vodIngested: "vod:ingested",
  vodUpdated: "vod:updated",
  pollStarted: "poll:started",
  pollFinished: "poll:finished",
  downloadStateChanged: "download:state_changed",
  downloadProgress: "download:progress",
  downloadCompleted: "download:completed",
  downloadFailed: "download:failed",
  libraryMigrating: "library:migrating",
  libraryMigrationCompleted: "library:migration_completed",
  libraryMigrationFailed: "library:migration_failed",
  storageLowDiskWarning: "storage:low_disk_warning",
  timelineIndexRebuilding: "timeline:index_rebuilding",
  timelineIndexRebuilt: "timeline:index_rebuilt",
  appTrayAction: "app:tray_action",
  appShutdownRequested: "app:shutdown_requested",
  notificationShow: "notification:show",
  watchProgressUpdated: "watch:progress_updated",
  watchStateChanged: "watch:state_changed",
  watchCompleted: "watch:completed",
} as const;

/**
 * Throw-style wrappers around the generated Result-style commands. One
 * entry per Rust `#[tauri::command]`. Type inference flows from the
 * generator so new fields show up here automatically.
 */
export const commands = {
  health: async (): Promise<HealthReport> =>
    unwrap(await generatedCommands.health()),
  setTwitchCredentials: async (
    input: SetTwitchCredentialsInput
  ): Promise<CredentialsStatus> =>
    unwrap(await generatedCommands.setTwitchCredentials(input)),
  getTwitchCredentialsStatus: async (): Promise<CredentialsStatus> =>
    unwrap(await generatedCommands.getTwitchCredentialsStatus()),
  clearTwitchCredentials: async (): Promise<void> => {
    unwrap(await generatedCommands.clearTwitchCredentials());
  },
  addStreamer: async (input: AddStreamerInput): Promise<StreamerSummary> =>
    unwrap(await generatedCommands.addStreamer(input)),
  removeStreamer: async (input: RemoveStreamerInput): Promise<void> => {
    unwrap(await generatedCommands.removeStreamer(input));
  },
  listStreamers: async (): Promise<StreamerSummary[]> =>
    unwrap(await generatedCommands.listStreamers()),
  listVods: async (input: ListVodsInput): Promise<VodWithChapters[]> =>
    unwrap(await generatedCommands.listVods(input)),
  getVod: async (input: GetVodInput): Promise<VodWithChapters> =>
    unwrap(await generatedCommands.getVod(input)),
  getSettings: async (): Promise<AppSettings> =>
    unwrap(await generatedCommands.getSettings()),
  updateSettings: async (patch: SettingsPatch): Promise<AppSettings> =>
    unwrap(await generatedCommands.updateSettings(patch)),
  triggerPoll: async (input: TriggerPollInput): Promise<void> => {
    unwrap(await generatedCommands.triggerPoll(input));
  },
  getPollStatus: async (): Promise<PollStatusRow[]> =>
    unwrap(await generatedCommands.getPollStatus()),
  // --- Phase 3 ---
  enqueueDownload: async (input: EnqueueDownloadInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.enqueueDownload(input)),
  pauseDownload: async (input: VodIdInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.pauseDownload(input)),
  resumeDownload: async (input: VodIdInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.resumeDownload(input)),
  cancelDownload: async (input: VodIdInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.cancelDownload(input)),
  retryDownload: async (input: VodIdInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.retryDownload(input)),
  reprioritizeDownload: async (
    input: ReprioritizeInput
  ): Promise<DownloadRow> =>
    unwrap(await generatedCommands.reprioritizeDownload(input)),
  listDownloads: async (input: ListDownloadsInput): Promise<DownloadRow[]> =>
    unwrap(await generatedCommands.listDownloads(input)),
  getDownload: async (input: VodIdInput): Promise<DownloadRow> =>
    unwrap(await generatedCommands.getDownload(input)),
  getStagingInfo: async (): Promise<StagingInfo> =>
    unwrap(await generatedCommands.getStagingInfo()),
  getLibraryInfo: async (): Promise<LibraryInfo> =>
    unwrap(await generatedCommands.getLibraryInfo()),
  migrateLibrary: async (
    input: MigrateLibraryInput
  ): Promise<MigrateLibraryOutput> =>
    unwrap(await generatedCommands.migrateLibrary(input)),
  getMigrationStatus: async (input: MigrationIdInput): Promise<MigrationRow> =>
    unwrap(await generatedCommands.getMigrationStatus(input)),
  // --- Phase 4 ---
  listTimeline: async (input: ListTimelineInput): Promise<Interval[]> =>
    unwrap(await generatedCommands.listTimeline(input)),
  getCoStreams: async (input: GetCoStreamsInput): Promise<CoStream[]> =>
    unwrap(await generatedCommands.getCoStreams(input)),
  getTimelineStats: async (): Promise<TimelineStats> =>
    unwrap(await generatedCommands.getTimelineStats()),
  rebuildTimelineIndex: async (): Promise<TimelineStats> =>
    unwrap(await generatedCommands.rebuildTimelineIndex()),
  getAppSummary: async (): Promise<AppSummary> =>
    unwrap(await generatedCommands.getAppSummary()),
  pauseAllDownloads: async (): Promise<number> =>
    unwrap(await generatedCommands.pauseAllDownloads()),
  resumeAllDownloads: async (): Promise<number> =>
    unwrap(await generatedCommands.resumeAllDownloads()),
  setWindowCloseBehavior: async (
    input: SetWindowCloseBehaviorInput
  ): Promise<void> => {
    unwrap(await generatedCommands.setWindowCloseBehavior(input));
  },
  toggleStreamerFavorite: async (
    input: ToggleFavoriteInput
  ): Promise<boolean> =>
    unwrap(await generatedCommands.toggleStreamerFavorite(input)),
  requestShutdown: async (): Promise<void> => {
    unwrap(await generatedCommands.requestShutdown());
  },
  emitTrayAction: async (input: TrayActionInput): Promise<void> => {
    unwrap(await generatedCommands.emitTrayAction(input));
  },
  listShortcuts: async (): Promise<Shortcut[]> =>
    unwrap(await generatedCommands.listShortcuts()),
  setShortcut: async (input: SetShortcutInput): Promise<Shortcut[]> =>
    unwrap(await generatedCommands.setShortcut(input)),
  resetShortcuts: async (): Promise<Shortcut[]> =>
    unwrap(await generatedCommands.resetShortcuts()),
  // --- Phase 5 ---
  getVodAssets: async (input: VodAssetsInput): Promise<VodAssets> =>
    unwrap(await generatedCommands.getVodAssets(input)),
  regenerateVodThumbnail: async (input: VodAssetsInput): Promise<void> => {
    unwrap(await generatedCommands.regenerateVodThumbnail(input));
  },
  getVideoSource: async (input: VodAssetsInput): Promise<VideoSource> =>
    unwrap(await generatedCommands.getVideoSource(input)),
  requestRemux: async (input: VodAssetsInput): Promise<void> => {
    unwrap(await generatedCommands.requestRemux(input));
  },
  getWatchProgress: async (
    input: WatchVodIdInput
  ): Promise<WatchProgressRow | null> =>
    unwrap(await generatedCommands.getWatchProgress(input)),
  updateWatchProgress: async (
    input: UpdateWatchProgressInput
  ): Promise<WatchProgressRow> =>
    unwrap(await generatedCommands.updateWatchProgress(input)),
  markWatched: async (input: WatchVodIdInput): Promise<WatchProgressRow> =>
    unwrap(await generatedCommands.markWatched(input)),
  markUnwatched: async (input: WatchVodIdInput): Promise<WatchProgressRow> =>
    unwrap(await generatedCommands.markUnwatched(input)),
  listContinueWatching: async (
    input: ListContinueWatchingInput
  ): Promise<ContinueWatchingEntry[]> =>
    unwrap(await generatedCommands.listContinueWatching(input)),
  getWatchStats: async (input: GetWatchStatsInput): Promise<WatchStats> =>
    unwrap(await generatedCommands.getWatchStats(input)),
  getAutostartStatus: async (): Promise<AutostartStatus> =>
    unwrap(await generatedCommands.getAutostartStatus()),
  setAutostart: async (input: SetAutostartInput): Promise<AutostartStatus> =>
    unwrap(await generatedCommands.setAutostart(input)),
};

/**
 * Raw access to the generator's Result shape, for code paths that prefer
 * branching on `status` rather than catching.
 */
export const rawCommands = generatedCommands;
