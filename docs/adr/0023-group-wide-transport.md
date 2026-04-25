# ADR-0023 — Group-wide transport & leader election

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 6)
- **Related.**
  [ADR-0021](0021-split-view-layout.md) (the layout this transport
  bar sits inside) ·
  [ADR-0022](0022-sync-math-and-drift.md) (the sync model the
  transport must coordinate with).

## Context

Multi-view v1 has two panes locked to a shared wall-clock. Two
people can't manipulate transport independently — that's the whole
point. We need to define exactly what play / pause / seek / speed
mean when applied to a group, and how the "leader" pane (the one
whose `currentTime` is the truth) is chosen and changed.

## Decision

### Transport semantics

All four transport actions are **group-wide**. There is exactly one
play/pause state, one wall-clock seek slider, one speed selector.

- **Play / Pause** — fans to every pane's `<video>` element.
  `play()` is best-effort: a follower whose autoplay was rejected
  (rare since the user already gestured to enter multi-view) shows
  a small "Click to start" hint, but the leader still plays.
- **Seek** — slider is wall-clock anchored. Sliding the slider
  computes a new `moment`. The leader's `currentTime` is set to
  `moment - leader.vodStartedAt`, clamped to its duration.
  Followers correct via the next sync-loop tick (ADR-0022).
- **Speed** — `setSpeed(rate)` fans to every pane's
  `playbackRate`. v1 supports the same speeds as the single-player
  (0.5×, 0.75×, 1×, 1.25×, 1.5×, 1.75×, 2×).

Per-pane transport is *out of scope* for v1. A pane has no
"play/pause" button of its own — only volume + mute, leader
promotion, and the close-pane action are pane-local.

### Leader election

Default leader is the first opened pane (the primary VOD the user
clicked). The user can promote either pane to leader via a "Promote
to leader" affordance in the pane chrome.

Promotion fires `cmd_set_sync_leader({ sessionId, paneId })`. The
backend updates `sync_sessions.leader_pane_id`, emits
`sync:leader_changed`, and the frontend's sync store refreshes.

A leader change does *not* re-anchor the moment. If pane A is
leader at `currentTime = 120s` and the user promotes pane B,
pane B's existing `currentTime` becomes the new source of truth —
which means a subsequent sync-loop tick will correct A toward B.
This is the correct semantic: the user picked B because they want
B's clock to be authoritative.

### Group-wide transport edge cases

#### Leader pane closed mid-session

In v1 the only way to close a pane is to close the whole session
(no per-pane close button — see ADR-0021). So the question of "what
happens when the leader is removed" only arises if a v2 patch adds
per-pane close. We handle the v1 case implicitly: closing the page
ends the session.

If a future change adds per-pane close, the contract is:

- If the leader is the closing pane and another pane remains, the
  remaining pane becomes leader. Position carries forward.
- If the leader is the closing pane and no pane remains, the
  session closes.

The backend service includes a `set_leader` API that supports this
transition cleanly when the time comes; we do not write the
per-pane close UI in v1.

#### One pane out of range

Per ADR-0022: an out-of-range follower pauses its `<video>` and
displays a hint overlay; the leader keeps playing. The transport
bar continues to drive the leader. If the user seeks the slider
back into the follower's range, the follower resumes on the next
sync-loop tick.

If the *leader* would go out of range, that's only possible when
the user has chosen a leader whose VOD is shorter than the group
duration *and* seeked past its end. In v1 we clamp the seek to the
leader's duration (the seek bar's max is `leader.duration`), which
makes this case unreachable. ADR-0022 owns the math.

#### Speed change with mid-tick correction

A speed change is a single transport action, not a per-pane
adjustment. The sync loop runs at 250 ms regardless of speed; the
threshold is in *wall-clock* milliseconds, not playback
milliseconds, so the loop's correction policy is unchanged.

(A 2× speed user is consuming wall-clock at 2× the rate, but the
drift threshold remains 250 ms of *real* time. This is the right
behaviour — we want the audio of pane A and pane B to be in sync
on the listener's clock, not on the playback clock.)

### UI affordances

- The transport bar lives at the bottom of `/multiview`.
- A leader badge (crown icon, monochrome) appears on the leader
  pane. Hover/focus reveals a "Promote to leader" tooltip on the
  *non-leader* pane, with a button.
- The seek slider's max is `leader.durationSeconds`. Its current
  value is `leader.currentTime`. Followers are not visualized on
  the slider — the unit is the leader's clock, mirroring how the
  single-player slider works today.
- A "Close group" button in the page header ends the session
  (closes the multi-view, releases the panes' resources).

### Persistence

Per ADR-0021 / migration 0010, the session row is persisted but the
live state (currentTime per pane, drift history) is not. v1 has no
"resume" UI — the persistence is the audit foundation v2 may use.

## Alternatives considered

### A. Per-pane transport with optional sync lock

Rejected. The whole point of multi-view is the sync. A "lock" toggle
adds a mode-bit the user has to track and an off-by-one chance for
desync. Group-wide is the simpler default; we can add per-pane in v2
behind an explicit "unlock pane" gesture if a real use case appears.

### B. No leader concept — sync to wall-clock midpoint of all panes

Rejected. Without a leader, every pane is being corrected toward a
moving target derived from its own measurement, which is exactly the
control-loop instability we avoid in ADR-0022. Picking one pane as
the truth is the cleanest model.

### C. Auto-elect the longest VOD as leader

Rejected. The user almost always wants the *primary perspective* as
leader (the streamer they were watching), which is captured by
"first opened" in our entry flow. Auto-electing by duration would
override that.

### D. Persist live state for "resume multi-view session"

Deferred. The DB row already exists post-migration 0010; the resume
UI is a v2 addition. The Phase-6 prompt is explicit that this is out
of scope.

## Consequences

**Positive.**
- One transport bar, one set of states, one source of truth per
  group. The mental model is "watch this perspective at HH:MM:SS,
  but with a second perspective tagging along".
- Leader changes are a single backend command + event; the UI
  surface is one button per non-leader pane.
- Edge cases are either provably unreachable (leader goes out of
  range) or have a clean fallback (out-of-range pane pauses + hint).

**Costs accepted.**
- A user who genuinely wants to scrub one pane independently
  cannot. They can leave multi-view, scrub, and re-enter with a
  fresh wall-clock anchor. v1 prioritises sync correctness over
  per-pane freedom.
- The seek slider's resolution is the leader's duration. If the
  follower is shorter, parts of the slider land in the follower's
  out-of-range state — the user sees the hint and can re-target.
- The "first opened" leader default depends on the entry flow
  ordering. The detail-drawer multi-select picks the *primary*
  (open detail-drawer) VOD as primary, which is the right
  ergonomic; an entry from a (future) library multi-select needs to
  preserve the ordering.

## Follow-ups

1. Per-pane "unlock" mode (v2). The backend has the data; the UI
   gesture and the dual-state of "locked vs. unlocked panes" is
   the new design.
2. Resume previous multi-view session (v2). DB row already exists;
   needs a "Recent multi-view sessions" surface.
3. Crossfader audio mix (v2). Orthogonal to transport but lands in
   the same transport bar real estate.
4. > 2 panes (v2). The leader-election model already supports
   N panes; we'd extend the UI affordances per pane.
