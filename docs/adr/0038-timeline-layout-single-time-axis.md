# ADR-0038 — Timeline-Layout: Single-Time-Axis with Lane-Packing

- **Status.** Accepted 2026-05-12
- **Date.** 2026-05-12 (drafted PROPOSED) → 2026-05-12 (accepted by CEO sign-off)
- **Acceptance.** CEO sign-off 2026-05-12 with four refinements landing in the
  body — CEO-A1 (Multiview pane-count → 4, render-logic delegated to a
  separate Multiview-Pane-Expansion-ADR; D5/D7 alignment), CEO-A2
  (multi-perspective threshold 75 % → 50 %; D5 substance correction, with
  Late-Join GTA-RP coverage as the carrying argument), CEO-A3 (hero-slot
  layout-reservation trigger pinned to *favourited* live, not all-tracked
  live; promoted from a Costs-accepted mitigation to a stand-alone
  decision — new **D9**), CEO-A4 (Q-6 keyboard cascade confirmed:
  Tab-focus drives the same 0.6–1.0 s GIF-strip delay as mouse-hover;
  ADR-VodCard need not re-open Q-6). Per-decision rationale and Folge
  in `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6.
- **Wave.** v2.1 ADR Wave 1 of (Timeline-Layout → VodCard → VisualSystem-v2).
  Parallel v2.1 wave: Multiview-Pane-Expansion-ADR (CEO-A1) — to be filed
  separately, referenced from this ADR as a contract.
- **Related.**
  [ADR-0015](0015-timeline-data-model.md) (`stream_intervals` + IPC) ·
  [ADR-0018](0018-watch-progress-model.md) (3-px progress stripe consumes
  watch-progress) ·
  [ADR-0021](0021-split-view-layout.md) (Multiview entry surface
  conventions) ·
  [ADR-0022](0022-sync-math-and-drift.md) (250 ms tick precedent the
  Now-indicator's tick choice mirrors) ·
  [ADR-0023](0023-group-wide-transport.md) (Multiview transport contract
  the selection-CTA crosses). Anchor:
  `docs/reference/visual-language-netflix.md` §3.6 (Single-Time-Axis,
  binding) + §6.1 + §7. Decision-log:
  `docs/decision-log/v2.1-anchor-acceptance.md` (Q-1, Q-2, Q-4, Q-5,
  Q-10).

## Context

The Phase-4 `TimelinePage` (`src/features/timeline/TimelinePage.tsx`,
v1.0 onwards) renders one horizontal lane per streamer with stream
bars positioned by `(startAt, endAt)`. It is faithful to the
Phase-4 mission ("prove the data layer, show overlap"), but it is
structurally the wrong frame for v2.1.

Per **§3.6 of the Netflix-anchor document** — binding spec since
CEO sign-off 2026-05-12 — Sightline's canonical browse surface is
the **Single-Time-Axis** pattern:

1. A horizontal time axis runs across the viewport, labelled with
   date / hour markers at the active zoom level.
2. VOD cards float above and below the axis, anchored at their
   `startAt` position.
3. Overlapping coverage windows pack into parallel lanes — lanes
   are not pre-assigned to streamers, they are a **packing result**
   of overlap.
4. The pattern is **not** a modification of §3.5 Horizontal Rail —
   H-25 deliberately N/A. Card visual language follows H-13 through
   H-22 unchanged.

The structural correction logged for **Q-4** (`docs/decision-log/
v2.1-anchor-acceptance.md` §2) makes the H-25 ↛ §3.6 transition
explicit. The CEO Decisions on **Q-1** (avatar-reveal sidebar),
**Q-2** (conditional Happening-Now hero), **Q-5** (multi-perspective
stack silhouette), and **Q-10** (no Continue-Watching rail on
Timeline; 3-px progress stripe per card) further pin the pattern
and reject several "obvious" alternative shapes — most notably
the one-rail-per-streamer that the current implementation is.

The `stream_intervals` schema (ADR-0015) is sufficient for §3.6 —
`(start_at, end_at, streamer_id, vod_id)` is exactly the input the
lane-packing algorithm needs. No data-layer change is required. The
IPC surface (`cmd_list_timeline`, `cmd_get_co_streams`,
`cmd_get_timeline_stats`) likewise stays unchanged. The work in v2.1
is entirely on the React renderer + a pair of new Zustand stores.

This ADR locks the **structural, algorithmic, and state-ownership**
decisions that translate §3.6 into runnable code, without picking
visual tokens (those go to ADR-VisualSystem-v2, Wave 3) or
specifying the VOD card's hover/preview behaviour (ADR-VodCard,
Wave 2). Where a decision in this ADR brushes a Wave-2 / Wave-3
surface, the decision is normatively bounded ("hover behaviour
delegated to ADR-VodCard; matches H-17/H-18/H-22") rather than
re-litigated.

---

## Decisions

### D1 — Lane-Packing Algorithm

**Decision.** Greedy Best-Fit, alternating above / below the axis.
The algorithm walks intervals sorted by `startAt` ascending and, for
each interval, picks the **side** (above / below) with fewer
currently-active lanes, then picks the **first lane on that side**
whose tail (`lastEndAt`) precedes the new interval's `startAt`. If
no existing lane fits, a new lane opens at the bottom of that side.

**Complexity.** O(n · log L) where `L` is the active-lane count
(per side, bounded by D4's `N_max`). For Sightline's expected
worst-case load (~ 30 streamers × 14 days retention × ≈ 5 VODs /
streamer / day ≈ 2 000 intervals in a 30-day window) this is on
the order of 20 000 comparisons — sub-millisecond in JS.

**Rationale.**

1. **Stable under streaming updates.** A new VOD ingested
   mid-session assigns to an existing lane or opens a new lane at
   the side with fewer occupants; **no existing assignment shifts**.
   The user does not watch the entire timeline re-shuffle when a
   poll completes — critical for cognitive continuity.
2. **Above / below balanced.** Alternation by side-occupancy keeps
   the axis visually centred. The eye reads the axis as the spine
   of the layout rather than as the floor under a top-heavy stack.
3. **Predictable enough to memoize.** Same input ⇒ same output.
   `useMemo(packIntervals, [intervals])` is a one-shot cache hit
   across re-renders that don't touch the data.
4. **Cheap enough to recompute on every data update.** The
   alternatives that try to be smarter (Best-Fit global,
   chromatic-optimal) either re-shuffle on update or cost more for
   no perceived UX win at our N.

**Pseudo-code.**

```ts
type Lane = { tailEnd: number };
type LaneAssignment = { vodId: string; side: 'above' | 'below'; laneIdx: number };

