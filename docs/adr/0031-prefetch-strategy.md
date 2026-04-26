# ADR-0031 — Pre-fetch strategy

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0030](0030-pull-distribution-model.md) (the state machine the
  pre-fetch acts on) ·
  [ADR-0018](0018-watch-progress-model.md) (watch-progress signals
  drive the pre-fetch trigger).

## Context

ADR-0030's pull model means a user opens VOD K and starts watching;
when they reach the end, they want K+1 to "just be there".  Without
pre-fetch, the next-VOD experience is a 2-5 minute download wait.
With pre-fetch, the system uses the time the user is watching K to
fetch K+1 in the background.

But pre-fetch has to be conservative — it's the "auto-download for
power users" feature in disguise.  If pre-fetch is too eager (fetches
K+2, K+3 on the assumption the user will keep watching) we're back
in v1.0's surprise-the-disk regime.

## Decision

A single-VOD lookahead with explicit triggers and clear bounds.

### Trigger

The player emits a `watch:progress_updated` event every ~5 seconds.
The first such event for a given session (or any event if no
prefetch has been triggered yet for this session) calls
`distribution.prefetch_check(currently_watching_vod_id)`.  The check
runs the following decision tree:

1. **Setting check.**  If `prefetch_enabled = false` or
   `distribution_mode = 'auto'`, no-op (auto mode already pulls
   everything; the toggle is for advanced users who want the pull
   discipline).
2. **Window check.**  If the streamer's `(ready + queued)` count is
   already at or above `sliding_window_size`, no-op.  The user has
   enough buffered.
3. **Find candidate.**  Query for the next-most-recent `available`
   VOD on the same streamer, ordered by `stream_started_at` ASC and
   restricted to VODs with `stream_started_at` > the current one
   (i.e., the *next* episode chronologically).  The currently-watching
   VOD plus everything older is excluded because:
   - Older VODs are typically the user backfilling history; pre-
     fetching one means we'd race with their explicit pick.
   - The "next" intuition (what comes after this one) maps to the
     streamer's chronology, not the listing order in the Library UI.
4. **Pick.**  If a candidate exists, transition it to `queued`.  The
   download worker pool picks it up the same way an explicit
   `pickVod` would.

The decision tree runs at most once per VOD per session — a per-
session in-memory `HashSet<vod_id>` of "we've already tried" guards
against the 5-second progress-event cadence triggering a thousand
prefetch_check calls.

### Bounds

- **One look-ahead.**  Pre-fetch never queues K+2 even if K+1 is
  already `ready`.  The user's explicit progression to K+1 is what
  triggers K+2's pre-fetch.
- **Per-streamer.**  Cross-streamer pre-fetch (e.g., user is watching
  Streamer A, pre-fetch a Streamer-B VOD) is out of scope — the
  signal that the user wants more of A is intrinsic to A.
- **Window-respecting.**  Pre-fetch stops when the per-streamer
  window cap is hit, identical to explicit picks.

### Backoff conditions

Pre-fetch is silently skipped when any of the following hold:

- The candidate is `is_sub_only = true` (yt-dlp will fail; surfacing
  the failure as a "pre-fetch error" in the UI is more confusing than
  helpful).
- The high watermark (Phase-7 auto-cleanup) is currently exceeded —
  the user is in disk-pressure territory and pre-fetch would worsen it.
- A `cleanup` run is in progress (the `executing` AtomicBool is set);
  pre-fetch waits for the next progress tick.
- An explicit `pickVod` race: if the candidate transitioned to
  `queued` between our check and our update, the SQLite UPDATE's
  WHERE clause filters it out.  No-op.

### Events

`distribution:prefetch_triggered` fires once per actual transition.
Useful for the Settings UI's "Last pre-fetch" line and for support
debugging.  No event when pre-fetch is skipped.

## Alternatives considered

### A. K+N lookahead (N ≥ 2)

Considered, rejected.  Two-step lookahead doubles the pre-fetch
footprint without doubling the value.  The user explicitly progressing
to K+1 is what triggers K+2's pre-fetch under the chosen model — same
end-state, less speculation.

### B. Pre-fetch on explicit "Watch" click instead of timeupdate

Considered.  Pre-fetch on click means K+1 starts downloading the
moment the user opens K.  Pro: the user gets K+1 ready slightly
sooner.  Con: a user who opens K to "see what it is" and closes it
at the 30-second mark has caused a 1.5 GB download.  The 5-second-
into-watching trigger is the better signal: by then we know the
user actually started watching.

### C. Streamer-frequency-weighted pre-fetch

A streamer who streams daily has K+1 only a day away; a weekly
streamer's K+1 is a week away.  Argue: pre-fetch only the "current
streamer".  Currently we already pre-fetch only the streamer whose
VOD the user is watching, so this is moot — but the rationale
matters for documenting why we don't need a frequency parameter.

### D. UI signal "pre-fetch this" / "skip pre-fetch"

Out of scope.  The setting toggle (`prefetch_enabled`) is the only
user-facing control in v2.0.  Per-VOD overrides are tracked as
follow-ups but not blocking the release.

### E. Time-based pre-fetch (every 60s of watch time, top up)

Rejected.  The user reaching the end of a 6-hour VOD is not
predictable from watch-time alone (skip-around behaviour, paused
watching, etc.).  The trigger should be cheap and idempotent — first
sustained watch event triggers once, end of story.

## Consequences

**Positive.**
- "Open the next episode and it's already there" — the killer UX of
  pull-mode without giving up the disk discipline.
- Bounded: one lookahead × one streamer × respecting window means
  the worst-case extra disk is exactly one VOD per active watch
  session.

**Costs accepted.**
- A user who closes a VOD 6 seconds in caused a pre-fetch they may
  not want.  The unpick-after-fact path exists but the user has to
  notice and act.
- The per-session HashSet of triggered VODs lives in memory; a daemon
  restart re-triggers pre-fetch on the next progress event.  This is
  benign — it just means a new pre-fetch attempt that's likely a
  no-op (window already full).

**Risks.**
- A streamer with very long VODs (8h+) and a fast pre-fetch download
  could fill the window faster than the user can watch through it,
  leading to `archived` accumulating.  The sliding-window enforcer
  handles this — archived ages out as new ones come in.
- The "sub_only" skip is a hard skip; if a streamer alternates
  sub-only and public VODs, pre-fetch picks the next *public* one,
  which may not chronologically be K+1.  Acceptable.

## Follow-ups

- "Always keep N ahead" mode — explicitly Auto-download via the
  pull-mode plumbing.  Nice-to-have for power users.
- Cross-streamer pre-fetch when the user has a habit of watching
  multiple streamers' VODs from the same time period (the multi-view
  use case).  Tracked for v2.1.
- Pre-fetch progress UI in the player ("K+1: 30% downloaded").  v2.1.

## References

- `src-tauri/src/services/distribution.rs::prefetch_check`
- `src/features/player/usePrefetchHook.ts`
- ADR-0030 (the state machine pre-fetch operates on)
- ADR-0018 (watch-progress events that trigger us)
