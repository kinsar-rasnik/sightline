# ADR-0036 — Per-download in-memory process lock

- **Status.** Accepted
- **Date.** 2026-04-26 (v2.0.3 hotfix)
- **Related.**
  [ADR-0035](0035-download-engine-settings-wiring.md) (the
  concurrency-cap fix that this lock complements) ·
  [ADR-0005](0005-background-polling-architecture.md) (the
  manager-loop pattern this fits into) ·
  [ADR-0012](0012-staging-atomic-move.md) (the staging-fragment
  semantics that the lock protects).

## Context

The CEO's v2.0.2 debug log showed two yt-dlp processes spawning for
the same VOD-ID, evidenced by doubled
`[info] v2756822427: Downloading 1 format(s): 1080p60` lines and
the eventual frag-rename collision:

```
ERROR: Unable to rename file:
   '...2756822427.mp4.part-Frag160.part'
   -> '...2756822427.mp4.part-Frag160'
```

Trace through `services::downloads::drain_once`:

1. Iteration 1: `pick_next_queued()` returns row X (state =
   `queued`).  `tokio::spawn` fires the worker task.  The task is
   *async* — its first action (transitioning state to `downloading`)
   has not yet run.
2. Iteration 2: `pick_next_queued()` returns row X **again**, because
   the state is still `queued` in the DB.  `tokio::spawn` fires a
   second worker task for the same VOD.

Two workers race over the same staging directory.  The first one
finishes a `.part-FragN.part` write, gets ready to rename to
`.part-FragN`, and the second worker has just deleted that
intermediate file as part of its own pipeline.  yt-dlp errors out,
both workers retry, the disk fills.

The Tokio scheduler executing `drain_once` is single-threaded, so
this is *intra*-tick: one drain pass spawning two tasks for the
same row before the first task could mark it as taken.  Sub-Phase C
of v2.0.3 had to provide an authoritative "this VOD is being worked
on right now" claim that survived the inherent race between picking
and state-transition.

## Decision

`DownloadQueueService` carries an
`Arc<Mutex<HashSet<String>>>` named `in_flight`.  Each worker spawn
synchronously claims the row's `vod_id` via `try_lock_inflight`,
which returns either `Some(InFlightGuard)` (slot acquired) or
`None` (already in flight — skip).  The guard is moved into the
spawned task; its `Drop` impl removes the `vod_id` on any task
exit (success, failure, panic, or graceful shutdown).

The lock is **in-memory only.**  A crash starts the next process
with an empty set; the existing
`DownloadQueueService::crash_recover` already resets stuck
`downloading` rows to `queued`.  No file-based lock means no
stale-lock cleanup story to maintain — a process restart is the
only "release" path that matters.

`pick_next_queued_excluding(&HashSet<String>)` replaces the old
`pick_next_queued`.  The new variant adds a `NOT IN (?, ?, ...)`
clause so the SELECT itself filters out in-flight VODs, preventing
the inner loop from looping forever on a single locked row when the
state-transition hasn't yet landed.  The lock check via
`try_lock_inflight` remains as belt-and-suspenders: the SELECT can
race against an insert into the HashSet, but the lock is the wall.

`drain_once` flow (post-fix):

```
1. cap   = current_concurrency(self).await        # live, 1..=3
2. set_active_workers(in_flight.len())            # GlobalRate refresh
3. loop:
3a.   in_flight = in_flight_snapshot()
3b.   if in_flight.len() >= cap: return
3c.   row = pick_next_queued_excluding(&in_flight).await?
3d.   if no row: return
3e.   guard = try_lock_inflight(row.vod_id)?      # belt-and-suspenders
3f.   set_active_workers(in_flight.len() + 1)     # post-spawn count
3g.   tokio::spawn(async move {
         let _guard = guard;                       # moved in, drops on exit
         process_one(...).await
      })
```

### Mutex poison handling

All three call sites that touch `in_flight`
(`try_lock_inflight`, `in_flight_snapshot`,
`InFlightGuard::Drop`) use the same recovery pattern:

```rust
let mut set = match self.in_flight.lock() {
    Ok(s) => s,
    Err(poisoned) => poisoned.into_inner(),
};
```

A panicking holder elsewhere in the system would otherwise leak the
slot for that `vod_id` permanently, blocking future downloads of
that VOD until the process restarts.  Recovering the data and
continuing keeps the system self-healing.