function packIntervals(intervals: Interval[]): LaneAssignment[] {
  const sorted = [...intervals].sort((a, b) => a.startAt - b.startAt);
  const above: Lane[] = [];
  const below: Lane[] = [];
  const result: LaneAssignment[] = [];

  for (const iv of sorted) {
    // Side selection: balance side-occupancy.
    const side: 'above' | 'below' = above.length <= below.length ? 'above' : 'below';
    const lanes = side === 'above' ? above : below;

    // First-fit on chosen side.
    let laneIdx = lanes.findIndex(l => l.tailEnd <= iv.startAt);
    if (laneIdx === -1) {
      lanes.push({ tailEnd: iv.endAt });
      laneIdx = lanes.length - 1;
    } else {
      lanes[laneIdx]!.tailEnd = iv.endAt;
    }
    result.push({ vodId: iv.vodId, side, laneIdx });
  }
  return result;
}
```

**Fallback for Engineering implementation.** If field testing shows
"deep stacks" (a single instant with > N_max overlap producing
unreadable density), fall back to **Interval Graph Coloring** with
a stable tie-break by `streamer_id`. The pure-function module
exposes both implementations behind a feature flag; default is
Best-Fit, the coloring variant is opt-in via an Engineering setting.

### D2 — Time-Axis Scale and Zoom Levels

**Decision.** Six discrete zoom levels with continuous-during-gesture
interpolation; gesture commits to the nearest discrete level on
release. Default at first render: **24 h (day view)**.

**Levels** (viewport-width of time):

| Level | Span      | Axis labels                              |
| ----- | --------- | ---------------------------------------- |
| 1     | 15 min    | minute markers + hour anchor             |
| 2     | 1 h       | minute markers + hour anchor             |
| 3     | 6 h       | hour markers + date anchor at 00:00      |
| **4** | **24 h**  | **hour markers + date anchor at 00:00**  |
| 5     | 7 d       | day markers + week anchor                |
| 6     | 30 d      | week markers + month anchor              |

**Interactions.**

- **Trackpad pinch / `Cmd + scroll`** — interpolates the time-scale
  continuously between adjacent discrete levels. On release, snaps
  to the closer level.
- **Buttons (`+`, `−`)** in the chrome — jump to adjacent discrete
  level.
- **Keyboard `[` / `]`** — same as buttons.
- **`Home`** — return to default (Level 4) and re-centre on Now.

Axis labels are derived from the **snapped** level; the in-flight
gesture renders cards with a linearly-interpolated `time → x`
mapping but does not switch label vocabulary mid-gesture
(switching from "minute markers" to "hour markers" while the user
is mid-pinch reads as a glitch).

**Default 24 h.** GTA-RP streams are 4–10 h long; a day shows 1–2
streams per streamer at readable card size and matches the
"yesterday's roleplay" entry mental model.

**Out-of-scope:** Level 7+ (90 d, 1 yr) — `stream_intervals`
retention is bounded by the user's tracked window, typically ≤ 30 d.
A "show all history" zoom would be a v2.2 affordance gated by data
density.

### D3 — Now-Indicator Behaviour

**Decision.** A vertical 1-px neutral line spans the lane area at
the current wall-clock instant. Initial render centres the line at
**~75 % viewport-width** when Now falls inside the visible range;
when Now is outside the range, the viewport stays on its existing
pan position and a "Jump to now" CTA surfaces in the chrome.

**Live tick cadence.**

- Levels 1–2 (15 min, 1 h): **10 s** tick.
- Levels 3–6 (6 h+): **30 s** tick.

Rationale: at Level 1 (15 min span on a ~1 200 px lane area), Now
moves ≈ 1.3 px per second; a 30-s tick would advance by ≈ 40 px,
which reads as a "jump" rather than a flow. At Level 4 (24 h span),
30 s advances ≈ 0.4 px — imperceptible, so the slower tick is fine.

**Auto-scroll ("Follow Now") posture.**

- **Off by default.** Initial mount centres once, then leaves the
  user in control.
- An opt-in toggle in the zoom chrome promotes the indicator to
  "follow-mode": the viewport auto-pans to keep Now at the 75 %
  mark.
- **Pan-detection guard.** Any user pan/zoom interaction within the
  last 10 s suppresses auto-scroll even when follow-mode is on.
  Prevents the "user pans away ⇒ snapped back ⇒ rage-quit" loop.
- The toggle persists in the `timeline-view-store` (in-memory; not
  in `app_settings` until a v2.2 ADR decides whether this is a
  user-pref or a per-session state).

**Visual.** 1-px neutral white at ~30 % opacity. Exact pixel
thickness and opacity ramp delegated to **ADR-VisualSystem-v2**.
This ADR only locks in *neutral, not accent* — the Now-indicator
must not consume the accent-restraint budget (H-06/H-07).

### D4 — Lane-Capacity and Vertical Overflow

**Decision.** Hybrid: lanes 1..N_max render normally per side; at
positions where overlap exceeds N_max, a **"+K more" stack-marker**
appears in lane N_max anchored at the over-saturated time-instant.
Clicking the marker opens a **Detail-Overlay variant** ("All streams
in this window") listing the overflowed cards vertically.

**N_max derivation** (viewport-dependent, runtime computed):

```
visibleLaneAreaPx = viewportHeight − topBarPx − heroSlotPx − bottomChromePx
N_max_per_side    = floor((visibleLaneAreaPx / 2) / laneHeightPx)
```

With `viewportHeight ≈ 880`, `topBarPx ≈ 56`, `heroSlotPx ∈ {0, 240}`,
`bottomChromePx ≈ 64`, `laneHeightPx ≈ 60` (latter pinned by
ADR-VodCard / VisualSystem-v2):

| Hero state | Lane area | N_max per side |
| ---------- | --------- | -------------- |
| Hero absent  | 760 px | 6 (rounded down to fit 8 → fits ~ 6 with gutters) |
| Hero present | 520 px | 4 |

(Exact numbers depend on tokens — ADR-VisualSystem-v2 pins them. The
algorithm above is the *contract*: N_max scales linearly with
available lane-area.)

The hero-state choice (`heroSlotPx = 240` vs `0`) is governed by
**D9** — the slot is reserved only when ≥ 1 *favourited* streamer
is live, not whenever any tracked streamer is live.

**Rationale for the hybrid choice.**

- **Vertical scrolling of the entire timeline** — rejected. Breaks
  §3.6's invariant that the time axis is the spine; vertical scroll
  re-introduces the lane-as-permanent-row model the structural
  correction Q-4 removed.
- **Adaptive lane height (cards shrink vertically)** — rejected.
  Breaks H-13 (consistent corner radius / aspect across the app)
  and §3.6 (16:9 thumbnail dominance).
- **Stack-marker only** — partially. At typical 2–4 overlap, flat
  lanes are readable and information-dense; pure stack-marker
  hides information needlessly.
- **Hybrid (chosen)** — preserves both invariants and degrades
  gracefully. Up to N_max overlap the user sees every card; beyond
  it, the stack-marker exposes the overflow as one click away.

**Stack-marker visual.** Neutral surface (no accent — H-06 / H-07),
sized identical to the multi-perspective stack silhouette (D5) so
the user reads "more cards behind this" as a consistent metaphor in
both contexts.

### D5 — Multi-Perspective Moment Grouping

**Decision.** Two intervals belong to the **same multi-perspective
moment** when both conditions hold:

1. Time-overlap ≥ **50 %** of the shorter interval.
2. Both intervals come from streamers in the user's tracked set
   (no cross-server / cross-user detection in v2.1).

**Rationale.** The 50 % threshold is the bar that catches GTA-RP's
signature *late-join* pattern: Streamer A is two hours deep into an
RP scene when Streamer B enters from a different entry-point;
B's coverage of that scene is the same multi-perspective moment told
from a fresher start. Empirically the majority of "same scene, second
perspective" GTA-RP cases overlap in the 50–70 % band of the shorter
session — they share the bulk of the scene from B's join onward and
diverge before / after. A 75 % threshold would drop most of them; a
90 % threshold would drop nearly all. Late-join is not the edge
case — for GTA-RP it is the dominant coverage pattern, which is what
makes 50 % the correct floor rather than the over-generous one. The
symmetric failure mode at the low end (accidentally grouping unrelated
"happened to share half an hour" sessions) is dampened by two
structural guards: only streamers in the user's tracked set are
eligible at all, and the stack-silhouette card always renders the
constituents inspectable on hover / click — a mis-grouping is one
gesture away from being read as two separate sessions.

The grouping is **server-side via `getCoStreams`** (ADR-0015's
existing IPC; `find_co_streams` returns sorted by overlap length
descending). The client-side wrapping translates the result into the
single-stack-card rendering. The threshold becomes a parameter on
`cmd_get_co_streams` only if the existing 0 %-overlap semantics
prove insufficient — for v2.1 the client filters the returned list
to entries that satisfy the 50 % rule.

**Visual consequence.** A grouped moment renders as a single
**stack-silhouette card** at the position of the **longest-running
interval** in the group. The card's image is the longest-interval's
thumbnail; the metadata strip carries streamer-avatar dots for all
participants (Q-5 decision).

**Click semantics.**

- **Click (default)** — opens a Detail Overlay with all
  perspectives listed. Each perspective has its own row + a
  per-perspective "Watch this perspective" CTA. At the bottom: a
  single primary "Open all in Multiview" CTA, enabled when the
  group's perspective count is in `[2, 4]`.
- **Hover (mouse-only, with H-18 0.6–1.0 s delay)** — the
  stacked cards fan out laterally beside the frontmost card. Each
  fanned-out card becomes individually clickable. The fan-out
  trajectory and timing is part of **ADR-VodCard** (Wave 2); this
  ADR specifies the **invocation gesture**, not the visual.
- **Shift+Click on the stack** — bulk-adds **all** perspectives in
  the group to the multi-selection (see D7). Convenience for power
  users who want to multiview the whole moment without going
  through the overlay.

**Multiview-pane-count interaction.** Per **CEO-A1** (2026-05-12;
see decision-log §6), v2.1 expands Multiview from ADR-0021's 2-pane
v1 to a 4-pane capacity. The "Open all in Multiview" CTA on the
stack-card is therefore **enabled when the group's perspective count
is in `[2, 4]`, matching the v2.1 Multiview 4-pane capacity**. The
pane-render mechanics (lane geometry within the Multiview surface,
leader-election among 4 panes, audio-anchor routing) live in a
separate **Multiview-Pane-Expansion-ADR** (parallel v2.1 wave, to be
filed); ADR-0038 takes that 4-pane capacity as a contract and
commits to a **selection cap of 4** plus routing through
`openSyncGroup`, which ADR-0023 already accommodates for the
leader-election shape at N panes.

### D6 — Viewport Virtualization

**Decision.** Pure-DOM rendering with a **time-window-based
render-buffer**. The visible time range plus a half-viewport-width
of padding on each side is materialised in the DOM; everything
outside the buffer is unmounted. CSS `transform: translateX(...)`
positions cards (not `left:`) to keep paint on the GPU compositor.

**Render-buffer.**

```ts
const buffer = 0.5 * (viewportUntil - viewportSince);
const renderedSince = viewportSince - buffer;
const renderedUntil = viewportUntil + buffer;
const visible = laneAssignments.filter(la => {
  const iv = intervalsById.get(la.vodId)!;
  return iv.endAt >= renderedSince && iv.startAt <= renderedUntil;
});
```

This absorbs short pans (≤ ½ viewport-width per move) without
re-render. Long pans cause the buffer to slide and a new slice to
mount — React's reconciliation is fine at this rate.

**Performance targets.**

- First paint: **< 200 ms** at 2 000 intervals (lane-packing +
  render-buffer-filter + initial DOM commit).
- Pan/zoom frame budget: **≤ 16 ms** at 2 000 intervals,
  validated via the Chrome DevTools Performance tab in dev.
- Lane-packing recompute: rate-limited to **≤ 1 / second** under
  ingest-event flood (a malicious or anomalous backfill could
  otherwise re-pack every 50 ms). The rate-limit window preserves
  the last result and applies a single recompute at the end.

**Escape hatch for the Engineering implementation.** If the DOM
approach can't meet 60 fps pan/zoom at 2 000 intervals, swap the
**card-render layer only** to an SVG `<g>`-tree behind the same
lane-packing data structure. The math, the stores, and the IPC
hooks stay identical; only the rendering primitive changes. A
Canvas-based renderer is **rejected** for accessibility cost
(keyboard navigation + screen-reader semantics in canvas need
parallel DOM scaffolding, which costs more than SVG buys).

**Why not `react-window` / `@tanstack/react-virtual`.** Both expect
a 1-D list or a 2-D grid with predictable cell sizing. Sightline's
timeline is 2-axes-of-position (time × lane) with absolute card
positions and variable widths — the abstraction fights us. Custom
item-renderers are possible but the resulting code is harder to
maintain than the time-window buffer above.

### D7 — Multiview Selection Integration

**Decision.** Click-based multi-selection with explicit modifier
gestures. Selection state lives in a new dedicated
`timeline-multiselect-store`. A floating selection-toolbar surfaces
when ≥ 1 card is selected; the "Open in Multiview" CTA is enabled
in the `[2, 4]` range, matching the v2.1 Multiview 4-pane expansion
per **CEO-A1** (decision-log §6). The selection-cap-of-4 is a firm
commitment, not a hedge — the pane-render mechanics live in the
separate Multiview-Pane-Expansion-ADR, and this ADR routes through
`openSyncGroup` (ADR-0023's leader-election shape, already N-pane-
aware) without re-litigating Multiview internals.

**Gesture map.**

| Gesture           | Effect                                                              |
| ----------------- | ------------------------------------------------------------------- |
| **Click**         | Default: open Detail Overlay for the card (single-VOD).             |
| **Shift+Click**   | Add to / remove from selection (toggle).                            |
| **Cmd/Ctrl+Click**| Same as Shift+Click (OS-conventional alias).                        |
| **Esc**           | Clear selection.                                                    |
| **Shift+Click stack** (D5)  | Add **all** perspectives in the stack to selection.       |

**Drag-Box-Selection deferred.** The gesture would conflict with
horizontal pan (already the dominant timeline gesture) and would
need a modifier (e.g. Cmd-drag) which clashes with browser
right-click-emulation on macOS. Tracked as v2.2 affordance behind an
explicit selection-mode toggle.

**Selection visual.**

- **Idle-selected:** H-20-equivalent thin neutral outline around
  the card (the same H-20 used for focus, so a selected-and-focused
  card reads as one state, not two) **plus** a small accent-coloured
  checkmark badge (≈ 16 px square) in the upper-right corner of the
  image area. The badge is the only accent-coloured surface on a
  selected card and satisfies H-07 ("discrete marker").
- **Hover-selected:** H-17 hover-zoom applies normally; the
  checkmark stays. Outline shifts from white-30 to white-50 to
  remain visible against the hover-brightened thumbnail.
- **Focus-selected:** H-20 outline takes priority; checkmark stays.

**Selection toolbar.** A floating bar at the **bottom-centre** of
the viewport, ~80 px above the bottom edge. Renders when
`selectedVodIds.size > 0`. Contents:

```
[N selected]   [Open in Multiview]   [Clear]
```

- `N selected` is plain text.
- `Open in Multiview` is enabled when `2 ≤ N ≤ 4` (per CEO-A1,
  matching the v2.1 Multiview 4-pane capacity). Calls
  `openMultiView({ vodIds: [...selectedVodIds] })` on the `nav-store`,
  which already owns the multi-view page lifecycle (see existing
  `nav-store.ts`).
- `Clear` empties the selection and dismisses the bar.

**Pre-existing-selection-on-navigation.** If the user navigates away
from `TimelinePage` (e.g. to `LibraryPage`) while a selection is
non-empty, the page-unmount **clears** the store. Selections do not
persist across page boundaries in v2.1. (Library has its own
selection paradigm if it ever gets one — Q-10 didn't ask for
Library-multiselect, so no store-sharing is required.)

### D8 — Data Layer and State Ownership

**Component hierarchy (one level of detail).**

```
TimelinePage
├── TimelineHeroSlot                  (conditional on ≥ 1 favourited live; see D9)
├── TimelineZoomChrome                (zoom +/-, "Follow Now", "Jump to now")
├── TimelineAxis                      (date/hour markers + Now indicator)
├── TimelineLanes                     (subscribes to lane-packing memo)
│   ├── TimelineCard                  (one per interval visible in render-buffer)
│   └── TimelineStackMarker           (per overflow position; D4)
└── TimelineSelectionBar              (mount when selection non-empty)
```

**Data flow.**

- **`useTimeline({ since, until, streamerIds })`** — existing
  hook. Filter window is derived from the active zoom level + pan
  offset. TanStack-Query key `["timeline", "list", filters]`,
  staleTime 30 s (unchanged from current implementation).
- **`useTimelineStats()`** — existing. Drives the initial-range
  centring and the "Jump to now" CTA availability.
- **`useCoStreams(vodId)`** — existing. Powers the Detail-Overlay
  multi-perspective list (D5).
- **`useVodSummary(vodId)`** — **new composite hook** combining
  `getVod`, `getVodAssets`, and `getWatchProgress` for a single VOD.
  Each card needs thumbnail URL, title, duration, streamer
  display-name, and watch-progress to render. The composite hook
  keeps each per-card render scoped; TanStack-Query's cache
  deduplicates concurrent fetches across cards. **Backend
  endpoint deferred.** If profiling shows the N+1-query pattern is
  hot, a dedicated `cmd_get_timeline_vod_summary({ vodIds[] })`
  batch endpoint can be added in a follow-up.

**Cache invalidation (event-driven, follows existing precedent).**

| Event                          | Invalidates                                       |
| ------------------------------ | ------------------------------------------------- |
| `vod:ingested`                 | `["timeline", "list", *]`, `["timeline", "stats"]` |
| `vod:updated`                  | `["vod", vodId]`, `["timeline", "list", *]`       |
| `watch:progress_updated`       | `["watch", "progress", vodId]` only — does **not** trigger re-pack |
| `streamer:added` / `removed`   | `["timeline", "list", *]`, `["timeline", "stats"]` |

The progress-stripe invalidation deliberately scopes to a single
VOD's cache key — moving a 3-px stripe does not require lane
re-packing.

**State stores.**

- **`timeline-view-store` (new, Zustand)**:
  ```
  zoomLevel: 1..6
  panOffsetSeconds: number     // signed offset from "Now centred at 75%"
  followNow: boolean
  lastInteractionAt: number    // epoch ms for pan-suppression guard
  ```
- **`timeline-multiselect-store` (new, Zustand)**:
  ```
  selectedVodIds: ReadonlySet<string>
  toggle(vodId)
  clear()
  commit()   // calls useNavStore.openMultiView({ vodIds: [...] })
  ```
- **`nav-store` (existing)** — `openMultiView` invoked from
  `commit()`.

**Lane-packing**: computed per render via
`useMemo(() => packIntervals(timeline.data ?? []), [timeline.data])`.
Pure function, no store required. The render-buffer filter (D6)
runs in a downstream `useMemo` keyed on the buffer window.

**Why not put lane-packing in a Zustand store?** Zustand is for
cross-component state. Lane-packing is per-data-snapshot derivation
— co-located with its consumer is the cleaner pattern. Stores would
require an action-dispatch on every data refresh; the `useMemo` is
free and re-evaluates exactly when its inputs change.

### D9 — Hero-Slot Layout-Reservation Trigger

**Decision.** The hero-slot vertical space (the conditional
`TimelineHeroSlot` component above the lane area) is
**layout-reserved** if and only if the user has at least one
**favourited** streamer currently live. When no favourited streamer
is live, the hero slot is *not* reserved — even when a
tracked-but-not-favourited streamer is live.

**Rationale.** Q-2 in `docs/decision-log/v2.1-anchor-acceptance.md`
set the conditional-hero trigger as "≥ 1 tracked streamer live."
That gives the correct *content* trigger but, taken literally as a
layout-reservation trigger, would cause a one-row layout shift
(≈ 240 px of reserved space appearing or disappearing, with N_max
per side dropping from 6 to 4 in the typical viewport per D4)
whenever any tracked streamer goes live or off — a frequent event
across a Sightline session. Favourited is a deliberate
user-curation signal: a favourited-streamer live transition is the
user's primary "watch now" moment, and reserving the hero slot in
advance pays back in stable framing exactly when stable framing is
most-attended-to. A tracked-but-not-favourited live transition is
by construction the less-attended-to case; absorbing the one-time
layout shift at that moment is the deliberate UX trade-off (CEO-A3,
decision-log §6).

**Trade-off accepted.** Non-favourited tracked-live transitions
cause a single layout shift at the moment the live-state flips.
This is the explicit UX cost paid for layout stability in the
common favourited-live case. If user testing surfaces that this
shift is too disruptive, the trigger can be broadened back to the
full tracked set (Q-2's original literal reading) at the cost of
permanent hero-slot reservation whenever any tracked streamer
*might* go live — a v2.2 tunable, not an ADR re-open.

**Interaction with D4 (N_max).** When the hero slot is reserved,
`heroSlotPx ≈ 240` in the D4 N_max-derivation formula; when it is
not reserved, `heroSlotPx = 0`. The N_max-per-side ladder (6 → 4 on
hero-present in the typical viewport) in D4 therefore depends on
the *favourited-live* state at render time, not on the broader
tracked-live state.

**Reference.** `docs/decision-log/v2.1-anchor-acceptance.md` Q-2
(CEO Decision on Conditional Happening-Now hero);
`docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 CEO-A3.

