# Sightline — Technical Specification

> **Status.** Draft. Each Phase refines the parts it touches.

## 1. Scope

Sightline is a native desktop application (macOS, Windows, Linux) that aggregates Twitch VODs for a user-curated set of GTA-RP streamers, filters by game, downloads them via a yt-dlp sidecar, and plays them back in a chronological library. All state is local.

## 2. Non-goals

- Server component. There is no cloud backend. An ADR is required before that changes.
- End-user Twitch OAuth. We read public VOD metadata via App Access Tokens only.
- A hosted directory of streamers. The user curates their own list.

## 3. High-level architecture

```
┌───────────────────────────────────────────────────────────┐
│  Frontend (Webview)                                       │
│  React 19 · TypeScript · Tailwind · shadcn · TanStack     │
│  Query · Zustand · tauri-specta bindings                  │
└────────────▲──────────────────────────────▲───────────────┘
             │ typed invoke()               │ typed event()
┌────────────┴──────────────────────────────┴───────────────┐
│  Tauri 2 Command/Event Bridge                             │
└────────────▲──────────────────────────────▲───────────────┘
             │                              │
┌────────────┴──────────────────────────────┴───────────────┐
│  Rust Process (Tokio)                                     │
│  commands/  — thin, serde-only                            │
│  services/  — orchestration (Poller, Downloader, ...)     │
│  domain/    — pure types, invariants                      │
│  infra/     — Db, HelixClient, YtDlp, Keyring, Fs         │
└───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────┐   ┌────────────────────────────┐
│ SQLite (sqlx)          │   │ Sidecars (yt-dlp, ffmpeg)  │
│ Local only, journaled  │   │ Pinned version + checksum  │
└────────────────────────┘   └────────────────────────────┘
```

## 4. Component responsibilities

### 4.1 Frontend
- Render library, settings, player, sync view.
- All backend interaction via `src/ipc/` (generated; do not hand-edit).
- No direct filesystem or network access.

### 4.2 Tauri bridge
- Host process + webview. Commands declared with `#[tauri::command]`.
- Capabilities configured per-window in `src-tauri/capabilities/` with the principle of least privilege.

### 4.3 Rust backend
- **commands/** — one file per feature (`health.rs`, `streamers.rs`, `vods.rs`, …). Each handler deserializes input, calls a service, and serializes the result. No business logic.
- **services/** — orchestrates domain operations. Examples: `PollerService`, `DownloadService`, `LibraryService`. Holds `Arc`-wrapped handles to infra.
- **domain/** — plain data types and invariants. No I/O imports.
- **infra/** — the outside world:
  - `db.rs` — `sqlx::SqlitePool` + migrations.
  - `helix.rs` (Phase 2+) — Twitch Helix client.
  - `sidecar/` (Phase 3+) — yt-dlp and ffmpeg process management.
  - `keyring.rs` — OS keyring wrapper (`keyring` crate).
  - `fs.rs` — library-root path management with sync-provider tolerance.

### 4.4 State stores

| Store         | Lives in                             | Purpose                           |
| ------------- | ------------------------------------ | --------------------------------- |
| Config        | JSON in OS config dir                | Non-secret prefs, paths           |
| Credentials   | OS keyring                           | Twitch Client ID + Secret         |
| Structured    | SQLite under the library root        | Streamers, VODs, downloads, watch |
| Large blobs   | Files under `library_root/vods/`     | VOD video files                   |

## 5. Key constraints

- **Offline-capable.** The app starts and renders even if the network is unreachable. Polling errors are non-fatal; the UI surfaces them.
- **Proton Drive / OneDrive / iCloud tolerant.** Library root may be inside a sync provider. SQLite uses WAL; retries on `SQLITE_BUSY` are handled at the infra layer.
- **Deterministic builds.** `Cargo.lock` and `pnpm-lock.yaml` committed. No `"latest"` dependencies.
- **Determined shutdowns.** On `window::close`, polling and downloads drain gracefully (configurable deadline).

## 6. Concurrency model

- One Tokio runtime, multi-threaded scheduler.
- Long-lived tasks: `PollerService::run`, `DownloadService::run`. Each owns its own channel for commands.
- Short-lived work spawned via `tokio::task::spawn` where independent; CPU-bound work (e.g., thumbnail extraction in later phases) via `spawn_blocking`.
- No mutex held across an `.await`. Prefer `Arc<Mutex<State>>` with small critical sections or `tokio::sync::RwLock` for read-heavy paths.

## 7. Error model

- Library-level: `thiserror` enums per layer. Example: `infra::DbError`, `services::PollError`.
- IPC-level: `AppError` wraps lower-level errors and derives `Serialize`/`specta::Type` so the frontend receives a typed, discriminated union.
- No `panic!` / `unwrap()` / `expect()` outside test code. Rust clippy lints enforce this.

## 8. Observability

- Structured logging via `tracing` + `tracing-subscriber`.
- Log file: `$LOG_DIR/sightline.log`, rotated daily, bounded to 20 MB per file, keep 7.
- No remote telemetry. An opt-in diagnostic bundle export is a future consideration with its own ADR.

## 9. Platform notes

| Concern        | macOS                                | Windows                              | Linux                                |
| -------------- | ------------------------------------ | ------------------------------------ | ------------------------------------ |
| Config dir     | `~/Library/Application Support`      | `%APPDATA%`                          | `~/.config`                          |
| Log dir        | `~/Library/Logs`                     | `%LOCALAPPDATA%\Logs`                | `~/.local/state`                     |
| Keyring        | Keychain                             | Credential Manager                   | Secret Service (libsecret)           |
| Webview        | WebKit                               | WebView2 (Evergreen)                 | WebKitGTK                            |
| Tray           | Menu bar                             | System tray                          | StatusNotifierItem / X11 fallback    |

## 10. Versioning and compatibility

- App version follows SemVer starting from `0.1.0` (phase 1 tag).
- The on-disk schema has its own integer version, independent of app version. Migrations are forward-only; each has a rollback note in the file header.
- IPC contracts are versioned via command names (`health`, `health_v2`) only when breaking. Within a minor version we extend additively.

## 11. Security posture

- Capabilities in `src-tauri/capabilities/` grant the frontend the minimum set of Tauri APIs. No `fs:allow-*` for arbitrary paths.
- Secrets never land in logs (custom `Debug` impls that redact).
- Sidecars invoked with explicit argument lists; user input is never concatenated into a shell string.
- Updates: Phase 1 has no auto-update. A signed update channel ships in Phase 7 with its own ADR.

## 12. Open questions

- Bandwidth throttle: token bucket in-app vs. OS-level (macOS `pfctl`, Linux `tc`). Decide in Phase 3.
- Multi-View Sync: shared audio mix vs. per-pane only. Decide in Phase 6.
- Auto-update: sparkle vs. tauri-updater. Decide in Phase 7.
