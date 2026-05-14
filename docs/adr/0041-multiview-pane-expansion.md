# ADR-0041 — Multiview Pane Expansion: 2×2 Quadrant Layout, Pane-Count Variants, Audio Anchor Semantics

- **Status.** Accepted 2026-05-12
- **Date.** 2026-05-12 (drafted PROPOSED) → 2026-05-12 (accepted by CEO sign-off)
- **Wave.** v2.1 ADR parallel wave — CEO-A1 follow-up from
  [ADR-0038](0038-timeline-layout-single-time-axis.md) (ACCEPTED 2026-05-12).
  Runs in parallel to the Wave-1 / Wave-2 / Wave-3 sequence
  (Timeline-Layout → VodCard → VisualSystem-v2). With this ADR's
  sign-off the v2.1 ADR phase is structurally complete and Wave-4
  engineering can begin.

## Acceptance

ACCEPTED 2026-05-12 by CEO as a block.

- M-A1 — 3-Pane Layout: (d) 2×2 grid with bottom-right slot empty.
- M-A2 — Pane-Close Affordance: (α) per-pane close button.
- M-A3 — Audio-Switch Delay: 200 ms.
- M-A4 — Audio-Switch Strategy: Hard-switch (`--motion-audio-switch-duration: 0ms`).
- M-A5 — Audio-Return on Hover-Exit: Spring-back to leader (M4d follows by symmetry).

All five engineering recommendations adopted without substance-patches.
Compound-Risk #4 (M-A3 × M-A4 pairing) explicitly acknowledged at sign-off — adopted as the paired "gentle, responsive" UX character.
See `docs/decision-log/v2.1-adr-0041-multiview-pane-expansion.md` §7 for the sign-off audit-trail.
- **Related.**
  [ADR-0021](0021-split-view-layout.md) (Multiview v1 foundation —
  layout, store shape, entry contract; this ADR extends from 2-pane
  to 1–4-pane variable capacity) ·
  [ADR-0022](0022-sync-math-and-drift.md) (sync math binding — 250 ms
  threshold + tick, seek-only correction, out-of-range model — applied
  unchanged at N > 2; per-tick cost scales linearly with pane count) ·
  [ADR-0023](0023-group-wide-transport.md) (transport semantics binding
  — group-wide Play/Pause/Seek/Speed, first-opened-default leader,
  `cmd_set_sync_leader` for promotion; audio-anchor is an additive
  layer on top, not a leader-election override) ·
  [ADR-0038](0038-timeline-layout-single-time-axis.md) (CEO-A1 source;
  D7 Selection-Cap = 4 binding; CTA enable rule 2 ≤ N ≤ 4) ·
  [ADR-0040](0040-visual-system-v2.md) (token consumption — TV1
  surface ladder + `--color-outline-focus`, TV3 spacing,
  TV5 `--focus-outline-*`, TV6 motion constants, TV7 z-index stack,
  TV11 Lucide icon system) ·
  [ADR-0019](0019-asset-protocol-scope.md) (asset protocol unchanged;
  `getVideoSource` IPC reuse).
- **Spec sources (binding).**
  `docs/reference/visual-language-netflix.md` §6.3 (MultiviewPage
  mapping), §7 Q-8 (Multiview leader-quadrant prominence) — Anker
  binding 2026-05-12. Heuristics H-01, H-05, H-08, H-19, H-20, H-22,
  H-23 apply per §6.3. H-04 (hero) and H-25 (rail) explicitly drop.
  Decision-logs: `docs/decision-log/v2.1-anchor-acceptance.md` §1 Q-8
  + §2; `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6
  CEO-A1.

## Context

CEO-A1 (ADR-0038 sign-off, 2026-05-12) commits v2.1 Multiview to
**4-pane capacity** and delegates pane-render mechanics to this
separate ADR. ADR-0038 D7 binds the Timeline selection-cap to 4 and
the "Open in Multiview" CTA to `2 ≤ N ≤ 4`, taking this ADR's
4-pane capacity as a contract. Anker §6.3 + Q-8 bind the
**equal-size quadrant layout** as the default posture and add an
**audio anchor** mechanic (audio default at the leader, switches on
pointer-hover to the hovered quadrant). Focus Mode (asymmetric
60 %/13 % layout) is preserved as a **future opt-in option**, not a
v2.1 default.

The existing `src/features/multiview/` surface (ADR-0021 v1, shipped
Phase 6) is the foundation this ADR extends. Inventory:

| File                              | Role                                                                | Status under v2.1                                      |
| --------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------ |
| `MultiViewPage.tsx`               | Page-level shell, mounts the panes, owns the page-level seek slider | **Extended.** Layout becomes pane-count-variable.      |
| `MultiViewPane.tsx`               | One pane: `<video>` element + chrome (leader badge, promote, volume, mute, out-of-range overlay) | **Extended.** Adds audio-anchor indicator, hover-handling, neutral leader outline per ADR-0040 TV1. |
| `MultiViewTransportBar.tsx`       | Group-wide Play/Pause + Seek slider + Speed selector + Close        | **Unchanged structurally.** Buttons re-skinned to TV11 Lucide icons (Wave 4).                       |
| `sync-store.ts`                   | Zustand: `sessionId`, `panes[]` (currently length 0 or 2), `leaderPaneIndex`, `overlap`, transport state | **Extended.** `panes[]` length grows from `0 | 2` to `0 | 2 | 3 | 4`. Audio-anchor state added.       |
| `sync-math.ts` + `.test.ts` + `.fixture.json` | Per-pane expected-time / drift math (ADR-0022)             | **Unchanged.** Math scales linearly per pane.          |
| `use-sync-loop.ts` + `.test.tsx`  | 250 ms tick loop, drift correction, out-of-range detection          | **Unchanged.** Loop iterates over `panes[]` regardless of length. |

Existing IPC bindings that stay valid:

- `commands.getVideoSource({ vodId })` — per-pane asset URL (ADR-0019 binding; unchanged).
- `commands.openSyncGroup({ vodIds, layout })` — group-open. `vodIds` is already an array; this ADR commits to an **additive widening** from 2 to 2–4 elements and a new `layout` discriminant value (see M6).
- `commands.closeSyncGroup`, `commands.setSyncLeader`, `commands.syncPlay/syncPause/syncSeek/syncSetSpeed`, `commands.getOverlap`, `commands.getVod` — unchanged.

This ADR pins the **structural** and **interaction** decisions that
take the 4-pane capacity from CEO-A1 commitment into runnable code,
without re-litigating ADR-0022 sync math, ADR-0023 transport
semantics, or ADR-0040 tokens. Where a decision in this ADR consumes
a Wave-3 token, the consumption is bounded to existing ADR-0040
values; new Multiview-specific tokens are documented in M12 as
additions, not as ADR-0040 re-opens.

---

## Decisions

### M1 — Pane-Count Layout Geometry

**Decision.** The Multiview viewport renders one of three layout
variants by pane count, all using CSS Grid:

| Panes (N) | Layout                                          | Grid template                                          | Inter-pane gap                       |
| --------- | ----------------------------------------------- | ------------------------------------------------------ | ------------------------------------ |
| 2         | Horizontal 50/50 (binding, ADR-0021)            | `grid-template-columns: 1fr 1fr; grid-template-rows: 1fr;` | `--multiview-pane-gap` (4 px, M12) |
| 3         | 2×2 grid with bottom-right slot empty (per M-A1)         | `grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr;` | `--multiview-pane-gap` (4 px) |
| 4         | 2×2 equal-size quadrants (binding, Anker Q-8)   | `grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr;` | `--multiview-pane-gap` (4 px) |

**N = 1 is not a Multiview state.** The selection-cap-of-4 with CTA
enable `2 ≤ N ≤ 4` (ADR-0038 D7) makes 1-pane unreachable through
the Timeline entry surface. If `openMultiView` is called with
`vodIds.length === 1` (e.g., a v2.x deep-link), the page collapses
back to single-player via `nav-store`. This ADR does not define a
1-pane Multiview layout.

**Inter-pane gap is `--space-1` (4 px) from ADR-0040 TV3.** The 4 px
end of the anchor §2.3 "tight gutters" range (4–8 px) is preferred
over 8 px because Multiview is *not* a categorical rail — it is a
synchronised group, where the panes read as one coherent surface.
Tighter gutters reinforce group-identity; looser gutters would
visually segment the four perspectives. This is engineering judgment
within the anchor-binding range.

**3-pane layout — M-A1 adopted option (d).** The four candidate
anchorings considered during the PROPOSED phase are preserved below
as the audit-trail:

```
(a) 2-top + 1-bottom-centred         (b) 1-top-wide + 2-bottom-half      (c) 3-column row
┌─────────┬─────────┐                 ┌───────────────────┐               ┌─────┬─────┬─────┐
│ Pane 0  │ Pane 1  │                 │      Pane 0       │               │  0  │  1  │  2  │
│         │         │                 │                   │               │     │     │     │
├────┬────┴────┬────┤                 ├─────────┬─────────┤               │     │     │     │
│    │ Pane 2  │    │                 │ Pane 1  │ Pane 2  │               │     │     │     │
│    │         │    │                 │         │         │               │     │     │     │
└────┴─────────┴────┘                 └─────────┴─────────┘               └─────┴─────┴─────┘
```

```
(d) 2×2 grid with bottom-right slot empty (engineering recommendation)
┌─────────┬─────────┐
│ Pane 0  │ Pane 1  │
│         │         │
├─────────┼─────────┤
│ Pane 2  │ (empty) │
│         │         │
└─────────┴─────────┘
```

**M-A1 adopts (d).** Reuses the 4-pane 2×2 Grid template (one CSS
template, not three), keeps each pane at the same aspect across
3-pane and 4-pane variants (no per-pane size shift when the user
adds a 4th pane via Timeline-selection + re-open), and gives the
user a visual cue that a 4th slot is available. Option (a) produces
a centred-orphan pane that doesn't sit at a Grid cell boundary; (b)
introduces an asymmetric "leader-ish" weight that conflicts with
Q-8 equal-size binding; (c) breaks the 2×2 mental model and would
require a separate 3-column template. Options (a), (b), (c) remain
in Alternatives Considered as the audit-trail.

### M2 — Reshuffle on Pane-Add (2→3, 3→4)

**Decision.** Pane-add is **deterministic by slot index**, using a
fixed left-to-right, top-to-bottom slot order under the 2×2 Grid
template. The new pane occupies the **first empty slot** of the
2×2 grid. Animation: a 200 ms opacity crossfade
(`--motion-zoom-duration` from ADR-0040 TV6) on the new pane's
mount; existing panes' positions do not shift because each pane is
anchored to a fixed grid cell.

```
Slot order under the 2×2 template:
  slot 0 (top-left)  → first opened pane (default leader, ADR-0023)
  slot 1 (top-right) → second opened pane
  slot 2 (bottom-left) → third opened pane
  slot 3 (bottom-right) → fourth opened pane