---

## Consequences

### Positive

1. §3.6 lands as runnable layout without pre-empting Wave-2 / Wave-3
   token / hover decisions.
2. Backend, IPC, and data-model are **untouched** — the migration
   risk is zero on those layers.
3. Lane-packing is O(n · log L) and stable under streaming updates;
   no full re-shuffles on poll completion.
4. Cache-invalidation follows the existing event-driven pattern;
   no new event topic is introduced.
5. Render-buffer + `transform: translateX(...)` keeps pan/zoom on
   the GPU compositor — the 60-fps target is reachable without
   premature SVG/Canvas migration.
6. Selection model accommodates the future Multiview pane-count
   expansion without committing to a count in this ADR.
7. The existing `useTimeline` / `useTimelineStats` / `useCoStreams`
   hooks are reused unchanged; only the rendering layer rewrites.

### Costs accepted

1. **Hero-slot conditional render.** When a non-favourited tracked
   streamer goes live mid-session, the layout shifts by
   `heroSlotPx`. See **D9 — Hero-Slot Layout-Reservation Trigger**
   for the favourited-vs-tracked trade-off and the deliberately
   accepted UX cost (CEO-A3).
2. **Drag-Box-Selection deferred.** Power users who want bulk
   multiview selection via gesture have to Shift+Click cards
   individually or use the Shift+Click-on-stack shortcut.
