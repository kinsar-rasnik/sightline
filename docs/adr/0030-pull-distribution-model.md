# ADR-0030 — Pull distribution model with sliding window

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0005](0005-background-polling-architecture.md) (the poller
  that produces VOD metadata, now rerouted to "available" instead of
  "downloaded") ·
  [ADR-0014](0014-tray-daemon-architecture.md) (the daemon tick we
  re-use for sliding-window enforcement) ·
  [ADR-0024](0024-auto-cleanup-service.md) (existing watermark-based
  cleanup, repurposed as the GC stage of the new state machine).

## Context

v1.0's distribution model was implicit and aggressive: when the
poller discovered a new VOD on a followed streamer, the download
queue automatically picked it up.  In a hobbyist regime with 2-3
streamers and 20 GB of free disk this is a recipe for a full disk
inside two weeks of casual watching.

The Phase-7 auto-cleanup mitigates this *after* the disk fills, but
the user is still surprised by 100 GB of downloads they never asked
for.  CEO direction: **"Default behaviour should be: poll metadata,
let the user pick what to actually download."**

## Decision

A pull-on-demand model where polling produces *metadata-only* rows
and the download queue is fed by explicit user picks (or a sliding-
window automation).  The `vods` table grows a new lifecycle column
that supersedes the old binary "downloaded vs not" view.

### State machine

```
                ┌──────────────┐
                │   available  │  Polled by background, metadata only
                └──────┬───────┘
                       │  pickVod / pickNextN
                       ▼
                ┌──────────────┐
                │    queued    │  In queue, waiting for a worker
                └──────┬───────┘
                       │  (download worker picks up)
                       ▼
                ┌──────────────┐
                │ downloading  │  yt-dlp + re-encode in flight
                └──────┬───────┘
                       │  on success
                       ▼
                ┌──────────────┐
                │    ready     │  File on disk, ready to play
                └──────┬───────┘
                       │  watch_progress.state ∈ {completed, manual}
                       ▼
                ┌──────────────┐
                │   archived   │  Watched, eligible for cleanup
                └──────┬───────┘
                       │  cleanup-service or sliding-window enforce
                       ▼
                ┌──────────────┐
                │   deleted    │  File gone; row preserved for re-pick
                └──────────────┘
```

Backwards transitions:

- `queued → available` via `unpickVod` (the user changed their mind
  before download started).
- `archived → ready` is **not** a state — re-watching an archived VOD
  is allowed by playing the file directly while it still exists.
  Once it's `deleted`, the user has to `pickVod` again, which goes
  through `available → queued` and a fresh download.
