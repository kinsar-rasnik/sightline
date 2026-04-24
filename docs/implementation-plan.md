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

## Phase 2 — Twitch ingest, metadata, chapters, polling

**Goal.** Poll the Twitch Helix API on a configurable adaptive interval, discover new VODs for followed streamers, fetch chapter metadata from the Twitch GraphQL endpoint, filter VODs by a game whitelist (default GTA V), enforce the live-stream gate, flag sub-only VODs, and surface the results in a minimal but real UI.

### Housekeeping first

- [x] Commit the Phase 1 working tree as logical Conventional Commits and tag `phase-1-complete`.
- [x] Wire typed IPC generation via `tauri-specta`; file [ADR-0007](adr/0007-ipc-typegen.md); regenerate bindings; update the `add-tauri-command` skill to reflect the new flow.

### Acceptance criteria

- [x] [ADR-0007](adr/0007-ipc-typegen.md) records the typed-IPC decision and drift-check flow.
- [ ] [ADR-0008](adr/0008-chapters-via-twitch-gql.md) records the decision to use the public Twitch GQL endpoint for chapters, with trade-offs and defensive-coding notes.
- [ ] Schema migrations `0002_streamers_vods_chapters.sql` and `0003_poll_log.sql` land reversibly and pass `PRAGMA user_version`.
- [ ] `AppConfig` persists Twitch Client ID + Secret via the OS keyring. Secrets never serialize to disk in plaintext; the frontend sees a masked `client_id` + a boolean `configured`.
- [ ] `HelixClient` handles App Access Token acquisition + refresh, conservative 600 points/min budget, exponential backoff on 401 (re-auth) and 429 (respect `Ratelimit-Reset`), and tests against a `wiremock`-mocked Helix.
- [ ] `GqlClient` fetches `VideoPlayer_VODSeekbarPreviewVideo` chapter moments with a hardcoded public Client-Id, defensive parsing, and fixture-based tests.
- [ ] `cmd_add_streamer(login)` resolves the user via Helix and registers them for polling; `cmd_remove_streamer` soft-deletes, preserving VOD history; `cmd_list_streamers` returns enriched rows (`vod_count`, `live_now`, `next_poll_eta`).
- [ ] `cmd_list_vods` supports filters (streamer_ids, status, game_ids, since/until), chronological sort by `stream_started_at`, paging.
- [ ] `cmd_get_vod(id)` returns the VOD plus its chapters.
- [ ] `cmd_trigger_poll` performs an ad-hoc poll that respects the global rate limit.
- [ ] `cmd_get_poll_status` returns per-streamer schedule state with `next_eta` + last result.
- [ ] `cmd_update_settings` / `cmd_get_settings` handle game filter, poll floor/ceiling, and credentials status (but not secrets).
- [ ] Polling scheduler runs on its own Tokio task, survives frontend close, honors an adaptive interval (10min live / 30min recent / 2h dormant) with ±10% jitter and a global concurrency cap; graceful shutdown on exit signal.
- [ ] `poll_log` table records every poll with counts + outcome.
- [ ] VOD ingest lifecycle (`pending → chapters_fetched → eligible | skipped_game | skipped_sub_only | skipped_live | error`) is invariant-tested end-to-end with a Helix+GQL double.
- [ ] Unit tests cover: duration parser (`1h23m45s`), chapter merger (gap fill), game-id matcher, poll-schedule decider, live-gate transition.
- [ ] Integration tests cover: happy-path Helix ingest + chapter merge + game filter; 401 → token refresh; 429 → backoff; cursor pagination; malformed GQL response; sub-only flag; first-backfill vs incremental-first-seen stop; polling scheduler with `tokio::time::pause` verifying intervals/jitter/cap.
- [ ] Frontend ships three real pages: Settings (credentials form with mask-on-save, game filter, interval sliders), Streamers (add-by-login, list with avatar / last-polled / VOD count / manual-poll / remove), Library stub (chronological list + filter chips + detail drawer with chapters).
- [ ] `security-reviewer` subagent passes the change set: no credentials logged, no credentials cross IPC after initial paste, keyring usage matches each OS.
- [ ] Docs freshness: `data-model.md`, `api-contracts.md`, and the README Quickstart are updated to match what landed.
- [ ] `docs/session-reports/phase-02.md` exists.
- [ ] Quality gate: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features && pnpm typecheck && pnpm lint --max-warnings=0 && pnpm test && pnpm build && pnpm tauri build --no-bundle`.

### Out of scope

- Downloading VOD video bytes — queue only (Phase 3).
- User-account OAuth, end-user Twitch login — perpetually out of scope (see tech-spec).
- Player, watch progress, multi-view, cleanup, installer signing.

---

## Phase 3 — Download engine, queue, library layout, storage hygiene

**Goal.** Orchestrate bundled yt-dlp + ffmpeg sidecars to download queued
VODs with per-item pause/resume, a persistent retryable queue, staged
writes + atomic move into a configurable library layout (Plex/Jellyfin
or flat), bandwidth throttle, quality preset + fallback, disk-space
preflight, and the UI surface to drive it all. Plays no video — the
player and watch-progress arrive in Phase 5.

### Housekeeping first (pre-feature)

- [x] Batch-merge the Dependabot backlog; defer majors that break (TS 6,
  vitest 4, jsdom 29) with notes in the phase report.
- [x] Wire the deferred `poll:started` / `poll:finished` emits in
  `services/poller.rs`, pipe them to a Zustand active-polls store, and
  render a per-row polling indicator. Test covers the emit path.
- [x] `#[specta(optional)]` on `SettingsPatch` / `VodFilters` so the
  frontend drops the `EMPTY_PATCH` spread (ADR-0009).