3. **Greedy Best-Fit can produce > chromatic-minimum lanes** at
   high-overlap moments. The stack-marker mitigates the symptom;
   the algorithm-fallback to Interval Graph Coloring is the
   contingency.
4. **No streamer-stable lane assignment.** The eye follows time,
   not the streamer-lane. This is explicit §3.6 trade-off — the
   time-axis is the dominant axis, the streamer identity moves to
   the card metadata.
5. **30-day-window limit on `stream_intervals`-derived data.** Users
   asking "what happened 6 months ago?" need a v2.2 history-mode
   that does not currently exist. Not a regression — Phase 4
   already had this limit.

### Neutral (forward references to other ADRs)

1. **Exact card geometry** (16:9 aspect resolution in px,
   border-radius, focus-ring pixel-thickness, hover-zoom factor
   numeric value) → **ADR-VodCard (Wave 2)**.
2. **Exact token values** (lane-height-px, axis-line-width-px,
   accent hex for progress stripe and selection checkmark, surface
   luminance ladder, Now-indicator opacity) → **ADR-VisualSystem-v2
   (Wave 3)**.
3. **Hover preview source generation** (GIF-strip frame count,
   capture cadence, generation pipeline) → **ADR-VodCard (Wave 2)**.
   This ADR references the Q-3 GIF-strip decision normatively but
   delegates the *how*.