```

**Audio-anchor on pane-add: unchanged.** The new pane is never
auto-promoted to leader (ADR-0023 first-opened binding) and never
auto-grabs audio (audio stays at the leader per Q-8). The user can
hover the new pane to switch audio per M4 or click "Promote to
leader" per ADR-0023.

**Reduced-motion fallback (H-22 binding).** When
`prefers-reduced-motion: reduce` is active, the crossfade collapses
to `--motion-fade-instant` (100 ms opacity-only fade per ADR-0040
TV6 reduced-motion override). No slide, no scale.

**Why not "reshuffle the existing panes to balance the layout."**
At 2→3, an inserted pane in slot 2 (bottom-left) is the natural
extension of the 2-pane horizontal layout. Reshuffling pane 0 and
pane 1 to centre the trio would re-flow the user's leader pane
mid-session — high cognitive cost for zero geometric gain.

### M3 — Reshuffle on Pane-Remove (4→3, 3→2, 2→0)

**Decision.** Pane-remove is **slot-preserving with re-pack at the
N → N−1 boundary**:

- **4→3:** the removed pane's slot is left empty; the remaining 3
  panes stay at their slots (slot 0/1/2/3, minus the closed one).
- **3→2:** the layout transitions from the 2×2-with-empty template
  to the horizontal 50/50 template (M1 N=2). Re-pack: the remaining
  two panes move to slot 0 (top-left → left half) and slot 1
  (top-right → right half).
- **2→1:** is **not a layout state** (M1). The session closes
  automatically; `nav-store` navigates back to the previous page
  (matching the existing "Close group" semantics from ADR-0023).
  The transport-bar Close button continues to map to "close the
  whole session."

**Animation.** The N → N−1 re-pack uses `--motion-zoom-duration`
(200 ms) opacity-fade-out on the closing pane, then layout-shift
on the remaining panes via the existing CSS Grid template
transition. Reduced-motion: instant re-pack.

**Leader re-election when the leader pane is removed.** ADR-0023
binding is `If the leader is the closing pane and another pane
remains, the remaining pane becomes leader`. At N > 2, the
remaining-pane is ambiguous; this ADR pins:

```
new_leader_index = min{ paneIndex | pane not closed }
```

The lowest remaining pane-index becomes leader. This is stable
(deterministic), predictable (the user can tell which pane will
inherit by the slot order), and matches ADR-0023's "first opened"
default spirit. Alternatives (geographically-nearest, explicit
user-pick popover) considered and rejected — see Alternatives.

**Audio-anchor re-routing when the audio-anchor pane is removed.**
If the closing pane is the current audio-anchor (i.e., a non-leader
pane that received audio via hover-switch per M4), audio re-routes
to the leader immediately (no transition). If the closing pane *is*
the leader and the leader was also the anchor (the common case at
default), audio re-routes to the new leader per the rule above.

**Pane-close affordance.** Per M-A2, per-pane close is shipped (see
**M7** for the affordance details). The re-pack and leader
re-election rules above apply uniformly regardless of which pane is
closed (the per-pane button, a keyboard shortcut, or the group-only
Close from the transport bar all converge on the same close path).

### M4 — Audio-Anchor Mechanic (Q-8 Extension)

**Decision.** Audio defaults to the **leader pane** (ADR-0023
binding). When the user hovers a **non-leader pane**, audio
switches to that pane (Anker Q-8 binding). This ADR pins the
sub-behaviour of the switch:

| Sub-decision                              | Engineering default                                                 | Status                       |
| ----------------------------------------- | ------------------------------------------------------------------- | ---------------------------- |
| **M4a** — Switch-delay on hover-start     | **200 ms (M-A3)**                                                   | Pinned                       |
| **M4b** — Switch-strategy (hard / crossfade) | **Hard-switch (M-A4)**                                           | Pinned                       |
| **M4c** — Return-behaviour on hover-exit | **Spring-back to leader (M-A5)**                                    | Pinned                       |
| **M4d** — Return-behaviour on viewport-exit | Same logic as M4c (consistency); **(M-A5 spring-back applies)**  | Engineering-default          |
| **M4e** — Audio-anchor visualisation     | Lucide `Volume2` glyph (TV11) in pane-chrome top-right corner, shown only on the currently-audible pane (whether leader-default or hover-switched) | Engineering-default |

**State diagram (audio-anchor behaviour, with M-A3 / M-A4 / M-A5
pinned).**

```
                           ┌──────────────────────────────────────────────┐
                           │                                              │
                           ▼                                              │
              ┌─────────────────────────┐                                 │
              │ ANCHOR = leader-default │ ◀── (initial state on session open)
              │ Volume2 glyph on leader │                                 │
              └────────────┬────────────┘                                 │
                           │                                              │
              pointer enters non-leader-pane                              │
              waits 200 ms (M-A3)                                          │
                           │                                              │
                           ▼                                              │
              ┌─────────────────────────┐                                 │
              │ ANCHOR = hovered-pane   │ ── pointer exits hovered-pane ──┤
              │ (hard-switch per M-A4)  │    M-A5: spring-back to leader   │
              │ Volume2 glyph migrates  │                                  │
              └────────────┬────────────┘                                 │
                           │                                              │
              pointer enters different non-leader-pane                    │
              waits 200 ms (M-A3)                                          │
                           │                                              │
                           ▼                                              │
              ┌─────────────────────────┐                                 │
              │ ANCHOR = newly-hovered  │ ────────────────────────────────┘
              │ Volume2 glyph migrates  │
              └─────────────────────────┘
              
              ┌─────────────────────────────────────────────────────────┐
              │ Side-effects, every state:                              │
              │   - pane.volume × pane.muted respected per-pane         │
              │     (M9 binding: per-pane mute can silence anchor)      │
              │   - leader-pane outline (M5) unchanged by audio switch  │
              │   - reduced-motion (H-22): hard switch (same as M-A4    │
              │     default), no audio crossfade ever                   │
              └─────────────────────────────────────────────────────────┘