- [x] `swatinem/rust-cache@v2` on the `checks` CI job.
- [x] `scripts/verify.sh` + opt-in pre-push hook, documented in
  CONTRIBUTING.md as required before every push.

### Acceptance criteria

- [ ] **Sidecars bundled**: yt-dlp and ffmpeg binaries ship per-platform
  via `scripts/bundle-sidecars.sh`. Startup health-check invokes
  `yt-dlp --version`, compares against a pinned minimum, and records
  the outcome in the `HealthReport`. Optional auto-update setting
  `autoUpdateYtDlp` (default on); failures fall back to pinned and
  never block startup.
- [ ] **Migrations 0004 + 0005**: `downloads` table (vod_id PK, state,
  priority, quality_preset, quality_resolved, staging_path, final_path,
  bytes_total/done, speed_bps, eta_seconds, attempts, last_error,
  timestamps, pause_requested) and `library_migrations` table. Indexes
  on `downloads(state)` and `downloads(priority DESC, queued_at ASC)`.
  `PRAGMA user_version = 5`.
- [ ] **State machine**: `queued → downloading → (paused | completed |
  failed_retryable | failed_permanent)` with `paused → downloading`
  and `failed_retryable → queued` (max 5 attempts, exponential
  backoff). Pure-domain transition table with exhaustive unit tests.
- [ ] **`infra/ytdlp`**: `YtDlp` trait with `fetch_info` + `download`;
  real `YtDlpCli` parses `--progress-template '%(progress)j'` into
  typed progress events; `YtDlpFake` (behind `test-support`) for
  deterministic tests. Exit-code handling for 0/1/2/100/signal. Argv
  only, never shell.
- [ ] **`infra/ffmpeg`**: same trait/fake pattern; used only for remux
  (.ts → .mp4) and thumbnail capture (frame at 10% of the VOD).
- [ ] **Token-bucket throttle**: global `TokenBucket` re-fair-shared
  across workers as concurrency or setting changes; fairness + no
  leakage under concurrent load, property-tested. Applied by
  passing a per-worker `--limit-rate`; document the limitation in
  ADR-0010.
- [ ] **Queue service**: `services/download_queue.rs` owns an
  mpsc-command channel + worker pool. On startup, any row stuck in
  `downloading` resets to `queued` (partial staging file discarded).
  Commands: enqueue, pause, resume, cancel, retry, reprioritize.
  Events: `download:state_changed`, `download:progress` (throttled
  ≤ 2 Hz per download), `download:completed`, `download:failed`.
- [ ] **Library layout trait + two impls**:
  - `plex` — `<root>/<Display Name>/Season YYYY-MM/<Display> - YYYY-MM-DD - <Title> [twitch-<id>].{mp4, nfo, -thumb.jpg}`
    with Kodi-compatible NFO (`<movie>` + per-chapter tags +
    `uniqueid type="twitch"`).
  - `flat` — `<root>/<login>/YYYY-MM-DD_<id>_<slug>.mp4`, thumbnails
    in hidden `.thumbs/`.
  - Filename sanitizer: strips FAT32/exFAT/NTFS illegal chars, trims
    trailing dots/spaces, caps at 200 chars. Property-tested.
- [ ] **Layout migration**: changing `libraryLayout` triggers a
  background migrator that atomically moves (or copy+verify+delete
  across filesystems) every completed download to the new layout.
  Emits `library:migrating`. Recorded in `library_migrations`.
- [ ] **Staging + atomic move**: default staging under platform cache
  dir outside any sync provider. Flow: yt-dlp to staging → optional
  ffmpeg remux → thumbnail → atomic move (or copy+fsync+verify+
  delete) → sidecars written → DB updated. Stale staging files
  (> 48 h) cleaned on startup.
- [ ] **Disk-space preflight**: estimate from `filesize_approx`, check
  staging partition for size × 1.2 and library partition for size ×
  1.1 before enqueueing. Fail `failed_permanent` with reason
  `DISK_FULL` if either partition is short; emit
  `storage:low_disk_warning` (deduped per threshold).