4. **Keyboard trigger for hover preview.** Q-6 finalised per
   **CEO-A4** (2026-05-12, decision-log §6) to the cascade reading:
   the GIF-strip plays on **Tab-focus** identically to
   **Mouse-hover**, with the same 0.6–1.0 s delay (H-18) and the
   same source frames (Q-3 GIF-strip). No separate Space-bar
   trigger. ADR-VodCard (Wave 2) therefore does not need a
   dedicated Q-6 section — this ADR's forward-reference is the
   final source for the keyboard-trigger behaviour. D6
   (`focus`-state visual; H-20 outline) and D7 (selection-toolbar
   focus order) compose with that reading without change.
5. **Multiview pane count.** ADR-0021 v1 is 2-pane. Per **CEO-A1**
   (2026-05-12, decision-log §6), v2.1 lifts the pane count to 4
   via a separate **Multiview-Pane-Expansion-ADR** (parallel v2.1
   wave, to be filed). This ADR's D5 / D7 selection-cap-of-4 and
   `openSyncGroup` routing are the contract; the pane-render
   mechanics (lane geometry, leader-election among 4, audio-anchor
   routing) live in that separate ADR.

### Risks

1. **Backfill-flood lane re-pack thrash.** A future bulk-ingest
   (e.g. a user adding a new streamer with months of retention)
   could fire many `vod:ingested` events in a burst, each triggering
   a `useTimeline` cache invalidation and a lane-pack recompute.
   **Mitigation:** rate-limit lane-packing recomputes to ≤ 1 / second
   under event flood; preserve the last result in-flight. The
   rate-limit is implemented in the `useTimeline` consumer (via a
   debounced selector), not in the IPC layer.
