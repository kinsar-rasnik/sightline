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
  type GetVodInput,
  type HealthReport,
  type LibraryInfo,
  type LibraryLayoutKind,
  type LibraryMigratingEvent,
  type LibraryMigrationCompletedEvent,
  type LibraryMigrationFailedEvent,
  type ListDownloadsInput,
  type ListVodsInput,
  type MigrateLibraryInput,
  type MigrateLibraryOutput,
  type MigrationIdInput,
  type MigrationRow,
  type PollFinishedEvent,
  type PollStartedEvent,
  type PollStatusRow,
  type QualityPreset,
  type RemoveStreamerInput,
  type ReprioritizeInput,
  type SetTwitchCredentialsInput,
  type SettingsPatch,
  type StagingInfo,
  type StorageLowDiskWarningEvent,
  type StreamerAddedEvent,
  type StreamerRemovedEvent,
  type StreamerSummary,
  type TriggerPollInput,
  type VodIdInput,
  type VodIngestedEvent,
  type VodUpdatedEvent,
  type VodWithChapters,
} from "@/ipc/bindings";

export type {
  AddStreamerInput,
  AppError,
  AppSettings,
  AppReadyEvent,
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
  GetVodInput,
  HealthReport,
  LibraryInfo,
  LibraryLayoutKind,
  LibraryMigratingEvent,
  LibraryMigrationCompletedEvent,
  LibraryMigrationFailedEvent,
  ListDownloadsInput,
  ListVodsInput,
  MigrateLibraryInput,
  MigrateLibraryOutput,
  MigrationIdInput,
  MigrationRow,
  PollFinishedEvent,
  PollStartedEvent,
  PollStatusRow,
  QualityPreset,
  RemoveStreamerInput,
  ReprioritizeInput,
  SetTwitchCredentialsInput,
  SettingsPatch,
  StagingInfo,
  StorageLowDiskWarningEvent,
  StreamerAddedEvent,
  StreamerRemovedEvent,
  StreamerSummary,
  TriggerPollInput,
  VodIdInput,
  VodIngestedEvent,
  VodUpdatedEvent,
  VodWithChapters,
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
};

/**
 * Raw access to the generator's Result shape, for code paths that prefer
 * branching on `status` rather than catching.
 */
export const rawCommands = generatedCommands;