```

**Why M-A3 (delay) ≠ Card-Hover-Delay (800 ms, ADR-0040 TV6).** The
800 ms `--motion-hover-delay` was picked for Card-Preview to debounce
mouse-pass-through over thumbnails on browse surfaces. In Multiview,
the panes are the *only* content; the user's pointer is on the panes
by default, and an 800 ms wait before audio responds to a deliberate
quadrant-focus gesture would feel sluggish. A purpose-distinct
`--motion-audio-switch-delay` token (M12) sits in the 0–250 ms band.
**M-A3 = 200 ms (engineering recommendation adopted)**, midway
between "instant" (0 ms, susceptible to pointer-drift) and
"deliberate" (300 ms+, perceptibly slow).

**Why M-A4 (strategy) is non-trivial.** Hard-switch (instant mute +
unmute) is implementation-cheap and immediately legible; crossfade
(e.g., 150 ms volume-ramp on both panes) is professional-cinematic
but adds a new motion token, a Web-Audio-Graph implementation cost,
and a click-event-during-transition edge case. **M-A4 = Hard-switch
(engineering recommendation adopted)** for v2.1 (simpler,
predictable); crossfade remains as a v2.x follow-up if real-use
feedback flags abrupt audio cuts.

**Why M-A5 (return-behaviour) is genuinely UX-prägend.** Two
defensible UX shapes existed at the PROPOSED phase:

- **Spring-back to leader on hover-exit.** Audio is always tied to a
  deliberate hover; absence of hover defaults to the leader. Reads
  as: "I'm choosing this pane while my pointer is there; otherwise,
  the leader has the floor."
- **Sticky-stay on last-hovered.** Audio stays where the user last
  put their attention until they redirect it. Reads as: "I picked
  this perspective; it keeps the floor until I pick another."

Both have shipped in adjacent products. **M-A5 = Spring-back
(engineering recommendation adopted)** because it preserves the
leader's role as the "default voice" and matches the
audio-default-at-leader semantic from Q-8 ("audio defaults to the
leader").

### M5 — Leader-Outline Visual (H-20 binding)

**Decision.** The leader quadrant carries an H-20-conformant outline
visual, using ADR-0040 TV1 + TV5 tokens unchanged:

| Property                | Value                                  | Source                          |
| ----------------------- | -------------------------------------- | ------------------------------- |
| Outline width           | `2px`                                  | `--focus-outline-width` (TV5)   |
| Outline color           | `rgba(255, 255, 255, 0.85)`            | `--color-outline-focus` (TV1)   |
| Outline offset          | `2px` inside the pane edge             | `--focus-outline-offset` (TV5)  |
| Position                | Inside the pane border, drawn via CSS `outline` with negative offset (so the outline does not push the pane geometry) | engineering default |
| Z-index                 | `--z-card-hover` (20)                  | TV7 (no new layer needed)       |

**Leader badge glyph.** A `Crown` icon from Lucide-react (TV11,
ADR-0040), monochrome outline at `--icon-size-base` (16 px), placed
in the pane-chrome top-left corner. Replaces the existing
`★ Leader`-in-amber-pill from `MultiViewPane.tsx` (which violates
H-20 / H-07 — accent fill on a non-CTA chrome element).

**Audio-anchor visualisation (M4e) and leader-outline interaction.**
When the audio-anchor and the leader are the same pane (default
state), only the Crown icon and the H-20 outline are visible; the
`Volume2` glyph is rendered next to the Crown to confirm "this pane
has audio." When the audio-anchor is a non-leader pane (hover-switched
state), the H-20 outline stays on the leader (unchanged), and the
`Volume2` glyph appears on the hovered pane (no second outline — the
visual cue is the migrating glyph, not a second focus-ring layer).
This keeps a single focus-ring discipline per anchor §1.4 / H-20.

### M6 — Pane-Selection-from-Timeline Integration

**Decision.** The Timeline `TimelineSelectionBar` (ADR-0038 D7) calls
**`openSyncGroup({ vodIds, layout })`** with `vodIds.length ∈ [2, 4]`.
The IPC schema is **additively extended**:

- `vodIds: string[]` — already array-shaped; v1.0 had `length === 2`
  invariant, v2.1 widens to `length ∈ [2, 4]`. No type change.
- `layout: "split-50-50" | "quadrant-2x2"` — discriminant for the
  backend session row. v1.0 sent `"split-50-50"` only;
  v2.1 sends `"split-50-50"` when N = 2 (back-compat) and
  `"quadrant-2x2"` when N ∈ {3, 4} (single template covers both N
  cases per M1).

**Backend treatment.** The backend `cmd_open_sync_group` already
validates the `vodIds` against existing-and-downloaded preconditions
(Phase 6); the v2.1 widening from 2 to 2–4 elements requires the
validator to update its length-check from `== 2` to `2 ≤ length ≤ 4`
and to accept the new `layout` discriminant. Schema stays the same
shape; no migration. The `sync_sessions.layout` column already
exists per ADR-0021 migration 0010 and stores a string.

**Pane-ordering ↔ Selection-ordering.** Selection-order (the order
in which the user Shift+Click'd cards in `TimelineSelectionBar`) maps
to pane-index ascending. First-selected → Pane 0 → default leader
(ADR-0023 first-opened binding preserved). Pane 1, 2, 3 follow in
selection order. This makes the leader-choice deterministic from
the user's selection-gesture, with no "guess which one was first"
ambiguity.

**No new IPC endpoint.** Engineering choice: extend `openSyncGroup`
additively. A separate `openSyncGroup4` or `openMultiPaneGroup` would
fork the IPC surface for no semantic gain — the leader-election shape
from ADR-0023 is already N-pane-aware (per Follow-up #4: "> 2 panes
(v2). The leader-election model already supports N panes; we'd extend
the UI affordances per pane").

### M7 — Pane-Close-Affordance

**Decision.** Per **M-A2**, option **(α) per-pane close** ships in
v2.1. The two options considered during the PROPOSED phase are
preserved below as the audit-trail:

| Option              | Affordance                                                           | UX                                                                 | Implementation                          |
| ------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------- |
| **(α) Per-pane close** | Small `X` icon (Lucide, `--icon-size-sm` 14 px) in pane-chrome top-right, hover-revealed (opacity 0 → 1 on pane-hover) | User can prune individual stale panes (e.g., the one that drifted into "Out of range" steady-state) without restarting the session | Adds a pane-local close handler + leader re-election trigger (M3) |
| **(β) Group-close only (status quo from ADR-0021)** | Transport-bar `Close` button only; closes the whole session         | Simpler chrome; matches the v1 mental model "multi-view is one unit"; user must restart session to drop one pane | No new affordance; M3 leader re-election still defined for the future-per-pane case (ADR-0023 Follow-up #1) |

**M-A2 adopts (α) per-pane close.** Backend `cmd_close_pane({ sessionId,
paneIndex })` added as additive IPC endpoint in Wave 4. v2.1 allows
up to 4 panes — at 4 the "always restart to drop one" cost is
higher than at 2, and the per-pane affordance is a small chrome
addition with clean semantics. ADR-0023 already documented the
leader-re-election contract for the per-pane-close case as
Follow-up #1, indicating this was always-anticipated.

### M8 — Transport-Bar Extension for N Panes

**Decision.** The existing `MultiViewTransportBar` (Play/Pause +
Seek + Speed + Close) is **structurally unchanged**. v2.1 changes
are visual-only (TV11 Lucide glyph re-skin in Wave 4) and the
header text update from `Leader: pane N` to support N ∈ {1..4}.

- **Per-pane volume + mute stays in pane-chrome** (ADR-0021
  binding). No consolidation into the transport bar.
- **No global "audio-anchor indicator" in the transport bar.** The
  Volume2 glyph on the currently-audible pane (M4e) is the
  single-source visual; duplicating it in the transport bar would
  create a competing signal.
- **No "promote to leader" affordance in the transport bar.** It
  stays in pane-chrome on non-leader panes per ADR-0023 (one
  button per non-leader pane).

The transport-bar `accent-[--color-accent]` references on focus
states are migrated to `--color-outline-focus` in Wave 4 per
ADR-0040 TV1. This is a Wave-4 token-migration item, not an M8
decision.

### M9 — Mute-Toggle Semantic under Audio-Anchor

**Decision.** Per-pane mute (ADR-0021 binding) remains a **local
pane property**. The audio-routing layer (Wave 4 implementation)
computes the effectively-audible signal as:

```
effective_audio = anchor_pane.volume × (anchor_pane.muted ? 0 : 1)
```

Where `anchor_pane` is the leader (default) or the hover-switched
pane (M4). Per-pane mute on the anchor pane silences the group;
per-pane mute on a non-anchor pane sets that pane's local state
(applied when it becomes the anchor via hover-switch). This
preserves the existing per-pane mute API + IPC surface unchanged.

**Why not a separate global mute in the transport bar.** The audio
anchor mechanic already provides a clean "silence the current
voice" path: clicking mute on the currently-audible pane is the
natural gesture. Adding a global mute button would introduce a
two-level mute (per-pane + global) state diagram, which is more
complex than the user benefit. Anchor H-19 mute discipline does
not apply (H-19 governs hover-preview audio on browse cards;
Multiview audio is live, not preview).

**Reduced-motion (H-22) does not affect mute semantics.** Mute is a
volume-state, not a motion-state. The audio-switch motion (M4b /
M-Q4) collapses to hard-switch under reduced-motion, but the mute
toggle behaves identically in both states.

### M10 — Focus-Mode-Toggle (Q-8 Future Opt-in)

**Decision.** **Deferred to a future v2.x ADR.** Anker Q-8 binding:
"A 'Focus Mode' toggle (leader ~60 % of viewport, followers ~13 %
each) is preserved as a **future opt-in option**, not the default."
This ADR scopes v2.1 Multiview to the equal-quadrants default;
Focus Mode is out-of-scope.

**Rationale.** Focus Mode requires its own pane-reshuffle math
(asymmetric grid: how is the 60 % leader sized, how are the 3
followers stacked, what happens at 2-pane and 3-pane in Focus
Mode), its own toggle affordance, and its own animation between
equal-quadrants and Focus mode (a layout-shift that touches every
pane). Conflating these into the v2.1 ADR would widen the scope
substantially for a feature that Anker Q-8 explicitly tags
"future opt-in." If real-use feedback after Wave-4 ship surfaces
demand, a v2.2 Focus-Mode-ADR can pin it without affecting
anything in this ADR.

### M11 — Out-of-Range per Pane at N > 2

**Decision.** ADR-0022 binding applies **unchanged per pane**:
when a follower pane's VOD range does not contain the leader's
current wall-clock moment, the pane pauses its `<video>` element,
displays the "Out of range — Sightline is waiting for alignment"
overlay, and fires `sync:member_out_of_range` (debounced 1 s
window). The leader keeps playing; the other in-range followers
keep playing.

**At N = 4, up to 3 panes can be simultaneously out-of-range.** The
visual state per out-of-range pane is the existing overlay, drawn
inside the pane's quadrant slot (the pane does **not** collapse
or vacate its slot — that would amount to a layout-reshuffle
mid-session, breaking M1's stable-grid invariant).

**Auto-collapse out-of-range panes is rejected.** Collapsing an
out-of-range pane would re-pack the grid (3 visible + 1 hidden),
shifting the other panes' positions mid-session. This is a
substantive extension over ADR-0022 (which spec'd pause + overlay,
not collapse) and conflicts with M1's stable-grid invariant.
Engineering judgment: out-of-range is rare enough that visible
overlays are tolerable; auto-collapse would be more disruptive than
helpful.

**Performance budget check (ADR-0022 binding).** The 250 ms tick
loop iterates `panes[]`; per-tick cost scales O(N). ADR-0022's
budgets (≤ 1 % CPU idle, ≤ 5 % CPU active on M1 Mac mini, dev
profile) were validated at N = 2. At N = 4, per-tick cost doubles
linearly. Engineering judgment: the budgets remain reachable at
N = 4 (the loop is light: 4 ref-reads + 4 expected-time-computes +
≤ 1 seek per pane per tick). **Documented as Risk #1** with the
mitigation that Wave-4 engineering validates the budgets at N = 4
on the bench machine before ship; if exceeded, the budget is
either widened (with CTO escalation per ADR-0022's existing
escalation path) or the tick cadence is dropped to 500 ms (still
within the 250 ms threshold's perceptual envelope at 4 panes).

### M12 — Token Consumption from ADR-0040 + New Multiview Tokens

**Decision (consumption inventory).** Multiview-chrome consumes the
following ADR-0040 tokens unchanged:

| ADR-0040 token                   | Multiview use                                                          |
| -------------------------------- | ---------------------------------------------------------------------- |
| `--color-bg`                     | Multiview viewport background (Anker H-01, H-05)                       |
| `--color-surface-1`              | Transport-bar background, pane-chrome backdrop pill                    |
| `--color-outline-focus`          | Leader-outline (M5), focus rings on transport-bar buttons              |
| `--space-1` (4 px)               | Inter-pane gap (M1, via `--multiview-pane-gap`)                        |
| `--space-2` (8 px)               | Pane-chrome inner-padding for badge/glyph clusters                     |
| `--motion-zoom-duration` (200 ms)| Pane-add crossfade (M2), Pane-remove fade-out (M3)                     |
| `--motion-easing-out`            | Easing curve for the above                                             |
| `--motion-fade-instant` (100 ms) | Reduced-motion fallback for M2 / M3 animations                         |
| `--z-card-hover` (20)            | Leader-outline z-layer (M5)                                            |
| `--z-toolbar` or `--z-selection-bar` (40) | Transport-bar z-layer (existing in v1; preserved)             |
| `--icon-size-base` (16 px)       | Crown, Volume2 glyphs in pane-chrome (M4e, M5)                         |
| `--icon-size-sm` (14 px)         | Pane-close `X` glyph if M-Q2 resolves to per-pane close                |
| `--icon-stroke-width` (1.5)      | All Lucide glyphs in Multiview chrome (H-23)                           |
| `--font-stack`, `--font-size-xs`, `--font-weight-mid` | Pane-chrome tooltips, "Out of range" overlay text   |
| `--card-corner-radius` (4 px)    | Pane corner-radius (Multiview panes inherit card-radius for visual consistency with TimelinePage and LibraryPage cards) |

**Decision (new Multiview-specific tokens).** Three new tokens are
introduced by this ADR; they will be added to `globals.css` in
Wave-4 engineering:

| New token                              | Value                                  | Justification                                                  |
| -------------------------------------- | -------------------------------------- | -------------------------------------------------------------- |
| `--multiview-pane-gap`                 | `var(--space-1)` (4 px)                | M1 binding. Aliased from `--space-1` for semantic readability  |
| `--motion-audio-switch-delay`          | `200ms` (M-A3 pinned)                  | M4a binding. Separate from `--motion-hover-delay` per M4 rationale |
| `--motion-audio-switch-duration`       | `0ms` (M-A4 pinned, hard-switch)       | M4b binding                                                    |

**Anti-smell verification (R-ADR-01).** The three new tokens are
**Multiview-specific extensions**, not re-opens of ADR-0040
decisions:

- `--multiview-pane-gap` is a semantic alias for `--space-1`, not a
  new spacing value. ADR-0040 TV3 spacing scale is unchanged.
- `--motion-audio-switch-delay` is purpose-distinct from
  `--motion-hover-delay`. M4 explains why Card-Preview hover delay
  (800 ms) doesn't transfer to Audio-Switch.
- `--motion-audio-switch-duration` is purpose-distinct from
  `--motion-zoom-duration` and `--motion-crossfade-duration`
  (which govern card-zoom and thumbnail↔frame-strip crossfade
  respectively).

No ADR-0040 token value is changed by this ADR.

---

## Consequences

### Positive

1. **CEO-A1 commitment delivered.** v2.1 Multiview goes from 2-pane
   (ADR-0021 v1) to 4-pane capacity, with the Anker §6.3 + Q-8
   equal-quadrants posture as the default.
2. **ADR-0021 binding preserved.** 2-pane layout, store shape,
   entry contract, `getVideoSource` IPC reuse, imperative video-ref
   pattern — all unchanged. v1 Multiview keeps working bit-perfect.
3. **ADR-0022 sync math + ADR-0023 transport semantics unchanged.**
   Per-pane scaling is linear in tick cost; out-of-range model
   applies per pane unchanged. Transport stays group-wide; leader
   election extends to N > 2 via the lowest-remaining-index rule
   (M3) compatible with ADR-0023's "remaining pane becomes leader"
   contract.
4. **ADR-0038 D7 Selection-Cap = 4 contract honoured.** CTA enable
   rule 2 ≤ N ≤ 4 routes through `openSyncGroup` with additive
   schema widening, no new IPC.
5. **ADR-0040 tokens consumed without re-litigation.** Leader
   outline uses `--color-outline-focus`; motion uses the existing
   `--motion-zoom-duration` / `--motion-fade-instant`; icons use
   the TV11 Lucide system. Three new Multiview-specific tokens
   are purpose-distinct additions.
6. **Audio-anchor mechanic specified end-to-end** (default at
   leader + hover-switch + state diagram + return-behaviour
   open-question + visualisation). Implementation contract is
   tight enough for Wave 4 to ship without re-asking.
7. **Reduced-motion (H-22) compliance documented** for every
   animation introduced (M2, M3, M4b).
8. **No backend touch, no DB migration.** `openSyncGroup` schema
   widens additively; `sync_sessions.layout` column already exists.
   v2.0.3 → v2.1 upgrade is renderer-only for Multiview.

### Costs accepted

1. **Layout stability over information density.** Out-of-range
   panes keep their grid slot (M11) rather than auto-collapsing,
   accepting "1 dark quadrant out of 4" as a tolerable state in
   exchange for stable mental geometry.
2. **No Focus Mode in v2.1** (M10). Users who want a leader-heavy
   asymmetric layout cannot get it before a future v2.x ADR pins
   Focus Mode geometry.
3. **Per-tick CPU cost doubles at N = 4** vs N = 2 (linear scaling
   per ADR-0022). Budget validation deferred to Wave 4; mitigation
   path (tick-cadence drop to 500 ms) reserved if budgets exceeded.
4. **Three-pane layout resolved per M-A1**: (d) 2×2 with bottom-right
   empty reuses 4-pane geometry. The three alternative shapes
   (a)/(b)/(c) remain in Alternatives Considered as the audit-trail.
5. **Audio-switch behaviour resolved per M-A3 + M-A4 + M-A5**:
   200 ms delay, Hard-switch, Spring-back to leader — adopted as
   a paired block (Compound-Risk #4 acknowledged at sign-off as the
   "gentle, responsive" UX character).
6. **Per-pane close resolved per M-A2**: option (α) ships with a
   Lucide `X` icon in pane-chrome top-right, hover-revealed; backend
   `cmd_close_pane` added as additive IPC endpoint in Wave 4.

### Neutral (forward references and follow-ups)

1. **Wave-4 engineering implements** the layout, audio-routing layer,
   token migration, Lucide glyph install, pane-close affordance (per
   M-Q2 resolution), and the CSS Grid templates per M1. This ADR
   commits the spec; the implementation is downstream work.
2. **CEO-Decision integration into ADR text.** On sign-off, M-Q1
   through M-Q5 become M-A1 through M-A5 with locked values; the
   Decisions sections (M1, M3, M4, M7) update their open-question
   placeholders with the accepted values. Per R-ADR-03 standard
   patch pattern.
3. **Crossfader audio mix (v2.x)** is orthogonal to this ADR's
   audio-anchor mechanic. The anchor identifies "which one pane is
   currently audible"; a crossfader would let the user blend two or
   more pane audio streams via continuous controls. Future ADR.
4. **Resume previous Multiview session (v2.x)** — ADR-0021 Follow-up
   #2 / ADR-0023 Follow-up #2. The DB row already exists post
   migration 0010; resume UI is the new design.
5. **Shareable Multiview deep-link URLs (v2.x)** — ADR-0021
   Follow-up #4. Needs `sightline://` scheme handler.