2. **Now-indicator tick visibility.** At Level 1 (15 min span), Now
   advances ≈ 1.3 px / s. A 10-s tick moves the line ≈ 13 px every
   tick — visible but acceptable. If user-testing surfaces "the
   line jumps," drop to 5 s at Level 1 only (still bounded under
   the rAF budget).
3. **Stack-marker click → Detail Overlay variant** is a new sub-
   pattern not currently described in §3.4 of the anchor.
   **ADR-VisualSystem-v2** will need to confirm the Detail-Overlay
   variant is anchor-faithful; if not, the stack-marker either
   opens a custom popover or routes to a dedicated "overflow"
   surface. This ADR locks in the *click → open list of overflow
   cards* contract; the visual envelope is Wave 3's call.
4. **Performance assumption at 2 000 intervals** is from synthetic
   load. The real-user 99-percentile may differ. If first-paint
   exceeds 200 ms at the real distribution, **D6's SVG escape
   hatch** lands without affecting the rest of the ADR.

---

## Alternatives considered

Per-decision summary. The **chosen** option's rationale is in the
Decisions section above; here each rejected alternative gets one
explicit sentence.

### D1 (Lane-Packing)

- **Greedy First-Fit alternating** — rejected. Side-occupancy
  imbalance reads as "everything on top" / "everything on bottom"
  at moderate overlap.
