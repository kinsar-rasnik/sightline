# Sightline

> A cross-platform desktop app for watching multi-streamer GTA Roleplay events on one unified, chronological timeline — with synchronized multi-perspective playback.

<p align="center">
  <em>v2.0.2 — storage-aware, local-first, MIT-licensed, runs on macOS / Windows / Linux.</em>
</p>

> **v2.0.2 hotfix** — fixes the broken sidecar resolution that shipped in v2.0.1 (`ffmpeg` / `yt-dlp` were not found inside the `.app` / `.deb` / `.msi` because the resolver used the Tauri 1 layout).  Encoder detection + downloads now work on first launch.  See [`CHANGELOG.md`](CHANGELOG.md) and [ADR-0034](docs/adr/0034-tauri2-sidecar-layout.md).

> **v2.0.1 highlights** (Phase 8 scope-closure) — see [`CHANGELOG.md`](CHANGELOG.md) for full release notes.
> - **Storage forecast UI** — Streamers → Add now shows a per-streamer disk / bandwidth estimate before you commit. Settings → Storage Outlook shows the global view with per-streamer breakdown and a Green / Amber / Red watermark indicator.
> - **Unified Library** — every non-deleted VOD is rendered with status-aware visual differentiation (opacity + badge) and per-card quick actions (Download / Cancel / Watch / Re-watch / Remove). Filter chips: All / Not downloaded / Downloaded / Watched.
> - **Pre-fetch wired** — when you cross 70% of a VOD (or have <2 min remaining), Sightline pulls the next chronological available VOD on the same streamer in the background.
> - **Windows CPU suspend** — re-encodes now adaptively pause on Windows under system load (NtSuspendProcess via PowerShell), matching the existing macOS / Linux SIGSTOP path.

> **v2.0 highlights** (storage-aware capstone) — see [`CHANGELOG.md`](CHANGELOG.md).
> - **Pull-on-demand** is the new default for fresh installs: polling discovers VODs as `available`, and you pick what to actually download.  Existing v1.0 installs keep their auto-download behaviour automatically; toggle in Settings → Distribution.
> - **720p30 H.265 default** with hardware-encode-first detection (VideoToolbox / NVENC / AMF / QuickSync / VAAPI).  Audio is **never** re-encoded.
> - **Background-friendly re-encode** drops priority and adaptively suspends ffmpeg when CPU load is high — no more frame drops in your game.
> - **Sliding window** caps disk use at `streamer_count × window_size × avg_VOD_GB`.
> - **Migration v1 → v2** is automatic and reversible.  See [`docs/MIGRATION-v1-to-v2.md`](docs/MIGRATION-v1-to-v2.md).

---

## Why

If you follow GTA-RP on Twitch (NoPixel, GTA World, community servers), you probably follow 3–20 streamers who share the same in-world event. When a heist or shootout happens, every participant's VOD is its own shard of the same story — but Twitch gives you no way to find them together, and no way to watch them in sync.

Sightline does exactly that:

- **Aggregates** VODs from every streamer you follow.
- **Orders them by actual stream start time**, not publish time — so a delayed upload still lands in the right place on the timeline.
- **Filters to GTA V only** (configurable whitelist), so non-RP content doesn't clutter the library.
- **Syncs playback** across two VODs to a shared wall-clock, so you can watch a heist from two perspectives at once.

All data stays on your machine. No account required. No telemetry. The optional update checker is **off by default** and, when enabled, makes one outbound GET per day to the public GitHub Releases API — no IDs, no hashes, no client identification beyond a `Sightline/<version>` User-Agent. See [ADR-0026](docs/adr/0026-update-checker.md).

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
- **Pull-on-demand distribution (v2.0)** — pick VODs explicitly, no surprise downloads.  Sliding window keeps disk use bounded.
- **Storage forecast (v2.0.1)** — see disk + bandwidth cost of adding a streamer before you commit, plus a global Storage Outlook in Settings with per-streamer breakdown and a Green / Amber / Red watermark indicator.
- **Quality pipeline (v2.0)** — 720p30 H.265 default, hardware-encode-first, audio passthrough invariant, CPU-throttle for background re-encodes.
- **Sub-only detection** — clearly flagged, never silently failed downloads.
- **Proton Drive–friendly.** The library root can live under any sync provider; the app handles temporary file locks gracefully.

