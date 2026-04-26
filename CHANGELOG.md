# Changelog

All notable changes to Sightline. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.2] — 2026-04-26

> **Critical hotfix.**  v2.0.1 macOS `.dmg` shipped with a broken sidecar resolver — the encoder-detection step on first launch failed with `sidecar: spawn: No such file or directory (os error 2)`.  v2.0.2 fixes the path resolution so the bundled `ffmpeg` and `yt-dlp` are actually found at runtime.  No breaking changes, no schema migrations, no setting renames.

### What was wrong

The bundled sidecars (`ffmpeg-aarch64-apple-darwin`, `yt-dlp-aarch64-apple-darwin`) ship inside the `.app` at `Contents/MacOS/` next to the main binary — that's the Tauri 2 layout.  The runtime resolver was still using the Tauri 1 convention and looking under `Contents/Resources/`, which doesn't contain the sidecars.  Result: the resolver returned `None`, the caller fell through to invoking `ffmpeg` / `yt-dlp` on PATH, and on a typical macOS install those binaries don't exist.  Same root cause on Linux + Windows; macOS surfaced first because the encoder-detection probe runs the moment Settings → Video Quality opens.

### Fix

- `resolve_sidecar` now probes `current_exe().canonicalize().parent()` first (the canonical Tauri 2 sidecar location across macOS `.app` / Linux `.deb` + `.AppImage` / Windows `.msi` + `-setup.exe`), then falls back to `BaseDirectory::Resource` for forward-compat, then to the repo-relative `src-tauri/binaries/` path for `pnpm tauri dev`.
- Pure helper `find_sidecar_in_dir` extracted as `pub` so the bundle layout invariants are independently testable without a Tauri runtime.
- Canonicalisation handles AppImage's FUSE-mounted squashfs symlink correctly; falls back to the raw path on canonicalize failure (benign on macOS / Windows).

### CI coverage gap closed

`src-tauri/tests/sidecar_smoke.rs` now contains 8 bundle-layout-simulation tests — one per OS-bundle layout we ship, plus precedence + missing + unicode-path edge cases.  These run on every CI matrix job (macos-latest / windows-latest / ubuntu-latest) without needing a real `pnpm tauri build`.  Catches the regression class that v2.0.0 + v2.0.1 missed because the existing smoke tests probed `src-tauri/binaries/` directly, not the bundled-app resolution path.

### Documentation

- New [ADR-0034](docs/adr/0034-tauri2-sidecar-layout.md) documents the Tauri 2 bundle layout per OS, the canonicalize() rationale, and the alternatives considered (notably `tauri-plugin-shell::Command::new_sidecar` as a v2.1 follow-up).  ADR-0013's "Runtime integration" subsection carries a partial-supersession banner.
- [docs/INSTALL.md](docs/INSTALL.md) macOS section rewritten — `xattr -d com.apple.quarantine` is now the required first-launch step (right-click → Open is hidden on Sequoia 15.3+), the "App is damaged" Gatekeeper message is explained as a missing-signature confusion not real corruption, and a troubleshooting block includes sidecar-presence verification.

### Schema

No new migrations.  Schema version stays at **17**.

### Reverting

This is a runtime-only fix.  Downgrading to v2.0.1 reproduces the bug; there's no reason to revert.  If you must, the schema is unchanged from v2.0.

## [2.0.1] — 2026-04-26

> **Scope-closure point release.**  Closes the AC9, AC10, AC8 and AC5 surfaces deferred from v2.0.  No breaking changes; no new schema migrations; no setting renames.  Existing v2.0 installs upgrade silently.

The "storage-aware" v2.0 release shipped the backend substance for pull-on-demand, the quality pipeline, and adaptive CPU throttling — but four user-facing surfaces were deferred under tight scope pressure (AC9 storage forecast UI, AC10 unified library UI, AC8 pre-fetch player wiring, AC5 Windows CPU suspend).  v2.0.1 closes those gaps so v2.0 finally meets its spec end-to-end.

### Highlights