- **Interval Graph Coloring (chromatic-optimal)** — rejected for
  steady-state. Re-coloring on streaming-update shifts existing
  assignments — breaks cognitive continuity. Kept as **Engineering
  fallback** for deep-stack pathology.
- **Streamer-Stable Lane Assignment** — rejected. Reverts to the
  Phase-4 per-streamer-row implementation that the structural
  correction Q-4 removed.

### D2 (Zoom)

- **Continuous-only zoom** — rejected. Axis-label transitions
  mid-gesture read as a glitch.
- **Discrete-only zoom** — rejected. Loses the trackpad-pinch
  gesture which is the most natural input.
- **Default zoom 7 d (Week)** — rejected. Day matches the
  "yesterday's roleplay" mental model and gives readable card sizes.

### D3 (Now-indicator)

- **1-s live tick** — rejected. Over-rendering for a UI element
  that moves at sub-pixel rates outside Level 1.
- **1-min live tick** — rejected. Visibly lags at Level 1.
- **Auto-scroll on by default** — rejected. Disrupts user-driven
  pan.
- **Now-indicator visualised as a hero-style marker** — rejected.
  Now is a temporal marker, not a content marker; the line is
  the correct primitive.

### D4 (Overflow)

- **Full vertical scrolling** — rejected. Breaks §3.6's time-axis-
  as-spine invariant.
- **Adaptive lane height (cards shrink)** — rejected. Breaks H-13
  consistency and §3.6 16:9 thumbnail dominance.
- **Stack-marker only (no flat lanes)** — rejected. Hides
  information at typical 2–4 overlap.

### D5 (Multi-Perspective)

- **75 % overlap threshold** — rejected (CEO-A2). Loses the majority
  of late-join scenarios, which are core GTA-RP coverage patterns
  (Streamer A two hours into a scene, Streamer B joins) — the
  signature *same-scene-second-perspective* shape, not the edge case.
- **90 % overlap threshold** — rejected. Under-groups: drops
  legitimate join / leave mid-session cases entirely. (The symmetric
  failure mode at the low end — accidentally over-grouping
  "happened to share half an hour" sessions — is what bounds the
  chosen threshold from below; 50 % sits above that floor and is
  guarded structurally by the tracked-set constraint and the
  inspectable stack-silhouette card.)
- **Tag-Match / Game-Match additional signal** — deferred. The
  required tag-data is not exposed at the IPC layer in v2.1;
  candidate for v2.2.
- **Single non-stack card with avatar-strip only** — rejected. The
  stack silhouette reads as "more behind," which is the metaphor
  §3.6 specifies.

### D6 (Virtualization)

- **`react-window` / `@tanstack/react-virtual`** — rejected. The
  abstraction fights free-form absolute-positioned cards across
  two axes.
- **SVG from the start** — deferred as an escape hatch. Same lane-
  packing data structure; only the rendering primitive changes.
- **Canvas** — rejected. Accessibility cost (keyboard navigation +
  screen-reader semantics) outweighs the perf headroom.

### D7 (Selection)

- **Click-adds-to-selection (no modifier)** — rejected. Conflicts
  with the Library pattern (click = open detail) and removes the
  default single-card-action.
- **Drag-Box-Selection** — deferred. Gesture conflict with
  horizontal pan; would need an explicit selection-mode toggle.
- **Accent-coloured selection outline (instead of badge)** —
  rejected. Conflicts with H-07 / H-20.
- **Floating toolbar at the top** — rejected. Top is the hero-slot
  area when present.

### D8 (Data / State)

- **Lane-packing as a Zustand-store action** — rejected. Zustand
  is for cross-component state; lane-pack is per-render derived.
