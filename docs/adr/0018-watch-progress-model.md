# ADR-0018 — Watch-progress data model + state machine

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 5)
- **Related.**
  [ADR-0002](0002-local-persistence-sqlite-sqlx.md)
  (SQLite / append-only migrations) ·
  [ADR-0015](0015-timeline-data-model.md) (the materialised-view
  pattern this follows).

## Context

Phase 5 needs to persist resume-from-position, watched fraction, and
a cumulative playback-time stat so the library can surface a
Continue Watching row and the Phase-7 auto-cleanup policies can be
built on real data. The naive approach — one column per field on
`vods` — breaks our rule that `vods` is the unchanged chronological
record of what the poller saw. A dedicated table keeps the
downstream pipelines decoupled and leaves room for per-user rows if
we ever grow the threat model to accommodate that.

## Decision

A new table `watch_progress` with one row per VOD the user has ever
opened:

```sql
CREATE TABLE watch_progress (
    vod_id                         TEXT    PRIMARY KEY REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    position_seconds               REAL    NOT NULL DEFAULT 0 CHECK (position_seconds >= 0),
    duration_seconds               REAL    NOT NULL CHECK (duration_seconds >= 0),
    watched_fraction               REAL    GENERATED ALWAYS AS (
        CASE WHEN duration_seconds > 0
             THEN position_seconds / duration_seconds
             ELSE 0
        END
    ) STORED,
    state                          TEXT    NOT NULL CHECK (state IN (
        'unwatched','in_progress','completed','manually_watched'
    )),
    first_watched_at               INTEGER,
    last_watched_at                INTEGER NOT NULL,
    last_session_duration_seconds  REAL    NOT NULL DEFAULT 0,
    total_watch_seconds            REAL    NOT NULL DEFAULT 0
);

CREATE INDEX idx_watch_progress_last_watched
    ON watch_progress(last_watched_at DESC);
CREATE INDEX idx_watch_progress_state
    ON watch_progress(state);
```

`watched_fraction` is a `STORED` generated column so a future
"sort library by % watched" query can index on it without the
services layer recomputing the value on every read.

### State machine

```
    unwatched
      │ first position > 0
      ▼
    in_progress ──► completed   (watched_fraction ≥ threshold)
      │                │
      │                └──► unwatched (mark-as-unwatched)
      │
      ├──► manually_watched (mark-as-watched)
      │     │
      │     └──► unwatched  (mark-as-unwatched)
      │
      └──► manually_watched (directly, from any non-completed state)
```

Two sticky terminal states — `completed` and `manually_watched` —
don't flip back under organic `timeupdate`. Only explicit
mark-as-unwatched returns a VOD to `unwatched`. The distinction
between the two terminals matters for later phases:

- `completed` implies the user actually watched up to the
  threshold; Phase 7's auto-cleanup can delete these.
- `manually_watched` is a user choice that asserts "I don't need
  to re-watch this" without implying the user actually saw the
  video. Phase 7 can still clean these up but the messaging
  should be different.

The state machine lives in `domain::watch_progress`. The services
layer never decides a transition on its own — it calls
`transition_on_update` / `on_mark_watched` / `on_mark_unwatched`
and persists the return value.

### Persistence strategy

- In-memory position tracked at the video's `timeupdate` event
  (~4 Hz).
- DB write every 5 s wall-clock OR on pause OR on tab blur OR on
  player unmount. Flush-on-close semantics: every exit path writes
  once more before returning.
- Position rounds to 0.5 s resolution via
  `domain::watch_progress::round_to_half_second` before the DB
  write. Cuts write amplification from ~240 writes/min to ~12.

### Resume math

- On open, read `position_seconds` and seek to
  `max(0, position - pre_roll_seconds)` (default 5 s).
- If `position >= duration - restart_threshold_seconds` (default
  30 s), return 0 instead — the user would otherwise resume in the
  last 30 s of a VOD, which is almost never what they want.
- Pre-roll and the restart threshold are both configurable; the
  state-machine module accepts them as a `ProgressSettings` struct.

### `total_watch_seconds`

Accumulates *unique* playback time — the player session manager
holds an `IntervalSet` (pure domain code in
`domain::interval_merger`) and tells the service layer the cumulative
new-coverage delta after each observation. Scrubbing back over
already-seen territory doesn't double-count.

The frontend's `timeupdate` handler drives the observation stream at
4 Hz; the service persists the cumulative total every 5 s, same
cadence as the position write, via
`WatchProgressService::add_watch_seconds`.

## Alternatives considered

### A. Overload `vods` with watch columns

Rejected. The Phase-2 contract on `vods` is "every row is a
poller-observed moment". Adding user-specific state couples ingest to
personal data and forces migrations every time the watch-state
vocabulary grows.

### B. Single `state TEXT` on `vods` without the detail table

Rejected for the same reason — losing `position_seconds` /
`total_watch_seconds` / the state transitions' timestamps would mean
the Continue Watching row couldn't rank by "most recently watched"
(our sort key) and Phase 7 couldn't reason about stale VODs.

### C. Per-user rows, keyed on `user_id` + `vod_id`

Rejected today. Sightline is single-user by design (Tauri app,
local-only). A future cloud-sync ADR would have to thread a user
identity through every query and this ADR would be superseded. For
now the single-user assumption buys us a cleaner PK.

### D. JSON column for everything except position

Rejected — the column-per-field approach lets SQLite's CHECK
constraint enforce the state-machine vocabulary, and the generated
`watched_fraction` column lets future indexes avoid a function scan.

## Consequences

**Positive.**
- The state machine lives in a pure module with exhaustive tests;
  services + commands don't re-implement it.
- Generated `watched_fraction` means the "X% watched" sort doesn't
  need a recompute.
- `ContinueWatchingEntry` can be built from a single JOIN against
  `vods` + `streamers`.
- Deleting a streamer's VOD (rare) cleanly drops the watch row via
  the FK cascade.

**Costs accepted.**
- Every `timeupdate`-driven write is a full upsert (not a
  partial). SQLite's WAL + 0.5 s rounding + 5 s debounce keeps this
  well under 1 MB/day even for a heavy watcher.
- A migration is required to add columns (e.g. if we ever track
  loudness-normalised audio). We've already accepted migration-per-
  change as the model (ADR-0002).

## Follow-ups

1. Phase 7 auto-cleanup policy — consumes `state`, `last_watched_at`,
   and `total_watch_seconds`.
2. Phase 7 "Hours watched" per-streamer stats — `stats(streamer_id)`
   already returns the aggregate; the UI lands with the polish pass.
3. Possibly expose `watched_fraction` as a sortable column on the
   library grid. Would require a small migration to index it; the
   generated column is already STORED so the index is cheap.
