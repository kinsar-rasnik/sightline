# ADR-0026 — Update checker (opt-in, GitHub Releases API, notification-only)

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 7)
- **Related.**
  [ADR-0014](0014-tray-daemon-architecture.md) (tray daemon tick we
  reuse) ·
  [ADR-0025](0025-release-pipeline.md) (Releases this checker
  reads).

## Context

[ADR-0025](0025-release-pipeline.md) ships unsigned binaries via
GitHub Releases. Without an in-app cue, a user installs v1.0.0 and
never knows v1.1 exists. We want a low-effort path that surfaces "a
new version is available" without:

- self-updating the binary (security-sensitive without code signing
  — out of scope for v1, see ADR-0025),
- shipping any telemetry (Sightline's privacy posture is "local-only",
  see `docs/tech-spec.md` §8 and the README),
- nagging the user (one banner per available version, with
  per-version skip).

## Decision

A new `services::updater` module + `commands::updater` IPC surface +
`features/updater` frontend. Behaviour:

- **Opt-in, default off.** A new `app_settings.update_check_enabled`
  column (default `0`) gates the entire feature. The Settings UI
  surfaces a single toggle.
- **GitHub Releases API as the source.** A single GET to
  `https://api.github.com/repos/kinsar-rasnik/sightline/releases/latest`,
  no auth header needed for public repos (60 req/hr/IP unauth limit
  — irrelevant at 1 req/day cadence).
- **Daily cadence** when enabled. The tray daemon (ADR-0014) ticks
  the updater no more than once per 24 h wall-clock — `app_settings.
  update_check_last_run` enforces this server-side.
- **Notification-only.** When the GitHub `tag_name` semver-greater
  than `env!("CARGO_PKG_VERSION")` and not in the user's skip list,
  we emit `updater:update_available` and the AppShell renders a
  banner with three actions:
  1. "View release" — opens the GitHub release URL via Tauri's
     existing shell-open path (the user lands in their browser; we
     never download the new binary in-process).
  2. "Skip this version" — writes the new tag to
     `app_settings.update_check_skip_version`, suppressing the
     banner until a still-newer release ships.
  3. "Remind me later" — dismisses the banner for the current
     session only; the next daily tick will re-show it.
- **No automatic install.** Self-update needs signing
  infrastructure (ADR-0025). Defer to a post-1.0 ADR.

### Schema

Migration `0014_update_settings.sql` extends `app_settings`:

```sql
ALTER TABLE app_settings
    ADD COLUMN update_check_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (update_check_enabled IN (0, 1));
ALTER TABLE app_settings
    ADD COLUMN update_check_last_run INTEGER;
ALTER TABLE app_settings
    ADD COLUMN update_check_skip_version TEXT;
```

`update_check_last_run` is nullable — a brand-new install hasn't
checked yet. `update_check_skip_version` is nullable — the user
hasn't skipped any version yet. Both populate via the service layer.

### Service layer

`UpdaterService::check_for_update(force: bool) → Option<UpdateInfo>`:

1. If `force = false` and `now - last_run < 24h`, return `None`.
2. GET the GitHub Releases endpoint with a 10 s timeout. Re-use the
   shared `reqwest::Client` from `AppState`. User-Agent is
   `Sightline/<version>` per GitHub's API guidance.
3. Parse `tag_name`, `name`, `body`, `html_url`, `published_at`. We
   accept `tag_name` of the form `v<semver>` and strip the leading
   `v` before comparison. Anything else logs a warning and returns
   `None`.
4. Compare against `env!("CARGO_PKG_VERSION")` using
   semver. Pre-release / build-metadata comparison follows
   semver 2.0 (build metadata ignored, pre-release ranks below
   release). The `semver` crate handles this.
5. If the GitHub tag matches the user's
   `update_check_skip_version`, return `None`.
6. Persist `last_run = now` regardless of outcome.
7. If newer, return `Some(UpdateInfo { ... })` and emit
   `updater:update_available` once.

A `force = true` call from the manual "Check now" UI bypasses the
24 h gate but still respects the skip list — clearing a skip is a
separate action via `cmd_skip_update_version("")` (empty string
clears).

Failure modes (`updater:check_failed { reason }`):