6. **Per-pane "unlock" mode** (ADR-0023 Follow-up #1) — separate
   ADR if a real use case appears.

### Risks

1. **Per-tick CPU budget at N = 4.** ADR-0022's budgets (≤ 1 % idle,
   ≤ 5 % active on M1 Mac mini, dev profile) were validated at
   N = 2. At N = 4 the loop runs 2× the work per tick. Mitigation:
   Wave-4 validation; if exceeded, drop tick cadence to 500 ms (per
   M11) or escalate to CTO per ADR-0022's existing path.
2. **Audio-routing implementation at N > 2.** v1 audio is per-`<video>`-
   element (each pane outputs independently; users manage volume
   per-pane). v2.1 audio-anchor implies a routing layer that
   silences non-anchor panes. The simplest implementation
   (`pane.volume = anchor ? user_volume : 0`) is correct but
   couples mute-toggle semantics to anchor-state (M9). If
   crossfade (M-Q4 option) is picked, a Web-Audio-Graph
   implementation may be required, expanding scope.
3. **3-pane CSS Grid edge cases at narrow viewports.** The 2×2
   template with one empty slot needs to gracefully degrade at
   narrow widths (window resize, sidebar reveal). Mitigation:
   Wave-4 validates against the minimum supported window-width
   from the existing app constraints.
