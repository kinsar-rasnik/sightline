# Sightline

> A cross-platform desktop app for watching multi-streamer GTA Roleplay events on one unified, chronological timeline — with synchronized multi-perspective playback.

<p align="center">
  <em>Status: Phase 1 — foundation. Pre-alpha. No binaries yet.</em>
</p>

---

## Why

If you follow GTA-RP on Twitch (NoPixel, GTA World, community servers), you probably follow 3–20 streamers who share the same in-world event. When a heist or shootout happens, every participant's VOD is its own shard of the same story — but Twitch gives you no way to find them together, and no way to watch them in sync.

Sightline does exactly that:

- **Aggregates** VODs from every streamer you follow.
- **Orders them by actual stream start time**, not publish time — so a delayed upload still lands in the right place on the timeline.
- **Filters to GTA V only** (configurable whitelist), so non-RP content doesn't clutter the library.
- **Syncs playback** across two VODs to a shared wall-clock, so you can watch a heist from two perspectives at once.

All data stays on your machine. No account required. No telemetry.

---

## Features

- **Follow streamers** by Twitch login name; fetches display name, avatar, and metadata.
- **Background polling** for new VODs (configurable interval). Runs as a tray / menu-bar app — close the window, the daemon keeps working.
- **Chapter-aware filtering.** Only VODs containing a `Grand Theft Auto V` chapter (or your custom whitelist) are queued.
- **Live-stream-safe.** A VOD is only eligible for download once the stream has ended.
- **Delayed-release tolerant.** VODs published days after the stream still land in the correct chronological slot.
- **Download engine** (yt-dlp sidecar) with per-VOD pause/resume, global bandwidth throttle, configurable quality preset, automatic fallback.
- **Library** ordered by `stream_started_at` (UTC) across all streamers.
- **Player** with resume-from-position, mark-as-watched, chapter scrubber.
- **Multi-View Sync.** Open two VODs side-by-side, lock them to shared wall-clock time, seek one and the other follows.
- **Auto-cleanup** of watched VODs (24h / 7d / 30d / off).
- **Sub-only detection** — clearly flagged, never silently failed downloads.
- **Proton Drive–friendly.** The library root can live under any sync provider; the app handles temporary file locks gracefully.

---

## Screenshots

> _Placeholder — will be added once Phase 4 (UI) ships._

---

## Installation

### Pre-built binaries

Pre-built installers (macOS `.dmg`, Windows `.msi`, Linux `.AppImage`) will be published to the [GitHub Releases](../../releases) page starting at **v0.2.0**. Phase 1 is source-only.

### Build from source

```bash
# Prerequisites: Rust 1.90+, Node 20+, pnpm 9+, platform Tauri deps
# (see https://v2.tauri.app/start/prerequisites/)

git clone https://github.com/<your-fork>/sightline.git
cd sightline
pnpm install
pnpm bundle-sidecars            # downloads pinned yt-dlp + ffmpeg
pnpm tauri dev
```

For a production build:

```bash
pnpm tauri build
```

---

## Quickstart

1. **Get Twitch API credentials.** Register an app at <https://dev.twitch.tv/console/apps>, grab the **Client ID** and generate a **Client Secret**. The redirect URL can be anything (we don't use OAuth — only App Access Tokens).
2. **Launch Sightline.** On first run, open **Settings → Twitch API** and paste in the Client ID and Secret. They're stored encrypted on disk via the OS keyring.
3. **Add streamers.** Paste in one Twitch login name per line. Sightline pulls metadata and starts polling.
4. **Wait for the first cycle.** Newly discovered VODs appear in the Library, filtered by your game whitelist (default: GTA V only).
5. **Download & watch.** Click a VOD to queue it, or enable auto-download per streamer. Once downloaded, click ▶ to play, or pair two with **Sync View**.

---

## Architecture overview

```
┌───────────────────────────────────────────────────────────┐
│  React 19 + TypeScript + Tailwind 4 + shadcn/ui           │
│  (TanStack Query · Zustand · typed IPC via tauri-specta)  │
└──────────────────────────▲────────────────────────────────┘
                           │  typed commands/events
┌──────────────────────────┴────────────────────────────────┐
│  Rust + Tauri 2 + Tokio                                   │
│  commands/ · services/ · domain/ · infra/                 │
│  SQLite (sqlx) · Twitch Helix · yt-dlp · ffmpeg           │
└───────────────────────────────────────────────────────────┘
```

Deep dives:

- [Technical specification](docs/tech-spec.md)
- [Data model](docs/data-model.md)
- [Tauri IPC contracts](docs/api-contracts.md)
- [Architecture Decision Records](docs/adr/)
- [Phased implementation plan](docs/implementation-plan.md)

---

## Roadmap

| Phase | Scope                                                  | Status        |
| ----- | ------------------------------------------------------ | ------------- |
| 1     | Foundation (repo, workforce, docs, code skeleton, CI)  | **In progress** |
| 2     | Twitch API client, streamer + VOD ingestion, polling   | Next          |
| 3     | Download engine (yt-dlp orchestration, queue, throttle)| Planned       |
| 4     | Library UI, settings, tray mode                        | Planned       |
| 5     | Player + watch-progress                                | Planned       |
| 6     | Multi-View Sync                                        | Planned       |
| 7     | Auto-cleanup, sub-only handling, polish, v1.0          | Planned       |

---

## Contributing

We welcome contributions. Please read:

- [CONTRIBUTING.md](CONTRIBUTING.md) — setup, branching, commit style, PR process.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — Contributor Covenant 2.1.

Good first issues are tagged `good-first-issue` in the tracker.

---

## License

MIT — see [LICENSE](LICENSE).

Sightline is not affiliated with Twitch Interactive, Inc., Take-Two Interactive, or any GTA-RP server or community.