### `set_active_workers` refresh

`GlobalRate` (ADR-0010) divides the bandwidth budget by an
externally-tracked worker count.  Up to v2.0.2 the count was set
by the per-spawn semaphore-permit math; in v2.0.3 the semaphore is
gone, so without explicit refresh, completed downloads (whose
guards drop) would leave the throttle dividing by a stale higher
count.

The fix: `drain_once` calls
`rate.set_active_workers(in_flight_snapshot().len())` at the top of
every pass.  Drift between drain ticks is bounded at 5 seconds —
acceptable for bandwidth allocation that re-evaluates per-chunk
inside yt-dlp anyway.

## Alternatives considered

### A. File-based lock (`<staging>/<vod_id>.lock` with PID payload)

Discussed in the v2.0.3 mission text: a file with the worker's PID,
checked at startup against `infra::process::liveness`, with
stale-lock cleanup on dead-PID detection.  Rejected.  The lifetime
mismatch (file outlives process; HashSet doesn't) creates a class
of stale-state bugs that the in-memory lock simply doesn't have,
and the AC3 acceptance criteria explicitly call out
"In-Memory-only Lock ist auch akzeptabel und einfacher" with a
CTO recommendation for the in-memory variant.

### B. Atomic claim via `UPDATE ... RETURNING`

Modify `pick_next_queued` to atomically transition state from
`queued` to `downloading` inside a single SQL statement, and use
the returned row.  This is technically the *cleanest* fix — no
in-memory lock needed at all.  Rejected for v2.0.3 because
`UPDATE ... RETURNING` requires SQLite 3.35+ (we're already on a
newer version, but the migration would touch the
`process_one` entry path which currently does the state transition
itself, plus a bunch of test infrastructure).  The in-memory lock
is additive — `process_one`'s state transition stays where it is,
no callers change.  Tracked as v2.1 candidate; if it lands the lock
becomes pure belt-and-suspenders.

### C. `oneshot` channel from the spawned task to the manager

The spawned task could send a message back as soon as its
state-transition lands, and `drain_once` could await that signal
before iterating.  Rejected as serialising the spawn pattern that
was supposed to be parallel; defeats the purpose of `tokio::spawn`.

## Consequences

**Positive.**
- The v2.0.2 frag-rename race (two workers, same VOD) cannot
  happen.  The lock is the wall.
- Crash semantics are simple: an in-memory lock evaporates with
  the process; `crash_recover` handles the DB side.
- The lock generalises to any future code path that wants "is this
  VOD currently being worked on?"  Read via `in_flight_snapshot()`.

**Costs accepted.**
- Two layers of defence (SQL exclusion + lock guard) where one
  would technically suffice.  Acceptable as belt-and-suspenders
  for a P0 hotfix; can simplify in v2.1 if the atomic-claim
  alternative lands.
- `current_concurrency` now reads settings on every tick (every
  5 s).  Adds one async DB read per drain pass.  Negligible.

**Risks.**
- A panicking thread could poison the in-flight mutex; the recovery
  arm keeps the data available but the panic itself is still a
  process-degradation signal worth investigating if it ever appears
  in support logs.
- The `NOT IN (?, ...)` placeholder list grows linearly in the
  in-flight set size.  Bounded at 3 by the cap clamp, so this is
  structurally fine.

## Follow-ups

- **v2.1 candidate:** `UPDATE ... RETURNING` atomic claim
  (alternative B).  Reduces the lock to pure defence-in-depth.
- **v2.1 candidate:** add `in_flight_count()` to the
  `DownloadsSummary` IPC payload so the tray tooltip can show "N
  in flight, M queued" rather than just queue depths.

## References

- `src-tauri/src/services/downloads.rs` —
  `DownloadQueueService.in_flight`, `InFlightGuard`,
  `try_lock_inflight`, `in_flight_snapshot`, `drain_once`,
  `pick_next_queued_excluding`.
- `src-tauri/src/services/downloads.rs::tests` — four
  AC2/AC3 trip-wire tests
  (`try_lock_inflight_blocks_duplicate_claims_same_vod`,
  `try_lock_inflight_allows_different_vod_ids`,
  `pick_next_queued_excluding_skips_locked_vods`,
  `drain_once_respects_concurrency_cap`).
- ADR-0035 (the wiring story this ADR is the AC3 half of)
- ADR-0010 (GlobalRate, the consumer of `set_active_workers`)
