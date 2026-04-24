# Sightline — Tauri IPC Contracts

> **Status.** Phase 1 defined the surface; Phase 2 extended it with Twitch credentials, streamers, VODs, settings, and polling controls. Every command listed here has Rust + TS types generated from a single source in `src-tauri/src/commands/` via [tauri-specta](https://github.com/specta-rs/tauri-specta) (see [ADR-0007](adr/0007-ipc-typegen.md)). TypeScript bindings live at `src/ipc/bindings.ts` — do not hand-edit.

## Rules

1. **One source of truth.** The Rust signature is canonical. TS bindings are regenerated on every `pnpm tauri dev` debug launch and via `cargo test --test ipc_bindings`.
2. **Typed errors.** Every command returns `Result<T, AppError>`. `AppError` is a tagged union serialized with `{ "kind": "...", ...fields }`; the frontend receives it as a TS discriminated union (see §Error model).
3. **No free-form strings.** Enumerations (ingest status, sort direction, log level) live in Rust `enum` types with `#[serde(rename_all = "snake_case")]`.
4. **Events are broadcast.** The backend emits events on well-known topics via `tauri::Emitter`; the frontend subscribes via `listen` from `@tauri-apps/api/event`.
5. **Additive evolution.** Add fields with serde defaults. A breaking change requires a new command name (`health_v2`) and a deprecation note here.

---

## Phase 1 commands

### `health`

Round-trips to verify that the webview, command bridge, and database are all alive.

Request: none. Response: `HealthReport { appName, appVersion, schemaVersion, startedAt, checkedAt }`. All timestamps are unix seconds UTC.

Errors: `AppError::Db { detail }` if SQLite is unreachable.

---

## Phase 2 commands

All Phase 2 commands are `async`. Inputs are passed as a single `input`/`patch` object (snake-case field names on the wire; camelCase in the generated TS).

### Credentials

- `setTwitchCredentials({ clientId, clientSecret })` → `CredentialsStatus`. Writes the pair to the OS keyring and persists `configured = true` + a masked Client-Id in `credentials_meta`. Emits `credentials:changed { configured: true }`.
- `getTwitchCredentialsStatus()` → `CredentialsStatus { configured, clientIdMasked, lastTokenAcquiredAt }`. **Never returns secrets.**
- `clearTwitchCredentials()` → `void`. Removes the keyring entries and resets the summary; emits `credentials:changed { configured: false }`.

Errors: `AppError::Credentials`, `AppError::InvalidInput`.

### Streamers

- `addStreamer({ login })` → `StreamerSummary`. Validates the login against the `^[A-Za-z0-9_]{3,25}$` regex, resolves via Helix `/users?login=`, and upserts the row (resurrecting a soft-deleted row if present). Emits `streamer:added`.
- `removeStreamer({ twitchUserId })` → `void`. Soft-deletes (`deleted_at`); VOD history stays. Emits `streamer:removed`.
- `listStreamers()` → `StreamerSummary[]`. Returns active rows with derived fields: `vodCount`, `eligibleVodCount`, `liveNow` (true if `last_live_at` within 10 min), and `nextPollEtaSeconds`.

Errors: `AppError::InvalidInput`, `AppError::TwitchNotFound`, `AppError::TwitchAuth`, `AppError::TwitchRateLimit`, `AppError::Db`.

### VODs

- `listVods({ filters, sort, limit, offset })` → `VodWithChapters[]`.
  - `filters.streamerIds?: string[]`
  - `filters.statuses?: string[]` — mirrors the `ingest_status` enum values (`pending`, `chapters_fetched`, `eligible`, `skipped_game`, `skipped_sub_only`, `skipped_live`, `error`)
  - `filters.gameIds?: string[]` — matches VODs that have at least one chapter with the given game id
  - `filters.since? / until?` — unix seconds UTC bounds on `stream_started_at`
  - `sort: "stream_started_at_desc" | "stream_started_at_asc"`
  - `limit` clamped [1, 500]; `offset` clamped to ≥ 0.
- `getVod({ twitchVideoId })` → `VodWithChapters { vod, chapters, streamerDisplayName, streamerLogin }`.

Errors: `AppError::NotFound`, `AppError::Db`.

### Settings

- `getSettings()` → `AppSettings`. Extended in Phase 3 with
  `libraryRoot`, `libraryLayout` (`plex` | `flat`), `stagingPath`,
  `maxConcurrentDownloads`, `bandwidthLimitBps` (`null` = unlimited),
  `qualityPreset`, `autoUpdateYtDlp`.
- `updateSettings(patch)` → `AppSettings`. Any subset of the top-level
  fields may be supplied. Intervals are normalized monotonically
  (`floor ≤ recent ≤ ceiling`); `concurrencyCap` clamped to [1, 16];
  `firstBackfillLimit` clamped to [1, 500]; `maxConcurrentDownloads`
  clamped to [1, 5]; `bandwidthLimitBps = -1` is a sentinel for
  "clear the cap" (stored as `null`).

Errors: `AppError::Db`, `AppError::Parse`.

### Polling

- `triggerPoll({ twitchUserId? })` → `void`. If `twitchUserId` is present, polls that one streamer on the next scheduler tick; otherwise re-evaluates every due streamer. Respects the global rate limit.
- `getPollStatus()` → `PollStatusRow[]`. Per-streamer summary: the `StreamerSummary` plus `lastPoll { startedAt, finishedAt, vodsNew, vodsUpdated, status }` if a prior poll exists.

---

## Phase 3 commands

### Downloads

- `enqueueDownload({ vodId, priority? })` → `DownloadRow`. Idempotent
  on the vod_id — re-enqueueing a row returns the existing one
  unchanged. Fires a wake-up to the worker pool.
- `pauseDownload({ vodId })` → `DownloadRow`. Valid only in
  `downloading` state. The in-flight yt-dlp child is aborted and the
  row transitions to `paused`; a later `resumeDownload` queues a
  fresh attempt.
- `resumeDownload({ vodId })` → `DownloadRow`. `paused → queued`,
  worker pool wakes up.
- `cancelDownload({ vodId })` → `DownloadRow`. Any non-completed state
  → `failed_permanent` with `last_error = "USER_CANCELLED"`.
- `retryDownload({ vodId })` → `DownloadRow`. Resets `attempts`,
  `bytes_done`, errors, and requeues. Works from either failed state.
- `reprioritizeDownload({ vodId, priority })` → `DownloadRow`. Higher
  priority runs first (default 100).
- `listDownloads({ filters? })` → `DownloadRow[]`. Ordered by
  `priority DESC, queued_at ASC`. Filters: `state?`, `streamerId?`.
- `getDownload({ vodId })` → `DownloadRow`. `AppError::NotFound` if
  the vod has never been enqueued.

### Storage

- `getStagingInfo()` → `StagingInfo { path, freeBytes, staleFileCount }`.
- `getLibraryInfo()` → `LibraryInfo { path?, freeBytes?, fileCount }`.
  `path` is `null` until the user picks a library root.

### Library migration

- `migrateLibrary({ targetLayout })` → `{ migrationId }`. Persists the
  layout choice in `app_settings` immediately, then spawns a
  background task that walks every `completed` download and moves
  files. Emits `library:migrating` / `library:migration_completed` /
  `library:migration_failed`. Errors: `AppError::LibraryMigration`
  (target equals current, no library root configured, another
  migration still running).
- `getMigrationStatus({ migrationId })` → `MigrationRow { id,
  startedAt, finishedAt?, fromLayout, toLayout, moved, errors,
  status }`.

---

## Events

Events use `tauri::Emitter::emit` from the Rust side. Topics and payload types are listed below; all payload types are emitted by tauri-specta into `src/ipc/bindings.ts`.

| Topic                   | Payload                                                                 | Fires when                                      |
| ----------------------- | ----------------------------------------------------------------------- | ----------------------------------------------- |
| `app:ready`             | `AppReadyEvent { startedAt }`                                           | DB migrated + command bridge ready (once).       |
| `credentials:changed`   | `CredentialsChangedEvent { configured }`                                | After set / clear.                               |
| `streamer:added`        | `StreamerAddedEvent { twitchUserId, login }`                            | After `addStreamer`.                             |
| `streamer:removed`      | `StreamerRemovedEvent { twitchUserId }`                                 | After `removeStreamer`.                          |
| `vod:ingested`          | `VodIngestedEvent { twitchVideoId, twitchUserId, ingestStatus, streamStartedAt }` | First time ingest records a VOD.                 |
| `vod:updated`           | `VodUpdatedEvent { twitchVideoId, ingestStatus }`                       | Subsequent status transitions.                   |
| `poll:started`          | `PollStartedEvent { twitchUserId, startedAt }`                          | At the top of a per-streamer poll (Phase 3 UX).  |
| `poll:finished`         | `PollFinishedEvent { twitchUserId, finishedAt, vodsNew, vodsUpdated, status }` | At the bottom of a per-streamer poll.           |
| `download:state_changed` | `DownloadStateChangedEvent { vodId, state }`                          | State-machine transition on a download row.     |
| `download:progress`      | `DownloadProgressEvent { vodId, progress, bytesDone, bytesTotal, speedBps, etaSeconds }` | yt-dlp progress tick, throttled to ≤ 2 Hz per download. |
| `download:completed`     | `DownloadCompletedEvent { vodId, finalPath }`                         | After the atomic move into the library succeeds. |
| `download:failed`        | `DownloadFailedEvent { vodId, reason }`                               | Retryable or permanent failure.                  |
| `library:migrating`      | `LibraryMigratingEvent { migrationId, moved, total }`                 | Per-file tick during a layout migration.         |
| `library:migration_completed` | `LibraryMigrationCompletedEvent { migrationId, moved, errors }`  | After a migration finishes (success or partial). |
| `library:migration_failed`    | `LibraryMigrationFailedEvent { migrationId, reason }`            | Migration aborted before completion.            |
| `storage:low_disk_warning`    | `StorageLowDiskWarningEvent { path, freeBytes }`                  | Fired once per threshold crossing; not continuous. |

Frontend cache invalidation subscribes to these in `src/lib/event-subscriptions.ts`. Topic names live in `src/ipc/index.ts::events` and `src-tauri/src/services/events.rs` as paired constants.

---

## Error model

```rust
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppError {
    Db { detail: String },
    Io { detail: String },
    InvalidInput { detail: String },
    NotFound,
    Internal { detail: String },
    Credentials { detail: String },
    TwitchAuth { detail: String },
    TwitchRateLimit { retry_after_seconds: u32 },
    TwitchApi { status: u16, detail: String },
    TwitchNotFound { detail: String },
    TwitchGql { detail: String },
    Ingest { detail: String },
    Parse { detail: String },
}
```

The TS side receives this as a discriminated union. Component code narrows on `error.kind`; never `catch (e: any)`. The generated Result shape from tauri-specta is unwrapped by `src/ipc/index.ts` into a throw-style API wrapping an `IpcError` class that carries the full `AppError`.

---

## Type generation

- Generation runs at `cargo build` in debug mode (invoked from `lib.rs` setup) and via `cargo test --test ipc_bindings` in CI.
- Output path: `src/ipc/bindings.ts`. Header marks the file as generated.
- `pnpm run check:ipc` runs the drift test + `git diff --exit-code` and is part of the phase-gate playbook.
- Hand-editing `src/ipc/bindings.ts` is prohibited.

---

## Capability matrix

| Capability file         | Window  | Grants                                             |
| ----------------------- | ------- | -------------------------------------------------- |
| `default.json`          | `main`  | `core:default` (all Phase 2 commands are in-process) |
| `library.json` *(P4)*     | `main`  | Library read/write commands                          |
| `player.json` *(P5)*      | `main`  | Watch-progress commands                              |

Capabilities are declared per-command (allow-list), never `"*"`.

---

## Security invariants (see also ADR-0007, ADR-0008)

- **Credentials cross IPC exactly once** — on the initial paste into `setTwitchCredentials`. Subsequent commands only return the status summary.
- **Secrets never reach logs** — `AppError` variants hold `detail: String` sourced from typed upstream errors; input validators in `services::credentials` trim and length-bound without reflecting the secret.
- **Keyring keys are compile-time constants** — service name = `"sightline"`, accounts = `"twitch_client_id" | "twitch_client_secret"`. No user-controlled key ever reaches `keyring::Entry::new`.
- **GQL Client-Id is hardcoded** — `PUBLIC_CLIENT_ID` constant; no user override path. See [ADR-0008](adr/0008-chapters-via-twitch-gql.md).
- **SQL is parameter-bound** — every `sqlx::query(...)` in the services layer uses `.bind(...)` for user input; dynamic `WHERE` clauses are built from a fixed set of fragments.