4. **Compound risk (historical): M-A3 × M-A4 paired adoption.** The
   audio-switch delay and the switch-strategy were **not orthogonal**
   at the PROPOSED phase — together they determine the perceived
   audio-switch UX character. The hypothetical pairings considered:
   - `(0 ms delay, hard-switch)`: instant, aggressive — feels
     reactive but susceptible to pointer-drift artefacts.
   - `(200 ms delay, hard-switch)`: gentle, responsive —
     engineering default; debounces pointer-drift, no audio
     artefacts.
   - `(200 ms delay, 150 ms crossfade)`: smooth, professional —
     more implementation cost, more polished feel.
   - `(800 ms delay, 250 ms crossfade)`: deliberate, cinematic —
     too slow for a viewport-spanning quadrant gesture.

   Resolved 2026-05-12. CEO sign-off adopted the (200 ms,
   hard-switch) pairing as the "gentle, responsive" UX character
   per engineering recommendation.
5. **M-A1 × M-A2 interaction (historical).** The 3-pane layout
   choice (M-A1) and the pane-close affordance (M-A2) interact at
   the pane-close → 3-pane re-pack boundary (M3 4→3 case). At the
   PROPOSED phase, if M-Q1 had resolved to option (a) (2-top +
   1-bottom-centred) or option (c) (3-column), the 4→3 re-pack
   would have been more disruptive (every pane shifts) than under
   option (d) (every pane stays in slot, one slot goes empty).
   Resolved 2026-05-12. M-A1 = (d) 2×2-with-empty minimises the
   re-pack disruption by design; M-A2 = (α) per-pane close ships.

