# Changelog

All notable changes to Sightline. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