- **New `cmd_get_timeline_vod_summary` batch endpoint** — deferred.
  Composite-hook in the renderer first; backend batch only if
  profiling shows hot N+1.

---

## Implementation Notes

### Files to be created (v2.1 implementation — out of scope for this ADR)

- `src/features/timeline/TimelinePage.tsx` — rewritten
- `src/features/timeline/TimelineHeroSlot.tsx` — new (conditional)
- `src/features/timeline/TimelineZoomChrome.tsx` — new
- `src/features/timeline/TimelineAxis.tsx` — new
- `src/features/timeline/TimelineLanes.tsx` — new
- `src/features/timeline/TimelineCard.tsx` — new (scaffolding;
  hover behaviour lands with ADR-VodCard)
- `src/features/timeline/TimelineStackMarker.tsx` — new
- `src/features/timeline/TimelineSelectionBar.tsx` — new
- `src/features/timeline/lane-packing.ts` — new pure-function
  module (Greedy Best-Fit + Interval Graph Coloring as a
  feature-flagged fallback)
- `src/features/timeline/use-vod-summary.ts` — new composite hook
- `src/features/timeline/use-timeline.ts` — extended (existing
  hooks retained, new derived hooks added)
- `src/stores/timeline-view-store.ts` — new
- `src/stores/timeline-multiselect-store.ts` — new

### Files removed / replaced

- The inline `TimelineLane` sub-component in
  `src/features/timeline/TimelinePage.tsx` is removed (its responsibilities
  move to `TimelineLanes` + `TimelineCard`).

### Files unchanged

- All `src-tauri/**` code. The data model and IPC surface are
  sufficient.
- `src/ipc/bindings.ts`. No new IPC commands in v2.1 Timeline-Layout.
- `src/stores/nav-store.ts`. `openMultiView` is reused as-is.
- `src/features/multiview/**`. Multiview entry contract via
  `openSyncGroup` is unchanged; the v2.1 Multiview-ADR (separate
  Wave 2 / Wave 3 work) governs the pane-count expansion.

### Test strategy

- **Lane-packing math** — `src/features/timeline/lane-packing.test.ts`,
  Vitest, pure-function tests:
  - empty input
  - single interval
  - no overlap
  - full overlap (k ≥ 2)
  - mixed overlap with gaps
  - overlap exceeding N_max → assert stack-marker positions
  - property-based (fast-check): result lane-count ≤ Greedy
    First-Fit reference; result above/below balance differs by
    ≤ 1.
- **Component snapshots** — TimelineLanes with mock interval sets
  at each zoom level (6 snapshots).
- **Selection state** — `timeline-multiselect-store` unit tests:
  toggle, clear, commit-routing-to-nav-store.
- **Hover state for card-as-selected** — snapshotted independently
  of the (deferred) full-VodCard hover behaviour.
- **E2E (pan / zoom / select)** — Playwright, gated on the
  parallel E2E mission landing (v2.1 backlog from v2.0.3). Skipped
  in v2.1 Timeline-Layout if E2E mission is later in the ordering.

### Migration path

v2.1 release ships the new `TimelinePage` end-to-end. No backend
migration; no DB migration; no `stream_intervals` rebuild. Users
upgrading from v2.0.3 to v2.1 see the redesigned timeline on first
launch with no data loss and no re-ingest. The lane-packing math
is deterministic from the same `(startAt, endAt, streamerId,
vodId)` rows the v1 implementation already reads.

---

## Open Questions for CEO

**None.** All decisions in this ADR are constrained by §3.6 +
decision-log Q-1 / Q-2 / Q-4 / Q-5 / Q-10, the algorithmic / zoom /
virtualization choices are engineering judgements with empirical
fallback paths, and forward-references to ADR-VodCard (Wave 2) and
ADR-VisualSystem-v2 (Wave 3) are explicit and bounded. ADR-0038 is
ready for ACCEPTED on CEO sign-off without additional discussion.

---

## References

- **Spec sources (binding).**
  - `docs/reference/visual-language-netflix.md` §3.6 (Single-Time-Axis)
  - `docs/reference/visual-language-netflix.md` §6.1 (TimelinePage mapping)
  - `docs/reference/visual-language-netflix.md` §7 (CEO Decisions Q-1, Q-2, Q-4, Q-5, Q-10)
  - `docs/decision-log/v2.1-anchor-acceptance.md`
- **Related ADRs.**
  - [ADR-0015](0015-timeline-data-model.md) — `stream_intervals`
  - [ADR-0018](0018-watch-progress-model.md) — watch-progress
  - [ADR-0021](0021-split-view-layout.md) — Multiview v1 layout
  - [ADR-0022](0022-sync-math-and-drift.md) — sync math (Now-indicator-tick precedent)
  - [ADR-0023](0023-group-wide-transport.md) — group transport
  - **ADR-Multiview-Pane-Expansion** (to be filed as a separate v2.1
    wave per CEO-A1) — pane-render logic for the 4-pane capacity
    that ADR-0038 D5 / D7 take as a contract.
- **Source files referenced.**
  - `src/features/timeline/TimelinePage.tsx` (current impl, to be rewritten)
  - `src/features/timeline/use-timeline.ts` (TanStack hooks, retained)
  - `src/stores/nav-store.ts` (`openMultiView`, reused)
  - `src/ipc/bindings.ts` (`Interval`, `TimelineFilters`, `cmd_list_timeline`, `cmd_get_co_streams`, `cmd_get_timeline_stats`, `openSyncGroup`)
