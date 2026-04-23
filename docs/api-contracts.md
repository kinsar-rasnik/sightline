# Sightline — Tauri IPC Contracts

> **Status.** Phase 1 defines the contract surface and the `health` command. Later phases extend additively. Every command listed here has Rust + TS types generated from a single source in `src-tauri/src/commands/` via [tauri-specta](https://github.com/specta-rs/tauri-specta).

## Rules

1. **One source of truth.** The Rust signature is canonical. TS bindings are regenerated — never hand-edited.
2. **Typed errors.** Every command returns `Result<T, AppError>`. `AppError` is a tagged union serialized with `{ "kind": "...", "detail": ... }`.
3. **No free-form strings.** Enumerations (download state, log level, ...) live in Rust `enum` types with `#[serde(rename_all = "snake_case")]`.
4. **Events are broadcast.** The backend emits events on a well-known topic; the frontend subscribes. Events are for state-fanout, not for command replies.
5. **Additive evolution.** Add fields with serde defaults. A breaking change requires a new command name (`health_v2`) and a deprecation note in this document.

---

## Phase 1 commands

### `health`

Round-trips to verify that the webview, command bridge, and database are all alive.

**Request.** No parameters.

**Response.**

```ts
type HealthReport = {
  appName: string;        // "sightline"
  appVersion: string;     // crate version from Cargo.toml
  schemaVersion: number;  // PRAGMA user_version
  startedAt: number;      // unix seconds, UTC, when the process started
  checkedAt: number;      // unix seconds, UTC, when this call was handled
};
```

**Errors.**

- `AppError::Db { detail }` if the SQLite handle is unavailable.

**Example (TypeScript).**

```ts
import { commands } from "~/ipc";

const report = await commands.health();
if (report.status === "ok") {
  console.log(report.data.appVersion);
} else {
  console.error(report.error);
}
```

---

## Phase 2 commands (planned)

### `followStreamer(input: FollowStreamerInput): FollowStreamerOutput`
### `unfollowStreamer(input: { twitchUserId: string }): void`
### `listStreamers(): StreamerRow[]`
### `listVods(input: ListVodsInput): VodRow[]`
### `setTwitchCredentials(input: { clientId: string; clientSecret: string }): void`
### `getTwitchCredentialsStatus(): { configured: boolean }`

Full request/response shapes will be defined in the Phase 2 ADR.

---

## Phase 3 commands (planned)

### `queueDownload(input: { twitchVideoId: string }): void`
### `pauseDownload(input: { twitchVideoId: string }): void`
### `resumeDownload(input: { twitchVideoId: string }): void`
### `cancelDownload(input: { twitchVideoId: string }): void`
### `listActiveDownloads(): DownloadRow[]`

---

## Events

Events use `tauri::Emitter::emit` from the Rust side. The frontend subscribes via `listen` from `@tauri-apps/api/event`.

### `app:ready`

Emitted once, after the DB has migrated and the command bridge is ready.

Payload:

```ts
type AppReadyEvent = { startedAt: number };
```

### `vod:discovered` *(Phase 2)*

Emitted for each VOD discovered in a poll cycle.

```ts
type VodDiscoveredEvent = {
  twitchVideoId: string;
  twitchUserId: string;
  streamStartedAt: number;
  state: "eligible" | "ignored";
  ignoredReason?: string;
};
```

### `download:progress` *(Phase 3)*

Emitted on every percent tick of an active download.

```ts
type DownloadProgressEvent = {
  twitchVideoId: string;
  bytesDownloaded: number;
  bytesTotal: number | null;
  state: "downloading" | "paused" | "completed" | "failed";
};
```

---

## Error model

```rust
// src-tauri/src/error.rs
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppError {
    #[error("database error: {detail}")]
    Db { detail: String },

    #[error("twitch api error: {detail}")]
    Twitch { detail: String },

    #[error("invalid input: {detail}")]
    InvalidInput { detail: String },

    #[error("not found")]
    NotFound,
}
```

The TS side receives this as a discriminated union. Command handlers in the frontend handle each `kind` explicitly — never `catch (e: any)`.

---

## Type generation

- Generation runs as part of `cargo build` via a build-script feature flag and writes into `src/ipc/bindings.ts`.
- CI enforces the bindings are in sync: `pnpm run check:ipc` diffs a freshly-generated file against the committed one.
- Hand-editing `src/ipc/bindings.ts` is prohibited. The file starts with a machine-generated warning header.

---

## Capability matrix

Each window declares its capabilities under `src-tauri/capabilities/`. A command is only callable from a window that has been granted it.

| Capability file      | Window  | Grants                                               |
| -------------------- | ------- | ---------------------------------------------------- |
| `default.json`       | `main`  | `health`, `app:ready`                                |
| `library.json` *(P4)*  | `main`  | library read/write commands                          |
| `player.json` *(P5)*   | `main`  | watch-progress commands                              |

Capabilities are declared per-command (allow-list), never `"*"`.
