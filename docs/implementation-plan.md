# Sightline — Phased Implementation Plan

> **Audience.** The Senior Engineer (Claude) and the CTO. The CEO reads the roadmap in `README.md`; this document is the operational version.

> **Discipline.** A phase is complete only when the acceptance criteria are checked, the quality gate is green, and a session report is filed under `docs/session-reports/phase-NN.md`. See [synthetic-workforce-blueprint.md §7](reference/synthetic-workforce-blueprint.md).

---

## Phase 1 — Foundation

**Goal.** A runnable `pnpm tauri dev` skeleton with a health-check window, the full synthetic workforce in place, and all decisions captured. No Twitch calls, no yt-dlp logic, no real UI beyond a splash/health panel.

### Acceptance criteria

- [x] Repo contains `LICENSE`, `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `CLAUDE.md`, and `.gitignore`.
- [x] `docs/reference/synthetic-workforce-blueprint.md` is canonical and referenced from CLAUDE.md.
- [x] Six ADRs exist under `docs/adr/` covering: stack, persistence, sidecars, IPC, polling model, package manager.
- [x] Technical specification, data model, and IPC contracts are drafted under `docs/`.
- [x] `.claude/` contains agents, hooks, rules, skills, and settings consistent with the blueprint.
- [x] `.github/` contains a CI workflow with a macOS/Windows/Linux matrix plus issue and PR templates.
- [x] `src-tauri/` compiles and produces a window on `pnpm tauri dev`.
- [x] A single `health` IPC command round-trips, returning the app version, schema version, and a timestamp.
- [x] SQLite database initializes and runs a no-op migration on startup.
- [x] Quality gate passes: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && pnpm typecheck && pnpm lint && pnpm test`.
- [x] `docs/session-reports/phase-01.md` exists.

### Out of scope

- Twitch API integration (Phase 2).
- VOD download, queue, throttle (Phase 3).
- Real library UI, settings dialogs, tray (Phase 4).
- Player, watch-progress tracking (Phase 5).
- Multi-View Sync (Phase 6).
- Auto-cleanup, sub-only handling, release polish (Phase 7).

---

## Phase 2 — Twitch ingest and polling

**Goal.** Poll the Twitch Helix API on a configurable interval, discover new VODs for followed streamers, filter by game whitelist, and record them locally.

### Acceptance criteria

- [ ] `AppConfig` persists Twitch Client ID + Secret via the OS keyring (never plaintext on disk).
- [ ] `HelixClient` handles App Access Token acquisition, refresh, rate-limit awareness, and exponential backoff on 5xx.
- [ ] `follow_streamer(login)` resolves to a `user_id` + profile blob and enqueues for polling.
- [ ] The `Poller` task runs on a schedule, batches requests, and respects live-state (live streams are deferred).
- [ ] New VODs landing on disk trigger a `vod:discovered` event on the IPC bus.
- [ ] Integration tests cover: 200 OK, 401 (token refresh), 429 (backoff), malformed payload, empty VOD list.
- [ ] `docs/adr/000N-helix-client-design.md` documents the client contract.

### Out of scope

- Downloading VOD video bytes — queue only.

---

## Phase 3 — Download engine

**Goal.** Orchestrate yt-dlp as a sidecar to download queued VODs with per-item pause/resume, bandwidth throttle, and configurable quality preset.

### Acceptance criteria

- [ ] `DownloadManager` owns a bounded concurrency pool (default 2).
- [ ] Per-download state machine: `queued → downloading → paused → completed | failed`.
- [ ] Progress events stream to the frontend at 1 Hz.
- [ ] Global bandwidth throttle honored across all active downloads.
- [ ] Retry policy: 3 attempts with exponential backoff, then mark failed.
- [ ] Sub-only VODs are detected and flagged, not silently retried.
- [ ] ADR on the yt-dlp invocation contract.

### Out of scope

- UI beyond the existing placeholder.

---

## Phase 4 — Library UI, settings, tray

**Goal.** The app is usable: a library grid ordered by `stream_started_at`, a settings dialog for credentials and preferences, and a menu-bar / system-tray mode that keeps polling alive with the window closed.

### Acceptance criteria

- [ ] Library grid with virtualized rows, sorted by `stream_started_at` descending.
- [ ] Streamer follow/unfollow flow.
- [ ] Settings panel: Twitch credentials, game whitelist, polling interval, download defaults.
- [ ] Tray icon with actions: open, pause all, quit. Close-to-tray on all platforms.
- [ ] Accessibility: keyboard navigation for library and settings, screen-reader labels, prefers-reduced-motion honored.

---

## Phase 5 — Player and watch progress

**Goal.** Play downloaded VODs locally with resume-from-position, mark-as-watched, and chapter scrubbing.

### Acceptance criteria

- [ ] Player (native `<video>` + custom controls) with keyboard shortcuts, variable playback rate, volume, fullscreen.
- [ ] Watch progress persists on pause, close, seek.
- [ ] Chapters surfaced in the scrubber.
- [ ] Mark-as-watched sets a flag and optionally enqueues auto-cleanup.

---

## Phase 6 — Multi-View Sync

**Goal.** Open two VODs side-by-side locked to a shared wall-clock time.

### Acceptance criteria

- [ ] Select two VODs from the library; both open in a split view.
- [ ] Seek on one updates the other, preserving the wall-clock offset.
- [ ] Volume and playback-rate controls are per-pane.
- [ ] Audio mix controls (mute one side, crossfade).

---

## Phase 7 — Polish and v1.0

**Goal.** Ship-ready release: auto-cleanup policies, sub-only handling end-to-end, installer signing on all platforms, accessibility and localization pass.

### Acceptance criteria

- [ ] Auto-cleanup (24h / 7d / 30d / off) configurable per streamer.
- [ ] Sub-only VODs show a distinct state with guidance, never fail silently.
- [ ] Code-signed installers on macOS and Windows; AppImage for Linux.
- [ ] English strings centralized in a catalog for future localization.
- [ ] v1.0 release checklist followed.

---

## Cross-cutting tracks

These run alongside every phase and are owned by the Senior Engineer:

- **Dependency hygiene.** Dependabot configured; `cargo audit` and `pnpm audit --prod` run on CI. Blocking vulnerabilities escalate to the CTO.
- **Performance budgets.** Startup under 2s on a mid-range laptop; poll cycle under 500ms CPU per 20 streamers; UI at 60 fps on library scroll.
- **Docs freshness.** Every public IPC command has a doc comment. Every phase bumps `docs/session-reports/phase-NN.md`.
- **ADR hygiene.** When a design choice is made implicitly in code, open an ADR to make it explicit.

---

## Out of perpetual scope

Documented here so nobody has to relitigate them:

- **Cloud sync of user data.** Local-first is a principle, not a phase. A future cloud feature needs an explicit ADR with a threat model.
- **Bundled Chromium.** We ride the OS webview. That is the reason Tauri was chosen over Electron; see ADR-0001.
- **Twitch account login (OAuth code flow).** We use App Access Tokens only. End-user OAuth would change the privacy posture materially; that is an explicit non-goal.
