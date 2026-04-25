# ADR-0022 — Sync math & drift correction

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 6)
- **Related.**
  [ADR-0020](0020-cross-streamer-deep-link.md) (the wall-clock math we
  reuse) ·
  [ADR-0015](0015-timeline-data-model.md) (`stream_intervals` for
  overlap discovery) ·
  [ADR-0021](0021-split-view-layout.md) (layout) ·
  [ADR-0023](0023-group-wide-transport.md) (transport semantics).

## Context

Phase 5 proved the wall-clock offset math
(`domain::deep_link::resolve_deep_link_target`) for the single-player
"watch this perspective at HH:MM:SS" deep link. Phase 6 multi-view
runs that math continuously: the leader pane's wall-clock position
is the truth, every follower pane periodically corrects to match.

We need to lock down three numbers and one model:

1. The **drift tolerance** — how far apart two panes can drift before
   we re-sync.
2. The **sync-loop cadence** — how often we measure and correct.
3. The **correction strategy** — seek vs. rate-adjust.
4. The **out-of-range model** — what a pane does when the leader's
   wall-clock falls outside its VOD's time window.

## Decision

### Wall-clock anchor model

Each pane has a constant `vodStartedAt` (the VOD's
`stream_started_at`). The shared "moment" is the leader pane's
wall-clock position:

```
moment = leader.vodStartedAt + leader.currentTime
```

Each follower's expected `currentTime` is computed via the same
`resolve_deep_link_target` math:

```
expected_follower = clamp(0, moment - follower.vodStartedAt, follower.duration)
```

The Rust function in `src-tauri/src/domain/deep_link.rs` is the
canonical source. The frontend mirrors it identically — the v1
implementation is a 3-line TS function with the identical clamp
behaviour, *not* an IPC round-trip per tick (an IPC call per pane per
250 ms is acceptable cost-wise, but the math is small enough that
keeping it client-side avoids the I/O entirely and lets us
unit-test both sites against the same JSON test vectors). If the
frontend ever drifts, the test fixture catches it.

### Drift tolerance

**250 ms.** Within tolerance, no action. Above tolerance, correct via
a seek.

Reasoning: 250 ms is just below the threshold where a viewer notices
audio/video desync between two non-co-located streamers. Going lower
(e.g. 100 ms) means correcting more often without a perceptible win;
going higher (e.g. 500 ms) starts to feel "off" during dialogue
beats.

The threshold is configurable via `app_settings.sync_drift_threshold_ms`
(50–1000 ms), so a power user with a different sensitivity can tune
it. The default is the production value.

### Sync-loop cadence

**setInterval/setTimeout @ 250 ms tick — explicitly NOT
`requestAnimationFrame`.**

Reasoning: drift detection is a low-rate problem. The two `<video>`
elements run their own decoders at native frame rate — we are not
trying to keep them aligned to a frame boundary, we are checking
whether their wall-clock positions have diverged beyond a threshold
that itself is at the 250 ms scale. rAF would fire ~60× per second
(60 fps) and burn CPU + battery for a measurement that doesn't change
that fast. A 250 ms tick matches the threshold and is the right rate.

If field testing shows visible UI stutter (e.g. the leader
indicator updates feel choppy because of a 250 ms latency), we can
drop to 125 ms before reaching for rAF. Going to rAF for the
sync loop would require a separate decision logged in the decision
log — it has CPU/battery implications that an end-user report alone
shouldn't justify.

When the page is hidden (`document.visibilityState === 'hidden'`)
the loop pauses entirely. The browser already throttles
`<video>` decoders heavily in background tabs, so any drift
correction we'd compute would be against stale numbers.

### Correction strategy

**Seek-only.** When `|follower.currentTime - expected| > threshold`,
set `follower.currentTime = expected` directly. The two `<video>`
elements briefly desync during the seek — perceptually a single
frame's worth — but we accept that over the alternative of fiddling
with `playbackRate` to gradually catch up.

