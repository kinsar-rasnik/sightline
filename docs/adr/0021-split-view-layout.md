# ADR-0021 — Split-View v1 layout & UI topology

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 6)
- **Related.**
  [ADR-0019](0019-asset-protocol-scope.md) (player paths flow through
  `MediaAssetsService::get_video_source`, reused by every pane) ·
  [ADR-0020](0020-cross-streamer-deep-link.md) (the wall-clock math
  Phase 5 proved end-to-end). ADR-0022 owns the sync math; ADR-0023
  owns transport semantics.

## Context

Phase 6 turns the cross-streamer deep link into a true multi-view
experience: pick two co-streams, watch them side-by-side, locked to a
shared wall-clock. The earlier roadmap considered three layout
families — split, picture-in-picture, and N-up grids. The Phase-6
prompt set the scope explicitly: ship a v1 with two panes, leave the
rest for v2. This ADR locks in *what* the v1 surface looks like so
ADR-0022 (math) and ADR-0023 (transport) can refer to a stable shape.

The user is a single-monitor, mostly mouse-and-keyboard viewer of
GTA-RP role-play. Two side-by-side perspectives is the canonical
"watch with the other party" use case the genre demands; PiP / 2×2
grids are nice-to-have.

## Decision

### Layout

Two panes, fixed 50/50 split, horizontal. CSS Grid with two
`1fr` columns. The split is non-resizable in v1 — a draggable divider
is a v2 affordance and gives the user nothing actionable when both
panes hold a fixed-aspect `<video>` element.

The page lives at the new `/multiview` "route" (the app uses an
in-memory `nav-store` rather than a router; we add `multiview` to the
`NavPage` union). One pane container per session; closing the page
closes the session.

### Composition

`MultiViewPage` mounts two `MultiViewPane` children. Each pane wires:

- A `<video>` element. The same `getVideoSource` IPC call the
  single-player uses; same `assetUrl` conversion. No path
  construction in the renderer (ADR-0019).
- An optional pane-chrome overlay: leader badge, "Promote to leader"
  button, per-pane volume + mute, an "Out of range" hint when the
  shared wall-clock falls outside the pane's VOD range.
- An imperative ref to the `<video>` exposed up to the page so the
  sync loop and group transport can read `currentTime` / call
  `play()`/`pause()` without prop-drilling each callback.

A page-level transport bar at the bottom owns play/pause, the seek
slider (a single bar across both panes — wall-clock anchored), and
the speed selector. Per-pane transport is *not* part of v1
(ADR-0023).

### State surface

A `multiview-store` (Zustand) holds:

- `sessionId`, `panes[]` (two entries: vodId, paneId, volume, muted),
  `leaderPaneId`, `status`, `driftHistory`.
- Actions: `openSession`, `closeSession`, `setLeader`, `setVolume`,
  `setMuted`, transport stubs (`play`, `pause`, `seek`, `setSpeed`)
  that fan to the backend service via IPC and emit local optimistic
  updates.

Live frame-by-frame state (each pane's `currentTime`, last drift
measurement) lives in component refs / state — persisting it would
serve no v1 use case.

### Entry point

The library detail drawer's existing co-streams panel (Phase 5,
single-player deep link) gains a multi-select mode: a checkbox per
co-stream and an "Open Multi-View" action that fires
`openSyncGroup({ vodIds: [primary, secondary] })`. This reuses the
co-streams query that already powers the cross-streamer deep link.
Full grid-level multi-select is a v2 feature.

### Out of scope (v1)

- Picture-in-picture layout. Window-blur PiP via the player's
  existing `pipOnBlur` setting still works in single-player; in
  multi-view we leave `pipOnBlur` off when the page is mounted.
- More than two panes.
- Resizable split.
- Crossfader audio mix (v1 ships per-pane volume + mute only).
- Per-pane transport (v1 is group-wide, see ADR-0023).
- Shareable deep-link URLs (sightline://multiview?...). Future.

## Alternatives considered

### A. Picture-in-picture (one main + one inset)

Rejected for v1. PiP works for "I'm focused on streamer A and want a
glance at streamer B"; the canonical RP scene the user is trying to
understand is a *conversation*, where both perspectives carry equal
information. The split layout treats the two streams as peers, which
is the right default. PiP can land in v2 as a layout switch.

### B. 2×2 grid (four panes)

Rejected for v1. Four-perspective scenes exist but are rare; the
sync, layout, and audio-mix complexity scale super-linearly (each
pair of panes is a sync pair to keep aligned). Two panes lets us
prove the engine before paying for the grid case.

### C. Resizable split

Deferred. The two `<video>` elements are letterboxed — sliding the
divider just changes the black-bar ratio, not the visible video
area. We can add a divider in v2 if the timeline overlay or chapter
strip asks for asymmetric weighting.

### D. Multi-select on the library grid as the primary entry

Deferred. The detail-drawer co-streams panel already enumerates
overlapping streams with overlap-length sorting; that's the natural
launch surface. Library-grid multi-select would require a checkbox
mode + "open as multi-view" CTA that competes with the existing
single-VOD click action. Adding it later is purely additive.

## Consequences

**Positive.**
- Single layout × single pane count keeps the v1 surface narrow:
  one CSS Grid template, one transport bar, one sync loop.
- Reusing `getVideoSource` + `assetUrl` means the asset-protocol
  scope (ADR-0019) covers multi-view automatically — no new path
  validation or capability.
- The detail-drawer entry leans on data the user already paid for
  (the `getCoStreams` query). No new ingest.
- Closing the page == closing the session. No background sync
  workers, no resource leaks, no orphaned UI state.

**Costs accepted.**
- Two panes is a hard limit in v1. A v2 expansion will need a
  different layout family (a row of N panes or a flex grid) and
  the multi-select UI grows accordingly.
- Per-pane transport is unavailable. A user who wants to peek at
  pane B's preceding moment without dragging both panes back will
  have to leave multi-view. We accept that trade for the simplicity
  of a single transport bar.
- The session row is persisted to `sync_sessions` (see the data-
  model update for migration 0010), but v1 has no "resume previous
  multi-view" UI. The persistence is the audit/replay foundation
  v2 may use.

## Follow-ups

1. PiP layout option (v2). Can ship as a layout selector on the
   transport bar; the underlying engine is identical.
2. > 2 panes (v2). Split layout becomes a row/grid; the leader-election
   model already accommodates more panes, only the UI grows.
3. Library-grid multi-select (v2). Adds a checkbox mode and an
   "Open Multi-View" CTA above the grid.
4. Shareable deep-link URLs (v2/v3). Needs a sightline:// scheme
   handler — out of scope for the desktop-only Phase 6.