- **Storage forecast UI** (AC9, ADR-0032).  Streamers → Add now renders a per-streamer forecast box (weekly downloads / peak disk / Green / Amber / Red watermark indicator) immediately after a successful add.  Settings → Storage Outlook shows the combined forecast across every active streamer plus a per-streamer breakdown for spotting the disk-pressure driver.  Math is the same `quality_factor_gb_per_hour` table that v2.0 shipped — v2.0.1 hooks it up to the renderer.
- **Unified Library UI** (AC10, ADR-0033).  The library page now renders every non-deleted VOD with status-aware visual differentiation (opacity tiers 60/70/80/100/100/50 % and a lifecycle badge) plus hover-revealed quick actions: Download / Cancel / Play / Re-watch / Remove / Pick again.  Filter chips replace the legacy ingest-status chips: All / Not downloaded / Downloaded / Watched.  Distribution events bust the vods cache so badges update without a manual refetch.
- **Pre-fetch player wiring** (AC8, ADR-0031).  The player now invokes `prefetchCheck` once per VOD per app session when watch progress crosses 70 % or remaining time falls below 120 s.  Throttled module-side via a `Set<vodId>` so the 5-second progress cadence cannot fan out into a thousand round-trips, and short-clip edge cases (≤ 90 s clip opened at currentSeconds = 0) are guarded behind a 5 s minimum-watched threshold.
- **Windows CPU suspend** (AC5, ADR-0029).  `NtSuspendProcess` / `NtResumeProcess` via PowerShell + `Add-Type` P/Invoke — same shell-out idiom as the existing `wmic`-based priority lower, no `unsafe_code` lint relaxation needed.  `default_suspend_controller()` now returns `WindowsSuspend` on `cfg(windows)` instead of the no-op fallback.
- **Stale-PID guard for SuspendController** (Phase 8 medium finding).  Every suspend / resume now probes `is_process_alive` before issuing the OS primitive; an ffmpeg child that completed between the throttle decision and the controller invocation is a benign no-op rather than a crash.  Locale-independent ESRCH detection on the Unix side (re-probes via `kill -0` instead of stderr-string matching).
- **Download-worker convergence** (deferred from Phase 8 final report).  `enqueue` and the worker's state transitions now mirror onto `vods.status` so the legacy `downloads.state` machine and the new lifecycle column agree on every row.  Pull-mode picks bridge into the worker via the `distribution_sink::VodPicked` handler (5 s tick latency on first download, accepted).

### Quality + scope-discipline

- New rules introduced before the run: **R-SC-01** (Scope-Reduction-Approval), **R-SC-02** (AC-Vollständigkeits-Check), **R-SC-03** (Versions-Manifest-Konsistenz).  See [`.claude/rules/scope-control.md`](.claude/rules/scope-control.md).
- 7 sub-phases, 5 R-RC-01 mid-phase reviews, 5 R-RC-02 re-reviews — all CLEAN.  **0 P0/P1 findings** unresolved at end-of-phase.
- 469 backend tests + 152 frontend tests, all green.

### IPC

3 new commands (no breaking changes to existing surface):
- `prefetchCheck` — invoked by the player at the 70 % progress threshold.
- `removeVod` — user-initiated remove on a downloaded VOD; transitions ready/archived → deleted and unlinks the file.
- `estimateStreamerFootprint` / `estimateGlobalFootprint` — drives the forecast UI.

The `Vod` IPC type now carries `status: VodStatus` so the renderer can render per-card lifecycle badges + filter chips without a parallel query.

### Schema

No new migrations.  Schema version stays at **17**.

### Reverting

Revert is `git revert` of the v2.0.1 commits + downgrade the binary; the schema is unchanged from v2.0 so a v2.0 binary still reads the same database.

### Limitations / known follow-ups

- Per-streamer quality overrides — v2.1.
- AV1 hardware encoders — post-2026 install-base catch-up.
- Auto-Issue-Triage workflow — separate follow-up project.
- Live-update forecast on settings changes — v2.1.
- Refactor `SuspendController` impls into `infra::process::suspend` — v2.1 cleanup.

## [2.0.0] — 2026-04-26

> **BREAKING for new installs.** Pull-on-demand is the new default; metadata polling no longer auto-enqueues every newly-discovered VOD.  Existing v1.0 installs are preserved on auto-download mode by the migration's backwards-compat detection — flip Settings → Distribution → Mode at any time.  See [`docs/MIGRATION-v1-to-v2.md`](docs/MIGRATION-v1-to-v2.md).

The "storage-aware" release.  v1.0's auto-download model didn't scale for hobbyist disks (a heavy GTA-RP watcher accumulated 200 GB / week with 5 streamers at source quality).  v2.0 introduces explicit pull-on-demand with a per-streamer sliding window, a 720p30 H.265 default that cuts steady-state disk use by ~9×, and a hardware-encode-first re-encode pipeline that doesn't fight your game for CPU.

### Highlights