---

## Alternatives Considered

### M1 (Pane-Count Layout Geometry)

- **Resizable split.** Rejected for v2.1 — same rejection as
  ADR-0021 Alternative C (letterboxed `<video>` panes mean
  resizing only changes black-bar ratios).
- **Picture-in-picture overlay (3-pane as "main + 2 inset corners").**
  Rejected. Conflicts with Q-8 equal-quadrants default; PiP is a
  separate layout family (ADR-0021 Follow-up #1).
- **Free-form drag-resize per pane.** Rejected. High implementation
  cost (drag handles, persistence, edge-snap math) for a feature
  Anker Q-8 does not call for; equal-quadrants is the binding
  default.

### M2 (Pane-Add)

- **Pane-add via auto-leader-promotion (the new pane becomes
  leader).** Rejected. Conflicts with ADR-0023 first-opened binding
  and would surprise the user mid-session.
- **Pane-add with slide-in animation from screen edge.** Rejected.
  More motion than necessary; opacity crossfade is consistent with
  ADR-0040 TV6 motion vocabulary.

### M3 (Pane-Remove)

- **Reshuffle the entire grid on every close** (e.g., always pack
  remaining panes from slot 0). Rejected. Disruptive to the user's
  spatial memory; the 4→3 case is the only re-pack needed and only
  if M-Q1 picks a non-(d) option.
- **Leader re-election by geographic proximity** (the leader-spot
  inherits to the nearest remaining pane). Rejected. "Nearest" is
  ambiguous on a 2×2 grid (multiple panes equidistant); index-based
  is deterministic.
- **Leader re-election by user popover** ("you closed the leader —
  pick a new leader"). Rejected. Adds a forced-dialog mid-session;
  ADR-0023 spirit is to let the user promote-leader proactively, not
  forced after a close.

### M4 (Audio-Anchor)

- **Audio always at leader, no hover-switch.** Rejected by Anker Q-8
  binding ("audio switches when the pointer hovers a different
  quadrant").
- **Audio always at hovered pane, with no leader-default.**
  Rejected. Loses Q-8's "audio defaults to the leader" — would mean
  no audio plays when the pointer is in the transport bar or off
  the panes entirely.
- **Click-to-switch instead of hover-switch.** Rejected by Anker
  Q-8 binding ("audio switches when the pointer hovers"). Hover is
  the explicit gesture.
- **Per-pane volume slider in transport bar instead of pane-chrome.**
  Rejected. Conflicts with ADR-0021 binding (per-pane volume in
  pane-chrome) and would clutter the transport bar.

### M5 (Leader-Outline)

- **Accent-coloured leader fill or border.** Rejected by H-20
  binding (neutral white outline only; accent forbidden as focus
  indicator). The current v1 `★ Leader in amber` is anchor-noncompliant
  and gets replaced in this ADR.
- **Drop-shadow / glow halo on the leader pane.** Rejected by H-15
  binding (no Material drop-shadow on cards; ADR-0040 TV7 binds
  card-shadow to `none`).
- **No leader visualisation at all (the leader is invisible chrome).**
  Rejected. ADR-0023 binding includes the leader-badge as an
  affordance ("Hover/focus reveals a 'Promote to leader' tooltip on
  the non-leader pane, with a button" implies the leader is visible).

### M6 (Selection Integration)

- **New IPC endpoint `openMultiPaneGroup({ vodIds[2..4] })`.**
  Rejected. Additive widening of `openSyncGroup` is cleaner; no
  fork in the IPC surface.
- **Separate `layout` enum per pane count (`split-50-50`, `triple-2x2-empty`, `quadrant-2x2`).**
  Rejected. The 3-pane case reuses the 2×2 template with one slot
  empty (M1 recommendation); a separate `triple-*` discriminant
  would imply a different geometric template, which option (d) does
  not need.

### M7 (Pane-Close)

- **Close via right-click context menu on the pane.** Rejected.
  Right-click is browser-OS-specific behaviour and conflicts with
  the platform's text-selection / image-save context. Native-app
  context menus would require Tauri menu plumbing (not in scope).
- **Close via keyboard shortcut only (Cmd/Ctrl+W on the focused
  pane).** Considered. Could ship alongside per-pane close button
  in option (α); not the primary affordance.

### M8 (Transport-Bar)

- **Add audio-anchor display in transport bar.** Rejected. M4e's
  pane-chrome `Volume2` glyph is the single-source visual.
- **Add pane-count indicator in transport bar.** Rejected. The
  layout itself is the indicator (visible grid).

### M9 (Mute-Toggle)

- **Per-pane mute is a "muted forever" persistent property.**
  Rejected. v1 per-pane mute is local-state-per-session, not
  persisted; v2.1 keeps that semantic.
- **Global mute in transport bar replacing per-pane mute.**
  Rejected. Would break ADR-0021 binding; also less expressive
  (user cannot pre-mute a pane they intend to hover later).

### M10 (Focus-Mode)

- **Spec Focus-Mode geometry in this ADR (60 % leader / 13 %
  followers).** Rejected. Anker Q-8 explicitly tags this "future
  opt-in"; spec'ing it here would widen scope for a feature that
  is not the v2.1 default.
- **Drop Focus-Mode entirely.** Rejected. Anker Q-8 binding
  ("preserved as a future opt-in option") — dropping would
  contradict the anchor.

### M11 (Out-of-Range)

- **Auto-collapse out-of-range panes.** Rejected (see M11 Decision
  rationale). Substantive extension over ADR-0022, conflicts with
  M1 stable-grid invariant.
- **Hide out-of-range panes (display: none) until they return to
  range.** Rejected. Same disruption as collapse; the overlay is
  the spec'd behaviour.

### M12 (Tokens)

- **Re-use `--motion-hover-delay` (800 ms) for audio-switch.**
  Rejected. Purpose-distinct (M4 rationale); 800 ms is for
  browse-card preview-debounce, not for in-focus quadrant audio
  routing.
- **Add a single `--multiview-*-duration` covering all Multiview
  motion.** Rejected. The pane-add/remove animations
  (`--motion-zoom-duration`) and the audio-switch animation
  (`--motion-audio-switch-duration`) are independent and may pick
  different values per M-Q3 / M-Q4 resolution.

---

## Implementation Notes

### Component tree (extends ADR-0021 baseline)

```
MultiViewPage  (extended: pane-count-variable layout, audio-anchor state)
├── MultiViewTransportBar  (unchanged structurally; Wave-4 Lucide re-skin)
└── (pane-count-variable grid)
    └── MultiViewPane × N  (extended: audio-anchor indicator, pane-close affordance per M-Q2, neutral leader-outline per M5)
        ├── <video>
        ├── Pane-chrome
        │   ├── Crown glyph (leader only)  [Lucide Crown, new under M5]
        │   ├── Volume2 glyph (audio-anchor only)  [Lucide Volume2, new under M4e]
        │   ├── X close button (per-pane, conditional on M-Q2 = α)  [Lucide X, new under M7]
        │   ├── Volume slider + mute toggle  (existing, ADR-0021)
        │   └── "Promote to leader" button (non-leader only, existing ADR-0023)
        └── Out-of-range overlay  (existing, ADR-0022)
```

### Files (paths)

**Extended (existing):**

- `src/features/multiview/MultiViewPage.tsx`
- `src/features/multiview/MultiViewPane.tsx`
- `src/features/multiview/MultiViewTransportBar.tsx` (Wave-4 Lucide
  re-skin only; no structural change per M8)
- `src/features/multiview/sync-store.ts`

**Unchanged:**

- `src/features/multiview/sync-math.ts` + tests + fixtures
  (ADR-0022 math is N-agnostic)
- `src/features/multiview/use-sync-loop.ts` + tests (loop iterates
  `panes[]`; N change is internal)

**Possibly new (depending on M-Q2 resolution):**

- A small `audio-routing.ts` module that computes `effective_audio`
  per M9 against the `multiview-store` audio-anchor state. Wave-4
  decides whether this is a new module or inlined into
  `MultiViewPane.tsx`.

**No new IPC files in `src/ipc/`** (additive widening of
`openSyncGroup` schema is auto-regenerated via tauri-specta;
no hand-edits).

### Store extension (`multiview-store` extends from `sync-store.ts`)

Additive state on top of ADR-0021 baseline:

```ts
interface MultiViewState {
  // ... ADR-0021 fields unchanged ...

  // M4 — audio-anchor state
  audioAnchorPaneIndex: number | null;
  // null = leader-default (resolves to leaderPaneIndex)
  // non-null = hover-switched anchor

  // M2/M3 — pane-count-variable; panes.length ∈ [0, 2, 3, 4]
  // (no length === 1 state per M1)

  // Actions:
  setAudioAnchor(paneIndex: number | null): void;
  // M2: openSession action accepts vodIds.length ∈ [2, 4]
  // M3: leader re-election rule applied internally on close
}
```

`audioAnchorPaneIndex` is **transient state**: not persisted, not
synced via IPC, reset to `null` on session close. The audio-anchor
is a UI affordance; the source of truth for "which pane is leader"
remains the backend-confirmed `leaderPaneIndex` per ADR-0023.

### IPC extension (additive)

```ts
// Existing: openSyncGroup({ vodIds: string[], layout: string }): SyncSession
// v1.0 invariant: vodIds.length === 2, layout === "split-50-50"

// v2.1 additive widening:
//   vodIds.length ∈ [2, 4]
//   layout ∈ "split-50-50" | "quadrant-2x2"
//   (renderer sends "split-50-50" at N=2 for back-compat, "quadrant-2x2" at N ∈ {3, 4})

// Backend validator update (Wave 4):
//   - cmd_open_sync_group: length-check from `== 2` to `2 ≤ length ≤ 4`
//   - layout discriminant: accept new "quadrant-2x2" string value
//   - sync_sessions.layout column already exists (migration 0010)
```

**Backwards compatibility.** A v2.0.3 client calling
`openSyncGroup({ vodIds: [a, b], layout: "split-50-50" })` continues
to work bit-perfect under the v2.1 backend. No migration. No
breaking change.

### Test strategy

- **`sync-math.test.ts`** — extend test fixtures to include N = 3
  and N = 4 cases (pure-function tests; ADR-0022 math is N-agnostic
  but test coverage should reflect the supported N range).
- **`sync-store.test.ts`** — extend to cover:
  - Open session with N ∈ {2, 3, 4} VOD ids
  - Audio-anchor state transitions (default-at-leader, hover-set,
    hover-clear per M-Q5 resolution)
  - Pane-remove leader re-election (M3 rule applied)
- **`use-sync-loop.test.tsx`** — extend to cover N = 4 tick
  behaviour and verify per-tick cost stays within 4 × the N = 2
  measurement (Risk #1 mitigation).
- **`MultiViewPage.test.tsx` (new)** — snapshot tests for each
  pane-count variant (1 N=2 + 1 N=3 + 1 N=4 = 3 snapshots).
- **Audio-anchor behavioural test** — Playwright (when E2E gating
  lands; deferred per ADR-0038 D-section gating).

### Migration path

v2.1 ship is renderer-only for Multiview. No DB migration; no
backend schema change; no `stream_intervals` rebuild. The
`sync_sessions` table accepts `layout = "quadrant-2x2"` without
schema change (existing string column). Users upgrading from v2.0.3
to v2.1 see the renderer pick the new layout when N ∈ {3, 4}
selections come in; existing 2-pane sessions render identically to
v1.0.

---

## Open Questions for CEO — RESOLVED 2026-05-12

This ADR surfaced **five Open Questions** at the PROPOSED phase, all
resolved on 2026-05-12 as a block per the Forward-as-is
recommendation in the Escalation note below. Per **R-ADR-04**
(≥ 5 OQ threshold), the Escalation note was the formal
forward-process surface. The OQ blocks below are preserved as the
audit-trail; the Acceptance-Block at the top of this ADR documents
the pinned M-A1..M-A5 values.

### M-Q1 → M-A1 — 3-Pane Layout: (d) 2×2 with empty bottom-right (ACCEPTED 2026-05-12)

**Question.** Which of four candidate 3-pane layouts should v2.1
adopt (see M1 ASCII diagrams)?

- (a) 2-top + 1-bottom-centred
- (b) 1-top-wide + 2-bottom-half
- (c) 3-column row
- (d) **Engineering recommendation:** 2×2 grid with bottom-right slot empty (reuses 4-pane template; minimises 4→3 re-pack)

**Engineering recommendation:** **(d)**. Reuses 4-pane CSS Grid
template, keeps pane aspect consistent across 3-pane and 4-pane,
minimises layout shift on 4→3 transition.

### M-Q2 → M-A2 — Pane-Close Affordance: (α) per-pane close button (ACCEPTED 2026-05-12)

**Question.** Should v2.1 ship with:

- (α) **Per-pane close button** (Lucide `X` in pane-chrome top-right, hover-revealed) — **engineering recommendation**
- (β) Group-close only (transport-bar Close button only, status quo from ADR-0021)

**Engineering recommendation:** **(α) per-pane close**. At 4-pane
capacity, the "always restart to drop one" cost is higher than at
2-pane; per-pane close is a small chrome addition with clean
semantics (M3 leader re-election rule already defined).

### M-Q3 → M-A3 — Audio-Switch Delay: 200 ms (ACCEPTED 2026-05-12)

**Question.** What delay should the audio-anchor apply before
switching audio on hover-of-non-leader-pane?

- 0 ms (instant)
- **200 ms (engineering recommendation)** — mid-band; debounces pointer-drift; feels responsive
- 800 ms (= `--motion-hover-delay` from Card-Preview) — too slow for in-focus quadrant gesture (M4 rationale)

**Engineering recommendation:** **200 ms**. Defines a new
purpose-distinct token `--motion-audio-switch-delay` per M12.

### M-Q4 → M-A4 — Audio-Switch Strategy: Hard-switch (ACCEPTED 2026-05-12)

**Question.** How should the audio-anchor switch perform the
audio transfer?

- **Hard-switch (instant mute + unmute) — engineering recommendation** — simpler, predictable, no implementation overhead
- Crossfade (e.g., 150 ms volume-ramp on both panes) — professional-cinematic, but adds Web-Audio-Graph implementation cost and a click-event-during-transition edge case

**Engineering recommendation:** **Hard-switch** for v2.1 (defines
`--motion-audio-switch-duration: 0ms`). Crossfade as a v2.x
follow-up if real-use feedback flags abrupt cuts.

### M-Q5 → M-A5 — Audio-Return Behaviour on Hover-Exit: Spring-back to leader (ACCEPTED 2026-05-12)

**Question.** When the user's pointer exits the hovered (audio-
anchored) non-leader pane, what happens to audio?

- **Spring-back to leader (engineering recommendation)** — audio is always tied to a deliberate hover; absence of hover defaults to the leader. Matches Q-8 "audio defaults to the leader" semantic
- Sticky-stay on last-hovered pane — audio stays where the user last directed attention until they redirect it

**Engineering recommendation:** **Spring-back**. Preserves the
leader's role as the default voice; aligns with Q-8 default
binding. Engineering default for **M4d** (viewport-exit) follows
the M-Q5 resolution by symmetry.

---

## Escalation note for CTO — RESOLVED 2026-05-12

This ADR surfaced five Open Questions for CEO (M-Q1 through M-Q5),
all resolved on 2026-05-12 as a block per the Forward-as-is
recommendation below. The threshold sat **exactly at the R-ADR-04
escalation threshold** of ≥ 5. R-ADR-04 documents three
Forward-Process options for the CTO; this Escalation note
recommended and explained the chosen path, which the CEO confirmed
without substance-patches.

### Forward-Process recommendation: **Forward-as-is (Option 1)**

All five OQs are accompanied by clear engineering recommendations
with rationale. The CEO can adopt as a block (precedent: Welle 1
4-decision-block, Welle 2 3-decision-block, Welle 3 7-decision-block
— all clean Block-Acceptance, no Re-Litigation per R-ADR-04
empirical observation). Per-OQ recommendations:

| OQ      | Engineering recommendation | CTO concur (Y/N)? | Rationale-tag for CEO brief                                  |
| ------- | -------------------------- | ----------------- | ------------------------------------------------------------ |
| M-Q1    | (d) 2×2-with-empty         | Y                 | Reuses 4-pane template, minimises 4→3 re-pack disruption     |
| M-Q2    | (α) Per-pane close         | Y                 | 4-pane capacity makes per-pane close worth the chrome cost   |
| M-Q3    | 200 ms                     | Y                 | Mid-band — responsive without pointer-drift artefacts        |
| M-Q4    | Hard-switch                | Y                 | Simpler, predictable; crossfade as v2.x follow-up            |
| M-Q5    | Spring-back                | Y                 | Aligns with Q-8 "audio defaults to leader" default-binding   |

### Why not Subdivide (Option 3)?

Per R-ADR-04 Subdivide-pfad-Warnung from Welle 3 retrospective:
classifying OQs as "tuning" vs "structural" is non-trivial. Here:

- **M-Q1** is **structural** (it picks the 3-pane CSS Grid template;
  Wave 4 builds different code paths under different options).
- **M-Q2** is **structural** (per-pane close is a new IPC endpoint
  + handler; group-only is no change).
- **M-Q3** is **tuning** (single-token value).
- **M-Q4** is **structural** (hard-switch is no extra impl; crossfade
  is a Web-Audio-Graph layer).
- **M-Q5** is **UX-feel** (neither pure tuning nor pure structural;
  affects user mental model).

Subdivide would force a structural-vs-tuning split that maps awkwardly
onto these five OQs. Forward-as-is is cleaner.

### Why not Pre-process (Option 2)?

Per R-ADR-04 Pre-process-Pfad-Warnung: introduces a third
"CTO-approved engineering defaults" category outside the
engineering-default / CEO-decision binarity. For 5 OQs with
clear UX impact (M-Q1, M-Q4, M-Q5 are all user-feel-prägend),
Pre-process would compress the CEO out of the decision they
should make.

### Compound-Risk callout

**M-Q3 × M-Q4 are non-orthogonal.** The audio-switch delay and the
switch-strategy together determine the perceived UX character. CEO
sign-off should pair these decisions — adopting `(200 ms delay,
hard-switch)` as a pair (engineering recommendation) gives the
"gentle, responsive" character; mismatched pairings (e.g.,
`(800 ms, hard-switch)` or `(0 ms, 250 ms crossfade)`) produce
non-linear UX shifts. Documented in Consequences > Risks #4.

---

## References

- **Spec sources (binding).**
  - `docs/reference/visual-language-netflix.md` §6.3 MultiviewPage mapping
  - `docs/reference/visual-language-netflix.md` §7 Q-8 Multiview leader-quadrant prominence
  - `docs/reference/visual-language-netflix.md` H-01, H-05, H-08, H-19, H-20, H-22, H-23
  - `docs/decision-log/v2.1-anchor-acceptance.md` §1 Q-8 + §2 structural correction
  - `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 CEO-A1
- **Related ADRs.**
  - [ADR-0019](0019-asset-protocol-scope.md) — asset protocol (unchanged)
  - [ADR-0021](0021-split-view-layout.md) — Multiview v1 foundation
  - [ADR-0022](0022-sync-math-and-drift.md) — sync math (scales linearly per pane)
  - [ADR-0023](0023-group-wide-transport.md) — transport semantics; audio-anchor is additive layer
  - [ADR-0038](0038-timeline-layout-single-time-axis.md) — CEO-A1 source, D7 Selection-Cap = 4
  - [ADR-0040](0040-visual-system-v2.md) — token consumption for chrome
- **Source files referenced.**
  - `src/features/multiview/MultiViewPage.tsx` (extended)
  - `src/features/multiview/MultiViewPane.tsx` (extended)
  - `src/features/multiview/MultiViewTransportBar.tsx` (Wave-4 Lucide re-skin)
  - `src/features/multiview/sync-store.ts` (additive state)
  - `src/features/multiview/sync-math.ts` / `use-sync-loop.ts` (unchanged)
  - `src/ipc/bindings.ts` (`openSyncGroup` additive widening, auto-regenerated)