We rejected rate-adjust (slow the leader / speed up the follower
until aligned) because:

- The user-facing playback rate is part of group transport
  (ADR-0023). Mixing automatic rate adjustments with the user's
  explicit speed setting creates a race: the user changes speed,
  the sync loop changes speed, the speed indicator flickers.
- Rate-adjust drift correction is open-loop control and tends to
  oscillate without a damping term. A seek is closed-loop in one
  step.

The seek correction is rate-limited: at most one correction per
pane per 1 second wall-clock, even if the loop tick runs every
250 ms. This stops a runaway feedback loop where the seek itself
introduces a measurement that triggers another seek.

### Out-of-range behaviour

A pane is "out of range" when the leader's `moment` falls outside
`[follower.vodStartedAt, follower.vodStartedAt + follower.duration]`.

When out of range:

- The pane pauses its `<video>` element.
- The pane shows an "Out of range — Sightline is waiting for
  alignment" overlay.
- The pane stops contributing to drift correction (it's the
  follower; the leader keeps playing).
- When `moment` re-enters the pane's range, the pane resumes,
  seeks to the correct expected position, and re-joins the sync
  loop.
- An IPC event `sync:member_out_of_range` fires once per
  out-of-range transition (debounced — repeated state assertions
  inside a 1-second window are coalesced).

The leader pane never goes out of range from its own perspective —
its `currentTime` is the source of the moment, by construction.

### Performance budget

The sync loop (frontend, JS) targets:

- ≤ 1% CPU at idle (page mounted, both panes paused).
- ≤ 5% CPU during active playback on the bench machine
  (M1 Mac mini, dev profile).

If we exceed those budgets at the chosen 250 ms cadence we have
under-estimated the work, in which case we either reduce work per
tick (skip correction when both panes are paused) or escalate to
the CTO before changing the cadence.

## Alternatives considered

### A. rAF-based loop with finer correction

Rejected. 60 fps is over-engineering for a 250 ms threshold.
See "Sync-loop cadence" above.

### B. Use `playbackRate` to gradually catch up

Rejected. Conflicts with the user's explicit speed setting
(ADR-0023) and tends to oscillate without a damping term.

### C. IPC round-trip per pane per tick

Rejected. The math is too small to be worth the IPC cost. Mirroring
the math in the frontend with a JSON test fixture both sides verify
against is the standard pattern (ADR-0020 set the precedent for the
single-player deep link).

### D. Predictive sync (extrapolate from the last few ticks)

Rejected for v1. A simple correctional seek is good enough for two
panes whose decoders both run at the same native rate. Predictive
sync helps at higher pane counts and across slower networks, neither
of which v1 has.

## Consequences

**Positive.**
- A single number — 250 ms — drives both threshold and cadence.
  Easy to reason about and easy to instrument.
- Pure-math, identical Rust + TS implementations testable against
  shared JSON fixtures.
- The rate-limited seek correction means the user can't end up in a
  thrash loop.
- Out-of-range is a visible UI state, not silent desync.

**Costs accepted.**
- A user who skips the leader by ≥ 250 ms incurs a perceptible
  follower seek. We accept this — the alternative is no correction.
- Two `<video>` elements briefly desync during a corrective seek.
  Acceptable for a low-rate (≤ 1/s/pane) event.
- The setInterval-based loop drifts slightly under load. We accept
  the drift because the threshold is the same order of magnitude.

## Follow-ups

1. If the production data shows pane drift over 250 ms in steady
   state, profile the loop and either (a) tighten the threshold or
   (b) add a damping term — both ADR-worthy in their own right.
2. Audio crossfade (v2) introduces a new failure mode: the
   crossfade timing must itself be wall-clock anchored. The math
   here generalises; the UI is the new ADR.
3. > 2 panes (v2) needs the leader-election model to handle
   multiple followers. The math is unchanged; the loop's per-tick
   cost scales linearly with pane count.