- [ ] **Quality preset + fallback**: `source | 1080p60 | 720p60 | 480p`
  with a documented format-selector chain. If the preset can't be
  satisfied, fall back one step and record the resolved quality.
- [ ] **Tauri commands (thin)**: `cmd_enqueue_download`,
  `cmd_pause_download`, `cmd_resume_download`, `cmd_cancel_download`,
  `cmd_retry_download`, `cmd_reprioritize_download`,
  `cmd_list_downloads`, `cmd_get_download`, `cmd_get_staging_info`,
  `cmd_get_library_info`, `cmd_migrate_library`,
  `cmd_get_migration_status`. All ≤ 20 lines, `#[specta::specta]`.
- [ ] **Frontend**: new `/downloads` route with live-updated table
  (thumbnail, title, streamer, state, progress, speed, ETA,
  actions); Library row badges + primary-action button reflecting
  download state; Settings gets "Downloads & Storage" section
  covering concurrency, bandwidth limit, quality preset, library
  root + layout selector with live preview, staging path, auto-
  update toggle, disk info. Layout switch triggers a confirmation
  dialog + the migrator.
- [ ] **ADRs**: 0010 bandwidth throttle approach, 0011 library layout
  pluggability, 0012 staging + atomic move strategy.
- [ ] **Docs**: `data-model.md` + `api-contracts.md` updated;
  `docs/user-guide/library-layouts.md` covers Plex vs flat; README
  Installation notes the bundled sidecars, Roadmap flips Download
  engine to ✅.
- [ ] **Security review** (subagent) pass: no shell injection, no path
  traversal, atomic-move semantics correct, no client_id leak into
  files on disk.
- [ ] **Cross-platform**: path-length budget tested with a long Proton
  Drive prefix on Windows. SQLite WAL confirmed. Sidecar names match
  Tauri's `<name>-<target>` convention per platform.
- [ ] **Observability**: tracing spans on every download + migration +
  subprocess invocation, `vod_id` as a span field. Optional
  rotating debug log under `~/.local/state/sightline/logs/`.
- [ ] `docs/session-reports/phase-03.md` exists.
- [ ] Quality gate (via `scripts/verify.sh`) green; tag
  `phase-3-complete`.

### Out of scope (stays for later phases)

- Player, resume-from-position, watch progress — Phase 5.
- Multi-view sync — Phase 6.
- Auto-cleanup policies, mark-as-watched UI, cross-streamer timeline
  overhaul — Phase 5 or 7.
- Installer signing / notarization — Phase 7.

---

## Phase 4 — Tray daemon, timeline foundation, UI polish, sidecar bundling

**Goal.** The app is usable headlessly, has a first-cut chronological
timeline across streamers, ships a polished Plex-grade library grid +
detail drawer, and — finally — bundles real yt-dlp + ffmpeg binaries
with verified hashes. Scope deliberately stops short of the player
(Phase 5) and multi-view sync engine (Phase 6).

### Housekeeping first

- [x] Dependabot sweep (no open PRs — Phase 3 cleaned the backlog).
- [x] Real sidecar bundling: pinned URL + SHA-256 per platform in
  `scripts/sidecars.lock`; bash + PowerShell scripts that verify hash
  BEFORE extraction; `scripts/verify-sidecars.sh` used by pre-push +
  CI + runtime; `tauri.conf.json` `externalBin` + `build.rs`
  `TARGET_TRIPLE`; CI matrix step that runs real binaries on all
  three OS; end-to-end smoke test in `tests/sidecar_smoke.rs`;
  [ADR-0013](adr/0013-sidecar-bundling.md) documents the design,
  source choices, alternatives, and refresh procedure.
- [x] `scripts/verify.sh` now invokes `verify-sidecars.sh` as the
  first gate step (`--no-sidecars` flag for fresh clones).

### Acceptance criteria

#### Tray / menu-bar daemon mode

- [ ] Tauri tray plugin wired on all three OS. Menu: summary row +
  Open Sightline, Pause all, Resume all, Quit.
- [ ] Close button hides the window; the Tokio services (poller +
  download queue) keep running. A one-time toast on the first close
  explains the new behavior and offers "quit on close" override.
- [ ] `cmd_set_window_close_behavior` persists `hide | quit`.
- [ ] Explicit Quit: broadcasts `app:shutdown_requested`, poller
  stops mid-cycle, download queue signals workers to pause + flush
  progress, 10 s timeout, then process exits cleanly. Integration
  test kills mid-download and verifies DB consistency on restart.
- [ ] Optional `startAtLogin` setting (default off) registers via
  Tauri's autostart plugin (LaunchAgent / Registry / XDG).

#### Timeline foundation

