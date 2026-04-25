# ADR-0024 — Auto-cleanup service

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 7)
- **Related.**
  [ADR-0011](0011-library-layout-pluggability.md) (library layout we
  delete from) ·
  [ADR-0014](0014-tray-daemon-architecture.md) (the daemon tick that
  runs the scheduled cleanup) ·
  [ADR-0018](0018-watch-progress-model.md) (watch state we read from
  to choose candidates).

## Context

Phase 7 is the v1.0 capstone. Sightline's library grows monotonically
under normal use — a heavy GTA-RP watcher accumulates several hundred
GB per month. Without an explicit shrink path the user eventually
runs out of disk, the download queue starts failing
`AppError::DiskFull`, and the only recovery is the user's file
manager.

[ADR-0018](0018-watch-progress-model.md) introduced the
`watch_progress` table specifically to make a principled cleanup
decision possible: `state ∈ {completed, manually_watched}` plus
`last_watched_at` plus `total_watch_seconds` is enough to rank VODs
by "user no longer needs this". This ADR fixes the policy.

Constraints the policy has to respect:

1. **Never delete data the user might want back without a clear
   confirmation path.** The download is gone when we delete the file
   — yt-dlp can refetch from Twitch only while the source VOD is
   still up.
2. **Must work without user input** for the scheduled tick path. The
   tray daemon (ADR-0014) is the only execution surface that survives
   a window close, and we want disk pressure handled in the
   background.
3. **Must surface the plan before executing it** when the user
   triggered cleanup manually.
4. **Must respect "I'm watching this right now"** — never pick a VOD
   the user opened in the player in the last 24 h.

## Decision

A `services::cleanup` module that owns disk-watermark logic, candidate
selection, and the cleanup execution itself. Settings persist in
`app_settings`; a `cleanup_log` table records every run for the UI's
History view and for post-mortem.

### Data shape

Migration `0012_cleanup_settings.sql` extends `app_settings` with
four columns:

```sql
ALTER TABLE app_settings
    ADD COLUMN cleanup_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (cleanup_enabled IN (0, 1));
ALTER TABLE app_settings
    ADD COLUMN cleanup_high_watermark REAL NOT NULL DEFAULT 0.9
        CHECK (cleanup_high_watermark >= 0.5
               AND cleanup_high_watermark <= 0.99);
ALTER TABLE app_settings
    ADD COLUMN cleanup_low_watermark REAL NOT NULL DEFAULT 0.75
        CHECK (cleanup_low_watermark >= 0.4
               AND cleanup_low_watermark <= 0.95);
ALTER TABLE app_settings
    ADD COLUMN cleanup_schedule_hour INTEGER NOT NULL DEFAULT 3
        CHECK (cleanup_schedule_hour >= 0
               AND cleanup_schedule_hour <= 23);
```

Migration `0013_cleanup_log.sql` adds the audit table:

```sql
CREATE TABLE cleanup_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    ran_at            INTEGER NOT NULL,
    mode              TEXT    NOT NULL CHECK (mode IN (
                          'scheduled', 'manual', 'dry_run'
                      )),
    freed_bytes       INTEGER NOT NULL DEFAULT 0,
    deleted_vod_count INTEGER NOT NULL DEFAULT 0,
    status            TEXT    NOT NULL CHECK (status IN (
                          'ok', 'partial', 'skipped', 'error'
                      ))
);
CREATE INDEX idx_cleanup_log_ran_at ON cleanup_log(ran_at DESC);
```

### Watermarks

Two floats in `[0, 1]`, both fractions of total disk capacity at the
library partition:

- `cleanup_high_watermark` (default 0.9) — when free-space-on-disk
  falls below `1 − high_watermark`, cleanup fires.
- `cleanup_low_watermark` (default 0.75) — when cleanup fires, it
  removes files until the partition is at or below `low_watermark`
  utilisation, then stops.

The hysteresis avoids a thrash where the next download immediately
re-trips the high watermark.

Watermarks are inclusive bounds (`>=` / `<=`) to match the
SQLite `CHECK` constraints, which use the same operators.

### Candidate selection

`compute_plan(probe, ...) → CleanupPlan` produces a ranked list of
deletion candidates without touching the filesystem. Ranking is
strict, not heuristic — every comparator below is checked in order
and ties fall through to the next rule:

1. **Skip recently-touched VODs.** Anything with
   `last_watched_at >= now - 86_400` is excluded entirely. The user
   may resume an active watch session.
2. **`completed` before `manually_watched`.** A user who marked-as-
   watched without actually watching has expressed a stronger "I
   don't need this" signal; we still prefer to delete VODs that the
   threshold-driven `completed` state captured first because
   `completed` carries an implicit "the user actually saw this".
   See [ADR-0018](0018-watch-progress-model.md) §State machine.
3. **`in_progress` and `unwatched` are NEVER candidates** — even if
   the disk is full, we do not auto-delete unwatched VODs. The user
   can flip them to `manually_watched` themselves; that's an explicit
   opt-in.
4. **Older `last_watched_at` first.** Within a state bucket, the
   least-recently-touched VOD goes first.
5. **Larger files first** when ages are equal (within the same
   wall-clock day) — frees more space per delete operation.

The plan stops accumulating when projected post-deletion utilisation
crosses below `low_watermark`. The plan also caps at 200 VODs per
run to keep a single tick from doing too much work; the next tick
picks up where this one stopped if pressure persists.

### Execution