---

## Screenshots

> _Placeholder — will be added once Phase 4 (UI) ships._

---

## Installation

### Pre-built binaries

Sightline ships unsigned binaries on the [GitHub Releases page](https://github.com/kinsar-rasnik/sightline/releases). Pick the asset for your OS:

- **macOS** — `Sightline_<version>_aarch64.dmg` (Apple Silicon). Intel Macs build from source — see [`docs/INSTALL.md`](docs/INSTALL.md).
- **Windows** — `Sightline_<version>_x64-setup.exe` (NSIS) or `Sightline_<version>_x64_en-US.msi` (MSI)
- **Linux** — `sightline_<version>_amd64.AppImage` (any modern distro) or `sightline_<version>_amd64.deb` (Debian/Ubuntu)

First-launch warnings are expected (the binaries are unsigned — see [ADR-0025](docs/adr/0025-release-pipeline.md) for why we don't sign for v1). [`docs/INSTALL.md`](docs/INSTALL.md) walks through the per-OS workaround.

#### Logs

If something goes wrong and you want to attach logs to a bug report, the rolling daily log file lives at:

- **macOS** — `~/Library/Logs/dev.sightline.app/sightline.<date>.log`
- **Windows** — `%LOCALAPPDATA%\dev.sightline.app\Logs\sightline.<date>.log`
- **Linux** — `$XDG_STATE_HOME/dev.sightline.app/sightline.<date>.log` (or `~/.local/state/dev.sightline.app/...` if `XDG_STATE_HOME` is unset)

The seven most recent days are retained. See [ADR-0037](docs/adr/0037-file-logger-activation.md) for the rotation policy.

### Build from source

Build-from-source is a first-class path — security-conscious users should prefer it.

```bash
# Prerequisites: Rust 1.90+, Node 22+, pnpm 10+, platform Tauri deps
# (see https://v2.tauri.app/start/prerequisites/)

git clone https://github.com/<your-fork>/sightline.git
cd sightline
pnpm install
./scripts/bundle-sidecars.sh    # fetches pinned yt-dlp + ffmpeg binaries
pnpm tauri dev
```

For a production build:

```bash
pnpm tauri build
```

#### Bundled sidecars

Sightline ships with two third-party executables running on your
machine:

- **yt-dlp** — fetches the VOD bytes from Twitch. Self-updatable;
  controlled by the "Auto-update yt-dlp" setting.
- **ffmpeg** — container-swap from `.ts` → `.mp4` and thumbnail
  extraction only. Never re-encodes.

Both are pinned to specific versions by SHA-256 in
`scripts/sidecars.lock`. `scripts/bundle-sidecars.sh` (macOS / Linux /
CI) and `scripts/bundle-sidecars.ps1` (Windows) fetch them, verify
the hash **before** anything is executed or extracted, and install
them under `src-tauri/binaries/<tool>-<target-triple>[.exe]` so the
Tauri bundler picks them up as `externalBin`. A mismatch aborts with
exit 3 — no compromised binary ever runs. See [ADR-0013](docs/adr/0013-sidecar-bundling.md)
for the design and refresh procedure; [ADR-0003](docs/adr/0003-pinned-sidecar-binaries.md)
covers the original decision.

---

## Quickstart

### 1. Register a Twitch developer application

Sightline uses the **App Access Token** flow (no end-user OAuth). You need your own Client ID and Client Secret:

1. Sign in at [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps) with any Twitch account. Sightline does not read that account — the credentials only authenticate the app itself.
2. Click **Register Your Application**. Pick any name (e.g. *Sightline Local*).
3. For **OAuth Redirect URLs**, enter `http://localhost` — Sightline never uses the redirect URL, but Twitch requires one.
4. Category: **Website Integration**. Client Type: **Confidential**.
5. After creating, open the app and copy the **Client ID**. Click **New Secret** and copy the **Client Secret**.

### 2. Launch Sightline and paste credentials

Run `pnpm tauri dev` (or the prebuilt binary once Phase 7 ships).

- Click **Settings** in the top nav.
- Paste the Client ID and Client Secret under *Twitch credentials*, click **Save credentials**.
- Sightline stores them in your OS keychain (macOS Keychain / Windows Credential Manager / Linux Secret Service) — never as plaintext on disk.
- Once saved, the form shows a masked preview (`abcd••••`) and a **Replace** button. Subsequent IPC calls never re-expose the secret.

### 3. Add streamers to follow

- Switch to the **Streamers** tab.
- Type a Twitch login (3–25 characters, alphanumeric + underscore) and click **Add**.
- Sightline resolves the user via Helix, stores the avatar + display name, and schedules the streamer for adaptive polling (10 min when live, 30 min when they streamed recently, 2 h when dormant).
- The first poll backfills up to 100 most recent VODs; subsequent polls only fetch new ones (stopping on the first already-seen VOD id).

### 4. Browse the Library

- The **Library** tab lists every ingested VOD ordered by stream start time, newest first.
- Filter chips narrow by status (`Eligible`, `Sub-only`, etc.) and by streamer.
- Click a row to see chapter breakdown, status reason, and a link back to Twitch.
- VODs are classified automatically: a match on your game filter (default: GTA V, id `32982`) + stream ended + not sub-only → **Eligible**. Non-matching games → **Skipped — game**; sub-only VODs are flagged (tooltip explains they'll be re-checked in case the streamer unlocks them); live streams are deferred until they end.

### 5. Download VODs

- Switch to the **Downloads** tab or click **Download** on any eligible
  library row.
- The queue runs up to **Max concurrent downloads** workers in
  parallel (default 2, max 5), respecting your global **Bandwidth
  limit**.
- Pause / Resume / Cancel / Retry each download from either the
  Downloads page or the Library row. Failed downloads auto-retry up
  to 5 times with exponential backoff before landing in
  `failed_permanent`.
- Downloads land in your **Library root** under the chosen
  **library layout** (Plex/Jellyfin or Flat — see
  [docs/user-guide/library-layouts.md](docs/user-guide/library-layouts.md)).
  Switching layouts later runs a background migrator that moves
  existing files atomically.

### 6. Watch

The Phase-5 player handles single-VOD playback with resume-from-
position, chapter navigation, and customisable shortcuts.

### 7. Multi-View

Phase 6 adds **`/multiview`** — open two VODs side-by-side, locked to
a shared wall-clock. Open the library detail drawer for any VOD, tick
a co-stream in the **Co-streams** panel (only available for VODs
already on disk), and click **Open Multi-View**.

Once mounted:

- **Group transport.** Play / pause / seek / speed apply to both
  panes simultaneously. The seek slider is wall-clock anchored —
  dragging it computes the leader's new `currentTime` and the
  follower corrects to match on the next sync-loop tick.
- **Leader pane.** The first opened pane is leader by default.
  Promote the other pane any time via the **Promote to leader**
  button on its hover chrome.
- **Per-pane audio mix.** Each pane has its own volume slider + mute
  toggle. The crossfader is a v2 affordance.
- **Out-of-range handling.** When the leader's wall-clock falls
  outside a follower's VOD window, the follower pauses and shows an
  "Out of range" overlay. The leader keeps playing.
- **Drift correction.** Followers re-sync via a corrective seek when
  drift exceeds 250 ms (configurable in Settings under
  `syncDriftThresholdMs`). The detection loop runs at 250 ms with a
  1 s per-pane cooldown to avoid feedback. See
  [ADR-0022](docs/adr/0022-sync-math-and-drift.md).

> _Screenshots TODO — Phase 7 marketing pass._

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
| 1     | Foundation (repo, workforce, docs, code skeleton, CI)  | ✅             |
| 2     | Twitch API client, streamer + VOD ingestion, polling   | ✅             |
| 3     | Download engine (yt-dlp orchestration, queue, throttle, library layout) | ✅             |
| 4     | Tray daemon, timeline foundation, UI polish, sidecar bundling | ✅             |
| 5     | Player, watch progress, Continue Watching, cross-streamer deep link | ✅             |
| 6     | Multi-View Sync engine (split-view v1) — two-pane wall-clock-locked playback with leader-led drift correction | ✅             |
| 7     | Auto-cleanup, release pipeline, update checker, v1.0   | ✅             |

Sightline is **complete at v1.0**. Post-1.0 follow-ups (Playwright + tauri-driver E2E coverage, per-pane volume persistence for sync sessions, PiP / >2-pane Multi-View, Homebrew / winget / AUR distribution, Code-signing) are tracked separately and won't block the v1 release cadence.

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