- [ ] Migration 0006: `stream_intervals` table with vod/streamer
  FKs, start/end, and range + streamer indexes. `PRAGMA user_version = 6`.
- [ ] `domain/timeline.rs` — pure: `Interval`,
  `overlapping(a, b) -> Option<Interval>`, `bucket_by_day`,
  `find_co_streams(around, all)`. Property-tested with proptest over
  thousands of random intervals.
- [ ] `services/timeline_indexer.rs` — subscribes to `vod:ingested`
  and upserts. First-launch backfill (full rebuild when
  `stream_intervals` empty but `vods` populated) with progress events.
- [ ] `cmd_list_timeline` / `cmd_get_co_streams` /
  `cmd_get_timeline_stats` / `cmd_rebuild_timeline_index`. Events:
  `timeline:index_rebuilding` / `timeline:index_rebuilt`.

#### Favorites (migration 0007)

- [ ] `streamers.favorite` column (default 0).
- [ ] `cmd_toggle_streamer_favorite({ streamer_id })`. Events
  `streamer:favorited` / `streamer:unfavorited`.

#### Library / home screen polish

- [ ] `/library` flips from list → virtualized grid (16:9 cards).
  Thumbnail, title, streamer + date, status badge, hover preview
  cycling the 6 extracted frames. Quick-action overlay for
  Play (disabled pending Phase 5), Mark watched (disabled pending
  Phase 5's watch-progress), Open detail, More menu.
- [ ] Slide-in detail drawer from the right: hero, full metadata,
  chapters-as-timeline with GTA V highlighted, co-streams panel
  (uses `cmd_get_co_streams`), download state + actions.
- [ ] Filter/sort bar: chip-style streamer + status + game + date
  range; sort: stream_start desc/asc, added desc, duration.
  Fuzzy search over title + streamer.
- [ ] URL-synced filter state (query params).
- [ ] Design tokens documented in `docs/design-tokens.md`. Dark-mode
  first with a fully-implemented light theme.
- [ ] First-run empty state (3-step checklist).

#### Timeline UI

- [ ] `/timeline` horizontal time-axis view, zoomable
  (day / week / month). Streamer lanes; overlaps visually obvious.
  Click a bar → detail popover with jump buttons to overlapping
  VODs. Filter chips. Keyboard-shortcuts. Virtualised viewport
  (budget: 5 years × 20 streamers scrolls at 60 fps).

#### Settings reorganization

- [ ] Sections: Twitch, Polling, Downloads & Storage, Library,
  Appearance (tokens), Advanced (shortcuts + autostart +
  window-close behavior + diagnostics).
- [ ] Shortcut customization UI. `cmd_set_shortcut`,
  `cmd_reset_shortcuts`.

#### Accessibility + keyboard navigation

- [ ] Visible focus rings on every interactive element.
- [ ] ARIA landmarks + live regions for polling/download updates.
- [ ] Global shortcuts (customizable): `g l` / `g t` / `g d` /
  `g s` / `g ,`, `/` focus search, `n` add streamer, `p` pause all,
  `?` shortcut help overlay, `Cmd/Ctrl+Q` quit.
- [ ] `@axe-core/react` in dev + `pnpm a11y` script.

#### Notifications

- [ ] Tauri notification plugin wired. Categories: download
  completed, download failed (always), new VODs from favorites,
  storage low.
- [ ] Coalesce per-category rate limiting — a 20-VOD ingest
  produces one notification, not 20.
- [ ] Global + per-category toggles in Settings.

#### Observability

- [ ] Startup timing breakdown logged at `debug` (migrate, poller,
  queue, indexer warmup, window show).
- [ ] Performance marks via the Performance API on heavy routes.

#### Security review

- [ ] Subagent pass on tray menu actions, autostart registration,
  shortcut customization JSON, and sidecar download scripts
  (checksum-before-execute is non-negotiable — no code runs from
  the downloaded binary before hash verification).

#### ADRs

- [x] [ADR-0013](adr/0013-sidecar-bundling.md) — sidecar bundling.
- [ ] ADR-0014 — tray / daemon architecture.
- [ ] ADR-0015 — timeline data model + indexer.

#### Docs

- [ ] `docs/user-guide/getting-started.md` refresh.
- [ ] `docs/user-guide/timeline.md`.
- [ ] `docs/user-guide/tray-mode.md`.
- [ ] `docs/design-tokens.md`.
- [ ] README Roadmap flips Tray / Timeline / UI polish / Sidecars
  to ✅.
- [ ] `docs/session-reports/phase-04.md`.

### Out of scope (stays for later phases)

- Player, resume-from-position, watch progress — Phase 5.
- Multi-view sync engine — Phase 6.
- Auto-cleanup, sub-only handling, release signing — Phase 7.
- Localization beyond English — post-1.0.

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