- Network unreachable → no event noise; the tick swallows it. The
  next daily tick will retry.
- Manual "Check now" failure → `updater:check_failed` event so the
  Settings UI can show "Couldn't reach GitHub. Check back later."
- 4xx/5xx from GitHub → same `check_failed` event with the status
  code in the reason string.

### Tray daemon integration

A new `updater_tick` future inside the existing tray loop
(parallel to the cleanup tick from ADR-0024). Wakes every 60
minutes; calls `check_for_update(false)` if enabled. The 60-minute
poll period combined with the 24 h `last_run` gate means we GET
GitHub's API at most once per day even on a long-running daemon.

### Frontend

A small `UpdateBanner` component mounted high in `AppShell`. Reads
state from `useUpdateStatus()` (a TanStack Query that calls
`getUpdateStatus`). Renders only when `available` and not session-
dismissed.

Settings page gains an "Updates" section with:
- Toggle: enable update checks (off by default)
- Last checked timestamp (or "—")
- Manual "Check now" button — calls `checkForUpdate({ force: true })`
- "Skip this version" CTA only when a newer version is available

## Alternatives considered

### A. tauri-plugin-updater (Sparkle-style self-update)

Rejected for v1. Needs signed builds (we don't sign; ADR-0025) and
a signed update manifest hosted somewhere we control. Adds
operational surface (compromising the manifest = compromising every
user) for a feature that v1 doesn't need.

### B. Show update info even when disabled

Rejected. The toggle's whole point is "I don't want this app
talking to the internet without my consent". A "but only this
endpoint" exception undermines the privacy claim.

### C. GitHub Atom feed instead of REST API

The Atom feed (`/releases.atom`) is unauthenticated and has no rate
limit, but parsing XML in Rust adds a dep we don't currently have
(`quick-xml`). The `releases/latest` JSON endpoint is unauth-allowed
to 60 req/hr/IP, plenty for our 1-req/day cadence. Stick with JSON.

### D. Per-version "skip" stored as a JSON array

Rejected for v1 — we only need "skip the most-recent one I saw and
chose to skip". A single TEXT column is enough. If a future v2
needs multi-skip support, the migration is `TEXT → TEXT[]` (JSON
array column).

### E. Push notifications via WebSocket

Rejected. Sightline is offline-tolerant; a push channel adds
infrastructure we don't have. Daily polling is fine for a desktop
app.

## Consequences

**Positive.**
- Users opt in to a single, transparent network call: `GET
  https://api.github.com/repos/.../releases/latest`. No telemetry,
  no ID, no hash of any local state.
- Default-off respects the privacy-first posture without taking
  away the convenience for users who want it.
- Skip-version UX prevents the banner from becoming background
  noise.
- Failure modes are observable in Settings ("Couldn't reach
  GitHub" line) without affecting the rest of the app.

**Costs accepted.**
- New migration + 3 commands + 2 events. ~150 LOC backend, ~80 LOC
  frontend. The `semver` crate adds ~20 KB to the binary.
- One more service in the tray loop. The total tick cost stays
  under 5 ms wall clock on idle.
- The check itself is silent on failure when scheduled — a sustained
  outage at GitHub means the user never sees the banner. Acceptable
  trade-off vs. a noisy "we couldn't check" toast every day.

**Risks.**
- The GitHub API URL is hard-coded — if the repo moves we have a
  stale URL in shipped binaries. Acceptable: it's a constant in
  source, so rotating it is a normal release.
- A maliciously-crafted GitHub release with a 100 MB body would
  slow the parse. We cap the response read at 64 KB; anything
  larger is treated as a parse failure.

## Follow-ups

- Multi-skip array if user feedback wants finer-grained control.
- A "release notes preview" inline view inside the banner. v1
  punts to "View release" → browser to keep the Markdown-render
  scope contained.
- Self-update via tauri-plugin-updater once we sign builds. Tracks
  with the same code-signing investment as ADR-0025's follow-up.

## References

- `src-tauri/migrations/0014_update_settings.sql`
- `src-tauri/src/services/updater.rs`
- `src-tauri/src/commands/updater.rs`
- `src/features/updater/UpdateBanner.tsx`
- `src/features/updater/use-update-status.ts`