`execute_plan(plan, mode) → CleanupResult` is the only path that
actually removes files. It:

1. Re-reads each candidate row inside a transaction to avoid races
   with the download queue (a file might have been re-enqueued
   between plan and execute).
2. Removes the file via the same atomic-move helpers Phase 3 already
   uses (`infra::fs::move_::atomic_remove`), preserving the layout's
   sidecars.
3. Updates the `downloads` row to `failed_permanent` with
   `last_error = 'CLEANED_UP'`. The Library UI re-renders the VOD
   with a "Re-download" CTA. **The watch_progress row is preserved**
   so a re-download surfaces the user's prior position.
4. Inserts a single `cleanup_log` row with the aggregate
   `freed_bytes` / `deleted_vod_count` / status.

Three modes:

- `scheduled` — fires from the tray daemon at the configured hour,
  no UI interaction. Status `skipped` if disk usage is below the
  high watermark; otherwise runs.
- `manual` — fires from the Settings UI's "Run cleanup now" action.
  Always runs the deletion path (after the user confirmed the plan).
- `dry_run` — the UI's "Preview what would be deleted" path. Returns
  the plan without touching disk; logs a row with `mode='dry_run'`.

### Scheduler integration

The tray daemon (ADR-0014) gains a `cleanup_tick` future. It checks
every 5 minutes whether the local clock has crossed the configured
`cleanup_schedule_hour` since the last run; if so, calls
`compute_plan` then `execute_plan(scheduled)` if disk pressure is
above watermark. The 5-minute granularity keeps the wake
unobtrusive; cleanup runs at most once per day per the
last-run guard.

If `cleanup_enabled = 0` the tick is a no-op. We still emit a
`cleanup:disk_pressure` event when free-space falls below the
high watermark even when disabled, so the UI can surface a banner
like "Disk is filling up — enable Auto-cleanup or free space
manually."

## Alternatives considered

### A. Time-only retention (e.g. delete VODs older than 30 days)

Rejected. Time alone doesn't track watch state — would happily
delete a 31-day-old VOD the user is actively re-watching. Combined
with a watermark it would also fire too aggressively for users with
lots of free disk.

### B. Watermark-only, no schedule

Rejected. Without a fixed schedule the tick fires on every disk
write, which would mean the cleanup service runs at the same cadence
as `download:progress` events (multiple times per second). The
watermark-with-once-daily-schedule strikes the right balance:
deterministic, predictable, observable.

### C. Cleanup picks `unwatched` when desperate

Rejected. The whole point of unwatched is "the user might still
watch this". Deleting it would be data loss. If the partition fills
to 100 % the download queue surfaces `AppError::DiskFull` and the
user has to act — that's an acceptable failure mode for a v1.

### D. Cleanup deletes the `watch_progress` row too

Rejected. Keeping the watch row means a re-download (after Twitch
mute lifts, user wants to rewatch, etc.) lands the user back at
their last position. The row is small; preserving it is cheap and
high-value.

### E. Async per-file events

Rejected for v1. We emit one `cleanup:executed` event with the
aggregate. Per-file events would dominate the event bus during a
500-file cleanup run with no UI win — the History view re-fetches
the log row.

## Consequences

**Positive.**
- Disk-full failures from heavy use largely disappear once the
  feature is enabled. Users who don't enable it see a clear banner
  when pressure crosses the high watermark.
- Watch progress survives re-downloads; the re-download CTA is the
  only post-cleanup recovery action a user has to learn.
- Audit trail in `cleanup_log` lets a user (and a debugging
  developer) see exactly what was freed and when.

**Costs accepted.**
- Two new migrations and four new commands. The IPC surface grows by
  ~5 %, which is the expected cost of a Phase-7 polish feature.
- The "skip recently-touched" check is wall-clock-aware via the same
  `Clock` trait the rest of the codebase uses, so tests can fake the
  age comparison cleanly. Adds a test surface that mirrors
  Phase-3 patterns.
- We rely on `infra::fs::space::FreeSpaceProbe` (an existing trait)
  to learn total + free disk capacity. `sysinfo`'s `total_space()`
  is added to the abstraction; behaviour on Proton-Drive-mounted
  paths is the same as the download preflight path (already
  exercised in Phase 3).

**Risks.**
- A user who toggles `cleanup_enabled = 1` on a partition that's
  already over the high watermark will see a large cleanup run on
  the next tick. The UI's onboarding for the toggle previews the
  plan first via a confirmation drawer.
- The 200-VOD cap means very full libraries take multiple ticks to
  drain. Acceptable: the goal is steady-state pressure relief, not
  a one-shot wipe.

## Follow-ups

- Per-streamer retention overrides (e.g. "never auto-delete from
  Streamer X") — out of scope for v1, easy to add as a column on
  `streamers` if/when users ask.
- Trash-bin instead of unlink. v1 deletes outright. A future
  ADR-NN could route through the OS trash (`trash` crate) so a user
  can recover. Punted because the OS trash is partition-bound and
  would need its own quota story.

## References

- `src-tauri/migrations/0012_cleanup_settings.sql`
- `src-tauri/migrations/0013_cleanup_log.sql`
- `src-tauri/src/services/cleanup.rs`
- `src-tauri/src/domain/cleanup.rs`
- `src-tauri/src/commands/cleanup.rs`
- `docs/data-model.md` §Phase 7 — Auto-cleanup
- `docs/api-contracts.md` §Phase 7 commands