- **Pull-on-demand distribution model** (ADR-0030).  Polling now produces `available` VOD rows; the user explicitly picks what to download (or pre-fetch picks the next chronological VOD on the streamer they're watching, ADR-0031).  Sliding window default `N=2` per streamer (range 1..=20) bounds steady-state disk use to a known constant.  Auto-download stays available as Settings → Distribution → Mode → "Auto-download (legacy)" — and is the default for v1.0 upgrades.
- **720p30 H.265 default for new installs** (ADR-0028).  Hardware-encode-first detection: VideoToolbox on macOS, NVENC > AMF > QuickSync on Windows, VAAPI on Linux.  Software fallback (libx265) is **opt-in** with an explicit "may saturate CPU during gaming" warning.  Audio is **never** re-encoded — `-c:a copy` invariant guarded by a structural regression test.
- **Background-friendly re-encode** (ADR-0029).  Two-layer policy: `nice +19` / `BELOW_NORMAL_PRIORITY_CLASS` at spawn, plus an adaptive throttle that suspends ffmpeg (Unix `kill -STOP/-CONT`) when sustained CPU load exceeds the high threshold (default 0.7) for 30 s and resumes when it drops below the low threshold (default 0.5) for 30 s.  Windows `SuspendThread` integration ships in v2.1; on v2.0 Windows the priority drop is the only active layer.
- **Pre-fetch hook** (ADR-0031).  When you watch VOD K, Sightline picks K+1 in the background — at most one chronological lookahead per streamer, bounded by the sliding-window cap.
- **Storage-aware Settings UI**.  New "Video Quality" section with example math per profile (e.g. *"720p30 — 6 h ≈ 1.4 GB, downloads in 4 min at 50 Mbit/s"*), hardware-encoder status with re-detect button, software opt-in with explicit warning, advanced sliders for concurrency + throttle thresholds.

### Phase milestones

- **Phase 8 — Storage-aware distribution (this release).**  Quality pipeline + pull model + sliding window + new ADRs 0028–0033.

### Schema

Schema version **17**.  Migrations 0015 (quality settings), 0016 (vod status machine + backfill from existing rows), 0017 (distribution settings + backwards-compat detection).  Forward-only and idempotent.  Reverting to v1.0 is **not supported** — `PRAGMA user_version` is monotonic.

### IPC surface additions

- 3 quality commands (`getEncoderCapability`, `redetectEncoders`, `setVideoQualityProfile`) + `EncoderCapability` / `VideoQualityProfile` / `EncoderKind` types.
- 5 distribution commands (`pickVod`, `pickNextN`, `unpickVod`, `setDistributionMode`, `setSlidingWindowSize`) + 4 events (`distribution:vod_picked`, `:vod_archived`, `:prefetch_triggered`, `:window_enforced`).

### Known limitations / post-2.0

These ship as deferred follow-ups for v2.0.x point releases — none are blocking the v2.0 release:

- **v2.0.x download-worker integration.**  Picking a VOD transitions `vods.status` to `'queued'`; in v2.0, the existing Phase 3 download service still drives `downloads.state` independently.  Both end-states are correct; the integration in v2.0.x makes `vods.status` the single source of truth and lets the worker observe it directly.
- **v2.0.x storage forecast UI.**  The math is in `domain/quality.rs::quality_factor_gb_per_hour` (ADR-0032).  The "before-streamer-add" forecast UI ships in v2.0.1.
- **v2.0.x library UI re-conception** (ADR-0033).  v2.0 ships the new Distribution Settings + Video Quality settings tabs.  The unified library card design (filter chips, per-VOD quick actions, status badges) ships in v2.0.1.
- **v2.1 Windows ffmpeg suspend.**  The throttle decision logic ships in v2.0; the actual `SuspendThread` integration on Windows is deferred.
- **All Phase 7 limitations carry forward.**  Unsigned binaries, Apple-Silicon-only macOS published binary, no self-update, two-pane multi-view only, no package-manager distribution.

[2.0.0]: https://github.com/kinsar-rasnik/sightline/releases/tag/v2.0.0

---

## [1.0.0] — 2026-04-25

The first public release. Sightline is a local-first desktop app that aggregates Twitch GTA-RP VODs across streamers into a single chronological library, downloads them via a bundled `yt-dlp` + `ffmpeg`, and plays them back with optional two-pane wall-clock-synchronized multi-perspective playback.

Distributed via [GitHub Releases](https://github.com/kinsar-rasnik/sightline/releases) for macOS (Apple Silicon), Windows (x64 MSI + NSIS), and Linux (AppImage + deb). Unsigned binaries; see [`docs/INSTALL.md`](docs/INSTALL.md). Intel Mac users build from source — GitHub retired the macos-13 runner in late 2025.

### Highlights

- **Multi-streamer ingestion** with adaptive polling intervals per streamer (10 min when live, 30 min when recently live, 2 h when dormant). Helix App Access Token flow — no end-user OAuth, credentials live in the OS keyring.
- **Chapter-aware filtering** via the public Twitch GraphQL endpoint. Only VODs containing your configured games (default GTA V, id `32982`) become eligible. Live, sub-only, and game-mismatched VODs are flagged but never silently failed.
- **Download engine** with up to 5 parallel workers, global bandwidth throttle, configurable quality preset (source / 1080p60 / 720p60 / 480p), automatic fallback, and atomic-move with SHA-256 verify on cross-filesystem destinations.
- **Two library layouts**: Plex/Jellyfin (`<Streamer>/Season YYYY-MM/`) with NFO + thumbnail sidecars, or Flat (single file per VOD). Layout switches run a background migrator with progress events.
- **Tray / menu-bar daemon** that survives a window close, drains gracefully on Cmd/Ctrl-Q, and surfaces "active downloads" + "next poll ETA" in the tooltip.
- **Timeline view** ranking events chronologically across streamers, with co-streamer cross-links and a deep-link math that opens any VOD at the matching wall-clock offset.
- **Player** with resume-from-position (0.5 s rounded, 5 s flush cadence), `manuallyWatched` toggle, completion threshold slider (70–100 %), and a Continue Watching row.
- **Multi-View Sync v1**: open two VODs side-by-side, lock to wall-clock, drift-correct on a 250 ms cadence (configurable), per-pane volume + mute, group-wide play / pause / seek / speed. Out-of-range followers auto-pause with an overlay; the leader keeps playing.
- **Auto-cleanup service**: opt-in disk-pressure relief with two watermarks, daily schedule, and a dry-run preview of every plan. Watch progress survives re-downloads. Audit log of every run feeds a History view in Settings.
- **Update checker** (opt-in, off by default): once-per-24h GET to the GitHub Releases API. No telemetry; no IDs; only outbound traffic is the version-check itself. Per-version Skip and Remind-me-later affordances on the in-app banner.
- **Asset-protocol scope narrowed** to the configured library root (defence-in-depth on top of the Phase-5 service-layer guard).
- **`cargo audit` is now a blocking CI gate** with a documented allow-list (`src-tauri/audit.toml`) carrying owner + expiry per accepted exception.

### Phase milestones

- **Phase 1 — Foundation.** Repo skeleton, Tauri 2 + Rust + React/TS stack, sqlx + WAL, typed IPC via tauri-specta, ADR-0001..0007.
- **Phase 2 — Twitch ingest.** Helix client with App Access Token flow, GraphQL chapters fetch, adaptive poller, ingest pipeline, ADR-0008.
- **Phase 3 — Download engine.** yt-dlp orchestration, queue, throttle, library layouts, atomic moves, library migrator, ADR-0009..0013.
- **Phase 4 — Tray daemon + UI polish.** Tray + headless mode, timeline data model + indexer, sidecar bundling with SHA-256 verify, shortcuts service, ADR-0014..0017.
- **Phase 5 — Player + watch progress.** Resume math, watch state machine, asset-protocol guard, autostart sync, ADR-0018..0020.
- **Phase 6 — Multi-View Sync.** Wall-clock-locked split view, drift correction, leader election, group-wide transport, ADR-0021..0023.
- **Phase 7 — Release + capstone (this release).** Auto-cleanup, GitHub-Releases pipeline, opt-in update checker, scope narrowing, audit hardening, ADR-0024..0027.

### Schema

Schema version `14`. Migrations 0001..0014 are forward-only and append to `<library_root>/sightline.sqlite`.

### Known limitations / post-1.0

These are tracked but do not block v1.0:

- macOS / Windows binaries are **unsigned**. Re-evaluate when there's funding.
- macOS published binary is **Apple Silicon only**. GitHub-Actions retired the macos-13 hosted runner in late 2025; Intel Mac users build from source. A future paid-tier or self-hosted runner can re-introduce the published x64 target.
- No self-update mechanism. The opt-in checker surfaces availability and links to the release page; downloading + installing is manual.
- Multi-View v1 is two panes only. PiP, >2 panes, crossfader, and shareable-sync URLs are tracked for v2.
- Per-pane volume / mute is per-session, not persisted across sessions.
- Playwright + tauri-driver E2E coverage for the player is deferred (Vitest covers the unit-level surfaces).
- No package-manager distribution (Homebrew tap, winget manifest, AUR package).

[1.0.0]: https://github.com/kinsar-rasnik/sightline/releases/tag/v1.0.0
