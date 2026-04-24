# ADR-0020 — Cross-streamer deep link

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 5)
- **Related.**
  [ADR-0015](0015-timeline-data-model.md) (the `stream_intervals`
  view the deep link relies on) ·
  [ADR-0018](0018-watch-progress-model.md) (watch-state surface).

## Context

The detail drawer's co-streams panel (Phase 4) lists other
streamers whose VODs overlap the one you're looking at. Phase 5
adds an action per co-stream: "Watch this perspective at HH:MM:SS"
— clicking opens the *other* VOD seeked to the shared wall-clock
moment. Phase 6 then builds split-screen multi-view sync on top of
the same math, with both players locked to a shared timeline.

We want the single-player deep link in Phase 5 so the underlying
math + data surface are proven before the multi-view UI lands.

## Decision

### Wall-clock math lives in `domain::deep_link`

The pure-data `resolve_deep_link_target(ctx)` function takes:

```rust
pub struct DeepLinkContext {
    pub moment_unix_seconds: i64,
    pub target_stream_started_at: i64,
    pub target_duration_seconds: i64,
}
```

and returns a non-negative seek position clamped to the target VOD's
duration:

```rust
let offset = (moment_unix_seconds - target_stream_started_at).max(0) as f64;
offset.min(target_duration_seconds.max(0) as f64)
```

Negative offsets (target started later than the moment of interest)
clamp to 0 so the player lands on the beginning rather than failing.
Offsets past the end clamp to the final frame rather than a NaN.

Time-zone / DST edge cases are handled by the unix-seconds
representation — wall-clock drift during a DST transition is
invisible at the unix layer. A dedicated test
(`dst_transition_is_a_no_op_at_the_unix_layer`) locks this in.

### Entry points

Two surfaces call this math:

1. **Detail-drawer co-streams panel** (Phase 5). The "moment" is
   defaulted to the current VOD's midpoint — a reasonable static
   shortcut when the user opens the drawer without a specific
   timestamp in mind.
2. **Player's live action** (Phase 5 follow-up / Phase 6). The
   moment becomes the player's live `currentTime`, so the user
   can jump to another streamer's perspective from exactly where
   they're watching.

Both funnel through the same `nav-store::openPlayer` action with
an `initialPositionSeconds` override, so the player route reads
a single contract.

### Downloaded-vs-not handling

If the co-stream VOD isn't downloaded yet, the action becomes
"Download and watch @ HH:MM:SS":

1. `cmd_enqueue_download` fires with priority 500 (elevated above
   the default 100 — this is a user-initiated request, not a
   background sweep).
2. `openPlayer` is called with `autoplay: false`. The player route
   renders the `partial` state until the download completes, at
   which point the existing `download:completed` event triggers a
   re-query of `getVideoSource` and the player transitions to
   `ready`.

The elevated priority is the only scheduling tweak — we don't
preempt an in-flight download because that would require a pause +
requeue path that isn't in the contract.

### Phase 6 foundation

The split-screen multi-view sync engine will:

1. Receive a list of co-stream IDs.
2. Open each in its own pane.
3. Lock their `currentTime`s via the same
   `resolve_deep_link_target` math on every seek.

Sharing the domain module means the single-player + multi-player
paths use the same numerics. The state machine for "which pane is
leader" + the audio-mix UI + the pane-per-stream container are the
new Phase-6 bits; the offset computation is done.

## Alternatives considered

### A. Client-side wall-clock math in the frontend

Rejected. Keeping it in the Rust domain layer means (a) exhaustive
unit tests run in the same gate as the rest of the domain, and (b)
Phase 6's sync engine doesn't have to cross the IPC boundary on
every seek.

### B. Per-co-stream "sync offset" table

Rejected. The math is a pure function of fields we already have
(`stream_started_at`, `duration_seconds`). A dedicated table would
be cache that quietly rots whenever a VOD's metadata is re-ingested.

### C. Round-trip server-side for every seek

Rejected for Phase 6 implications. The pane-sync loop needs to
react at 60 fps; an IPC round-trip per frame is unacceptable. Pure
math the frontend can mirror (with the Rust version as the canonical
source) is the only viable path.

## Consequences

**Positive.**
- The Phase-5 deep-link action is a ~100-line UI surface on top of
  a 10-line math function; the risk surface is tiny.
- Phase 6 starts with the offset math already proven — the sync
  engine's novelty is the pane-state machine, not numerics.
- Downloaded-vs-not is a single code path: enqueue + open + let the
  state machine catch up. No special-case waiting loops.

**Costs accepted.**
- Elevated priority is a blunt tool. If a user triggers the
  download-and-watch action while an existing download is mid-
  flight, the new one will run after it rather than preempting —
  acceptable because the existing behaviour was already "queue
  by priority descending".
- The UI surface for the deep-link label uses the same math twice
  (backend + frontend display). A single drift point is tolerable
  given the math is five lines; widening either requires updating
  both sites.

## Follow-ups

1. Per-frame sync (Phase 6) — the loop that calls
   `resolve_deep_link_target` on each `timeupdate` when two panes
   are linked.
2. Audio-mix UI (Phase 6) — per-pane volume + crossfade, orthogonal
   to this ADR.
3. Deep-link shareable URLs — a future feature would let a user
   paste a "watch perspective B at 01:23:45" URL. Would need a
   schema (sightline://watch?vod=...&t=...) + a Tauri deep-link
   plugin. Out of scope for v1.
