# ADR-0015 — Timeline data model and incremental indexer

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 4)
- **Related.** [ADR-0002](0002-local-persistence-sqlite-sqlx.md) (SQLite).
  [ADR-0008](0008-chapters-via-twitch-gql.md) (chapters give the per-game
  segmentation the timeline's popover depends on).

## Context

Phase 4 introduces the `/timeline` route — a horizontal chronological view
across streamers so the user can see "who was live at the same time" at a
glance. Phase 6 (multi-view sync) builds on this: the user picks two bars
that visually overlap and watches the two VODs side-by-side locked to a
shared wall-clock.

Two competing design pressures:

1. **Speed.** The UI needs to render 5 years of data for 20 streamers at
   60 fps. Scanning `vods` on every pan/zoom would be too slow; the query
   pattern is "give me every stream with `end_at >= since AND start_at <=
   until`", and `vods` has no `end_at` column (end is computed from
   `stream_started_at + duration_seconds`).
2. **Correctness.** An interval's source of truth is `vods` — the Helix
   response is the canonical record. The timeline must not drift.

## Decision

### Storage

A materialised view in the form of a dedicated table, `stream_intervals`
(migration 0006):

```sql
CREATE TABLE stream_intervals (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  vod_id      TEXT    NOT NULL UNIQUE REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
  streamer_id TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
  start_at    INTEGER NOT NULL,
  end_at      INTEGER NOT NULL,
  created_at  INTEGER NOT NULL,
  CHECK (end_at >= start_at)
);
CREATE INDEX idx_intervals_start    ON stream_intervals(start_at);
CREATE INDEX idx_intervals_range    ON stream_intervals(start_at, end_at);
CREATE INDEX idx_intervals_streamer ON stream_intervals(streamer_id, start_at);
```

- `UNIQUE(vod_id)` enables `ON CONFLICT DO UPDATE` on re-ingest.
- `CASCADE` on both foreign keys: dropping a streamer (hard delete) or a
  VOD collapses the materialised row.
- Three indexes cover the three main access patterns: range-bounded scan,
  lane-scoped scan, and start-time-ordered enumeration.

### Indexer service

`services::timeline_indexer::TimelineIndexerService` owns the table.

Two trigger points:

1. **Incremental, event-driven.** The poller's event sink hooks into
   `IngestEvent::VodIngested` and calls `upsert_from_vod`, which reads
   `duration_seconds` from the vods row and writes / updates the interval.
   This is the steady-state path — a VOD is ingested, the interval is
   live for the next `/timeline` query.
2. **Backfill.** On first startup after migration 0006 lands, the
   `stream_intervals` table is empty but `vods` is populated. The
   service's `rebuild_all` is kicked off in a background task during
   `setup`; progress events (`timeline:index_rebuilding`) fire every
   `Rebuilding { processed, total }` step so the UI can render a progress
   bar. For Phase 4 we use a single bulk `INSERT ... SELECT` inside a
   transaction — the dominant cost is the SELECT, so streaming wouldn't
   actually be faster. If we see real users with hundreds of thousands of
   rows later we can switch to a paginated walk, but the transaction
   remains the simpler default.

The admin command `cmd_rebuild_timeline_index` lets a user trigger the
same rebuild on demand (used if the user notices a row is stale after
fixing upstream data manually).

### Read surface

Three IPC commands:

- `cmd_list_timeline({ since?, until?, streamerIds? })` — range query.
  Returns `Vec<Interval>` ordered by `start_at`. A dynamic `IN` clause is
  assembled from a fixed set of fragments (no string concatenation from
  user input); every value is `.bind(...)`ed.
- `cmd_get_co_streams({ vodId })` — "which other streamers overlap this
  VOD". First resolves the subject's interval, then reuses `list` with
  the subject's `[start, end]` window, then calls the pure
  `domain::timeline::find_co_streams`.
- `cmd_get_timeline_stats()` — total count, earliest/latest endpoints,
  peak concurrent group size (sweep-line over endpoints).

### Pure domain helpers

`domain::timeline` has:

- `Interval { vod_id, streamer_id, start_at, end_at }` — mirrors the DB row.
- `overlapping(a, b) -> Option<Interval>` — half-open intersection (touching
  endpoints don't count).
- `bucket_by_day(&[Interval]) -> BTreeMap<day, Vec<&Interval>>` — used by
  the frontend for day/week/month views; clips midnight-straddling
  intervals into multiple buckets without mutating inputs.
- `find_co_streams(around, all) -> Vec<CoStream>` — filters out same-vod +
  same-streamer, sorts by overlap length descending.

Everything in `domain::timeline` is pure. Tests cover the happy cases
plus proptest-backed properties: `overlapping` is symmetric; the
intersection duration never exceeds either input's; `find_co_streams`
never returns same-streamer hits; `bucket_by_day` preserves every input.

## Alternatives considered

### A. Compute overlaps in the UI from `vods` directly

Rejected. The row-count grows linearly with streamers × months; the UI
already has to maintain scroll position across zooms and pans, so
pushing overlap computation into the render path costs frames. A
materialised view is the standard answer.

### B. Store intervals as (streamer, start, duration)

Rejected. Range queries (`end_at >= since`) want `end_at` as a column we
can index directly. Computing it from `start + duration` at query time is
strictly more work for the planner.

### C. Use a SQLite R-tree for overlap queries

Considered; deferred. SQLite's R-tree module works well for 2-D range
queries but we only have a 1-D time axis; the simple composite index is
already efficient enough. If Phase 6's sync-overlay query pattern
changes the shape (e.g. to "give me every interval containing *this*
instant"), we can add an R-tree then.

### D. Compute peak concurrency on-write (bump a counter)

Rejected. Off-line sweep-line on `O(n log n)` sort is fast enough for
reasonable sizes (our largest expected dataset is ~100 k rows), and
the counter approach tangles the write path with queries the user might
not run in a given session.

## Consequences

**Positive.**
- `/timeline` renders a range in one indexed scan.
- Co-stream lookup on the library detail drawer is O(k × m) where k is
  the number of streams overlapping the subject's window — typically ≤ 5.
- Cascades mean a streamer removal leaves no orphan intervals.
- The rebuild path is the migration story: new users get the index
  backfilled automatically on first boot after they upgrade to a
  Phase 4 release.

**Costs accepted.**
- The table is a pure derivation of `vods`. Adding an interval means two
  writes (the vod upsert + the interval upsert) but both go through the
  same transaction in `services::ingest`, so consistency is preserved.
- A bug that desynchronises the table requires a rebuild; the admin
  command exists for exactly that.

## Follow-ups

1. Phase 6 will likely want a Phase-6-specific read (e.g. "give me the
   first instant where N ≥ 2 streamers are live"). That's a new SQL
   query, not a new table — we already have the indexes.
2. If we add a "hide sub-only VODs from timeline" preference, that's a
   filter applied at the query, not a structural change.
3. Partial indexes could narrow some reads (e.g. "only favourited
   streamers") if profiling shows the lane-scoped query is hot.