- `deleted → available` is also not a state.  The poller already
  produces `available` for every VOD it sees; a re-pick after delete
  finds the row in `available` (the cleanup pass only flips the
  status, it doesn't remove the row).

The `downloads` table from Phase 3 stays as the operational queue
for the `queued`/`downloading` segment.  When a download completes,
the orchestration layer flips both the `downloads` row to `completed`
*and* the `vods.status` to `ready`.  When auto-cleanup runs, it flips
both to `deleted` / `failed_permanent`.  Migration 0016 sets up the
state column; service code keeps the two tables consistent.

### Sliding window

`sliding_window_size` (default 2, range 1..=20) is the maximum number
of `ready`+`queued` VODs Sightline will keep on disk *per streamer*.
When the user marks a VOD as watched (state crosses to `archived`),
the sliding-window enforcer scans that streamer's archived VODs in
oldest-first order and flips the oldest one beyond the window
threshold to `deleted`, deleting the file via the existing cleanup
helpers.

This is **per-streamer**, not global.  A user with 5 streamers and
N=2 keeps up to 10 VODs on disk simultaneously.

### Distribution mode

A new setting `distribution_mode` selects between:

- `'pull'` — the new model.  Default for new installs.  Polling
  produces `available`; downloads only happen on explicit pick or
  pre-fetch.
- `'auto'` — the v1.0 model.  Default for existing installs (the
  migration sets it).  Polling auto-enqueues the `download` row, the
  status column moves directly through `available → queued →
  downloading → ready` without user input.

Both modes share the same state machine; `'auto'` mode just runs
`pickVod` automatically inside the polling pipeline.  This keeps the
DB schema consistent and means the user can flip the toggle at any
time without breaking existing rows.

### Pre-fetch (ADR-0031)

Watching VOD K with the player open is a strong signal that the user
wants K+1 next.  A pre-fetch hook on the player's `loadeddata` event
calls `prefetchCheck`, which under `'pull'` mode picks K+1 in the
background if it's still `available`.  Details in ADR-0031.

### Migration path

`0016_vod_status_machine.sql` introduces the `status` column on
`vods`, populates it from existing rows:

- `vods` row with no matching `downloads` row → `available`.
- `downloads.state = 'completed'` and watch_progress NOT in completed/
  manually_watched → `ready`.
- `downloads.state = 'completed'` and watch_progress IN completed/
  manually_watched → `archived`.
- `downloads.state = 'queued'` → `queued`.
- `downloads.state = 'downloading'` → `queued` (crash-recovery already
  handles `downloading` reset; we route through queued).
- `downloads.state ∈ {'paused', 'failed_retryable', 'failed_permanent'}`
  → `available` (the user has to re-pick to retry; failed_permanent
  with `last_error = 'CLEANED_UP'` becomes `deleted`).

`0017_distribution_settings.sql` adds the `distribution_mode` (default
`'pull'`), `sliding_window_size` (default 2), and `prefetch_enabled`
(default 1) columns.  The migration itself performs the
backwards-compat detection: it issues
`UPDATE app_settings SET distribution_mode = 'auto' WHERE EXISTS
 (SELECT 1 FROM downloads WHERE state IN ('completed','queued','downloading'))`
so an existing populated `downloads` table preserves v1.0
auto-download behaviour.  New installs (empty downloads) keep the
column DEFAULT of `'pull'`.  Doing the detection in the migration
rather than at runtime means there is exactly one moment that
decides — no startup race, no partial-state where the value differs
from what the user sees in Settings.

The `'downloading'` state is included in the detection because a
crash-recovery row left in that state at upgrade time still
indicates an existing install with content; without that branch a
user who crashed mid-download in v1.0 would silently land in
`'pull'` mode after upgrade.

## Alternatives considered

### A. Streaming hybrid: stream VODs that aren't downloaded

Rejected by CEO.  Multi-view (Phase 6) needs frame-accurate seek and
the Twitch HLS endpoint loses frames at scrubbed timestamps.  A
hybrid stream/download model would compromise the multi-view
experience or require maintaining two playback paths.

### B. Hard-pull only, no sliding window

Rejected.  Without the window, every "next episode" requires the user
to remember to pick it.  Sliding-window-with-pre-fetch keeps the
"watched a VOD, the next one is already there" UX while bounding
disk usage to a known constant per streamer.

### C. Global window instead of per-streamer

Rejected.  A user following 5 streamers wants 1-2 episodes from each,
not 5 episodes from one and zero from the others.  Per-streamer
windows match the watching pattern.

### D. Soft-delete with file retention until disk pressure

Rejected.  The whole point of the pull model is to bound steady-state
disk use predictably.  "Mostly delete unless we're not pressured"
re-introduces the v1.0 surprise mechanic.

### E. Replace `'auto'` mode entirely on upgrade

Rejected.  Existing v1.0 users have built habits around their
download library.  Forcing them onto pull mode would surprise users
who wanted the "everything downloads automatically" property.  We
make `'pull'` the default for new installs but preserve `'auto'` for
existing ones; the Settings UI surfaces both with a clear migration
explanation in `docs/MIGRATION-v1-to-v2.md`.

## Consequences

**Positive.**
- Predictable disk footprint: `sum(streamers) × window_size × avg_VOD_GB`.
- The Library UI becomes meaningful — it shows what's *available*,
  not just what's *downloaded*.
- Cleanup-by-watching (the new state-machine path) replaces "cleanup
  by watermark" as the primary disk-pressure relief.  Watermark stays
  as an emergency belt-and-braces.

**Costs accepted.**
- Two new migrations + a moderate rewrite of the `downloads` service
  to consult `vods.status` rather than enqueuing on its own initiative.
- Library-UI re-conception (ADR-0033) — the "downloads" page is no
  longer the only place the user sees their library.
- Migration of existing installs preserves behaviour at the cost of
  the Settings UI explaining "Distribution Mode: Auto-download
  (legacy)" which some users will find confusing.

**Risks.**
- An upgrade where the migration mis-detects "existing install" and
  flips a heavy user onto pull mode would feel like a regression.
  Mitigation: the migration's detection check is the
  `EXISTS (SELECT 1 FROM downloads WHERE state IN
  ('completed','queued','downloading'))` predicate — any of those
  states present in the legacy install pins the user on `'auto'`.
- The pre-fetch hook (ADR-0031) racing with explicit user picks could
  produce duplicate `pick` calls; service-layer idempotency on
  `vods.status = 'available' → 'queued'` covers this.

## Follow-ups

- Per-streamer window override (some streamers stream daily, some
  weekly).  Tracked for v2.1.
- "Pin VOD" — exclude a specific VOD from the sliding-window auto-
  delete.  Useful for highlights / favourite scenes.  Tracked for
  v2.1.
- A "Manual mode" toggle that disables sliding-window enforcement
  entirely (only auto-cleanup-watermark remains).  Currently
  achievable by setting `sliding_window_size = 20` and accepting
  the watermark fallback; a dedicated toggle would be cleaner.

## References

- `src-tauri/migrations/0016_vod_status_machine.sql`
- `src-tauri/migrations/0017_distribution_settings.sql`
- `src-tauri/src/domain/distribution.rs`
- `src-tauri/src/services/distribution.rs`
- `docs/data-model.md` §Phase 8 — Distribution status machine
- `docs/api-contracts.md` §Phase 8 commands
- ADR-0024 (cleanup integration)
- ADR-0031 (pre-fetch policy)
