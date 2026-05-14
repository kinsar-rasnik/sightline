# ADR-0039 — VodCard: Composition, Hover-Preview, Multi-Perspective-Stack, Retention-Countdown

- **Status.** Accepted 2026-05-12
- **Date.** 2026-05-12 (drafted PROPOSED) → 2026-05-12 (accepted by CEO sign-off)
- **Acceptance.** CEO sign-off 2026-05-12 with three engineering defaults
  adopted as a block, no DV substance change — **VOd-A2** (Retention
  source = client-side compute from `streamer.broadcasterType` +
  `vod.streamStartedAt`; backend `retention_at` STORED column moves to
  the v2.2 backlog with explicit trigger of real-use-drift reports),
  **VOd-A3** (Completed-watch indicator = option (b), a neutral
  checkmark glyph in the metadata strip's Line 2 Column 2,
  right-aligned; Wave-3 thumbnail-opacity reduction reserved as a
  visual-token follow-up if real-use feedback surfaces an
  "every-card-has-checkmark" background-noise pattern, single-line
  CSS change in the visual-token layer, no ADR re-litigation),
  **VOd-A5** (Stack-card fan-out edge trajectory = auto-mirror to
  lateral left; vertical and suppress rejected on lane-packing-
  collision and edge-card-behaviour-divergence grounds). Per-decision
  rationale and Folge in `docs/decision-log/v2.1-adr-0039-vodcard.md`
  §7. The CTO forwarded the three OQs as-is to the CEO (Option 1 of
  the three forward-process options the Escalation note documented);
  the CEO accepted all three engineering defaults as a block without
  substance change. Anti-Smell audit pattern from Wave 1 confirmed
  working — no hidden sub-decisions surfaced post-sign-off.
- **Wave.** v2.1 ADR Wave 2 of (Timeline-Layout → **VodCard** → VisualSystem-v2).
  Parallel v2.1 wave: Multiview-Pane-Expansion-ADR (per ADR-0038 CEO-A1) —
  filed separately; referenced normatively from this ADR where the
  `Open in Multiview` CTA on a stack-card crosses the contract boundary.
- **Related.**
  [ADR-0038](0038-timeline-layout-single-time-axis.md) (D5 stack-card
  Click-Routing + Fan-out invocation, D7 selection visual = H-20 outline
  + 16-px accent checkmark, D9 hero-slot favourited-trigger — the VodCard
  in the hero slot is a separate render variant, not specified here) ·
  [ADR-0008](0008-chapters-via-twitch-gql.md) (Twitch-GQL chapters
  feed — VodCard does NOT render chapter data in v2.1; chapters live in
  Detail Overlay) ·
  [ADR-0013](0013-sidecar-bundling.md) +
  [ADR-0034](0034-tauri2-sidecar-layout.md) (yt-dlp / ffmpeg sidecar
  layout — VodCard does not invoke sidecars directly; the frame-strip
  asset pipeline runs at VOD-ingest in `services::downloads`) ·
  [ADR-0015](0015-timeline-data-model.md) (`stream_intervals` feeds the
  parent TimelineLanes; per-card data via `getVod`) ·
  [ADR-0018](0018-watch-progress-model.md) (`watch_progress` is the
  3-px stripe's source) ·
  [ADR-0019](0019-asset-protocol-scope.md) +
  [ADR-0027](0027-asset-protocol-scope-narrowing.md) (asset protocol
  serves the thumbnail + frame-strip bytes; `getVodAssets` is the
  service-layer choke point — no new asset endpoint in this ADR) ·
  [ADR-0033](0033-library-ui-redesign.md) (existing `VodCard.tsx` in
  LibraryPage; this ADR specifies the v2.1 successor used by both
  TimelinePage and LibraryPage, with the v2.0.1 VodCard rewritten to
  match — implementation in Wave 4 engineering).
- **Spec sources (binding).**
  `docs/reference/visual-language-netflix.md` §3.2 (Thumbnail Card),
  §3.3 (Hover Preview Card), §6.2 (VodCard mapping), §7 (CEO Decisions:
  Q-3 GIF strip / Q-5 stack silhouette + fan-out / Q-6 keyboard cascade
  via CEO-A4 / Q-7 retention 3-stage escalation / Q-10 3-px progress
  stripe). Decision-log:
  `docs/decision-log/v2.1-anchor-acceptance.md` Q-3/Q-5/Q-6/Q-7/Q-10 and
  `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 CEO-A4
  (Q-6 keyboard-cascade as binding forward reference).

## Context

The VodCard is the atomic visual element of Sightline's v2.1 browse
language. It is what the user actually sees — once on the TimelinePage
(per ADR-0038's §3.6 Single-Time-Axis layout), and once on the
LibraryPage's continue-watching rail and grid. The pattern §3.2 +
§3.3 + §6.2 of the Netflix anchor binds the card's *shape*; the CEO
Decisions Q-3 / Q-5 / Q-7 / Q-10 plus the CEO-A4 cascade bind its
*behaviour* at the points where Sightline's domain diverges from the
anchor (no audio in the preview, multi-perspective stack-cards, 7-d /
48-h / 24-h retention escalation, 3-px continue-watching stripe).

Predecessor components survey:

- `src/features/vods/VodCard.tsx` (310 lines) — the Phase-8 / ADR-0033
  Library-grid card. Already cycles a 6-frame `previewFramePaths` set
  at 400 ms / frame, already renders a 1-px watch-progress bar at the
  bottom of the image, already shows a small "✓" badge at 100 %
  watched. The pre-Phase-8 visual language is **off-anchor on multiple
  axes**: accent-coloured idle border on selected (H-07 violation),
  accent-coloured focus outline (H-20 violation), accent-filled
  checkmark badge fillout (H-07 borderline), 1-px progress bar (Q-10
  prescribes 3-px). This ADR specifies the v2.1 successor; the
  rewrite lands in Wave 4 engineering and replaces the v2.0.1
  component end-to-end (no two-VodCard-variants in the codebase
  after Wave 4).
- `src/features/player/ContinueWatchingRow.tsx` — a thinner card
  variant on the LibraryPage's H-25 rail (Q-10 on Library); same
  thumbnail + small metadata strip + 1-px stripe shape. Folds into
  the same v2.1 VodCard via a `variant="compact"` prop (DV1).
- `src/features/timeline/TimelinePage.tsx` — Phase-4 proof-of-concept;
  renders intervals as `<button>` bars *without* a card. Replaced
  outright by ADR-0038's `TimelineCard` (which mounts the v2.1
  VodCard).

Data-layer survey:

- `getVodAssets({ vodId }) → VodAssets { thumbnailPath, previewFramePaths }`
  exists today. `previewFramePaths` is **always 6 frames**, evenly
  spaced through the VOD timeline (`PREVIEW_FRAME_PERCENTS` in
  `services/downloads.rs:978`), pre-generated at ingest by
  `extract_preview_frames` (`services/downloads.rs:985`) and
  back-filled by the `backfill_preview_frames` background task in
  `services/media_assets.rs:173`. **No new IPC endpoint, no new
  ffmpeg invocation, no new storage budget needed** to fulfil Q-3:
  the bytes are already on disk; the v2.1 question is how the
  renderer sequences them.
- `streamers.broadcaster_type` exists in the schema (`partner` /
  `affiliate` / `''` per `docs/data-model.md` §Phase 2). **No
  `retention_at` field exists** — retention is a client-side
  derivation in v2.1 (see DV5; VOd-A2 in CEO Decisions §below
  accepts client-compute as the v2.1 default and moves
  `retention_at` to the v2.2 backlog).
- `getWatchProgress({ vodId }) → WatchProgressRow | null` exists;
  `watched_fraction` is a `STORED` generated column (ADR-0018).
- `getCoStreams({ vodId }) → CoStream[]` exists; sorted by overlap
  length descending. ADR-0038 D5 specifies the client-side filter to
  ≥ 50 % overlap. Stack-detection in the VodCard subscribes to this.

The decisions below are the *composition*, *states*, *data
touchpoints*, *accessibility envelope*, and *performance
architecture* of the VodCard. They are deliberately bounded:

- **Out of scope.** Exact pixel sizes, hex values, opacity ramps,
  border-radius numerics, lane-height-px, hover-zoom factor numeric
  — these go to Wave 3 (ADR-VisualSystem-v2) under the contract this
  ADR establishes (16:9 image, ≥ 75 % image-area share, 3-px stripe
  *order of magnitude*, H-20 outline width *order of magnitude*).
- **Out of scope.** GIF-pipeline implementation (yt-dlp flags, ffmpeg
  filter chain, sidecar job scheduling). The asset format and
  generation strategy are fixed (DV10), but the build-time
  parameterisation of the existing ingest pipeline (PNG vs. WebP
  follow-up, frame-count tunable, etc.) lives in a separate
  Engineering wave after Wave 4 if PNG-cycle perf turns hot.
- **Out of scope.** Multiview pane-render logic — the separate
  Multiview-Pane-Expansion-ADR (CEO-A1) handles that contract; this
  ADR routes selection / fan-out / Multiview-CTA through ADR-0038's
  D5 / D7 surface without re-litigation.

---

## Decisions

### DV1 — Card-Geometry Contract

**Decision.** The VodCard is composed of two stacked regions:

1. **Image area** — 16:9 aspect ratio, occupying **≥ 75 %** of the
   card's bounding height. This is the H-16 invariant; the binding
   spec is `image_area_height ≥ 0.75 × card_height`.
2. **Metadata strip** — occupying **≤ 25 %** of the card's bounding
   height, sitting *below* the image area (not as an overlay scrim
   on the image; §3.2 allows either layout, this ADR picks
   *below-image* for legibility at small zoom-level widths).

The card's bounding height is `lane_height_px` (the variable ADR-0038
D4 already exposes for the N_max-per-side computation). The
exact `lane_height_px` value is delegated to ADR-VisualSystem-v2.

**Width is zoom-dependent, with three render variants.** Because
ADR-0038 §3.6 anchors cards on the time axis by `start_at` with
horizontal extent corresponding to duration × pixels-per-second(zoom),
the card's width is not a fixed constant. It scales with the active
zoom level. Three variants govern what renders at a given width:

| Variant   | Width range (delegated)         | Renders                                              |
| --------- | ------------------------------- | ---------------------------------------------------- |
| **Full**  | ≥ "card-readable-width-px"      | Image area + metadata strip (both lines)             |
| **Compact** | "card-readable-width-px" > w ≥ "card-image-only-width-px" | Image area + single-line metadata strip (title only, truncated) |
| **Sliver** | < "card-image-only-width-px"   | Image area only; no metadata strip; rounded slim bar |

The exact "card-readable-width-px" / "card-image-only-width-px"
thresholds are **delegated to ADR-VisualSystem-v2**. The contract this
ADR establishes is: **three variants, monotonic width-threshold
ladder, no per-pixel customisation** outside the ladder.

**At Levels 5–6 (7 d / 30 d viewports), most cards fall into the
Sliver variant.** A 4-hour stream on a 30-day viewport is ~6 px
wide — Sliver is the only honest rendering at that density. This is
the same fall-back that lets D4's stack-marker compose with Q-5's
stack silhouette without visual collision; both surfaces shrink
together.

**Hero-slot variant (D9 of ADR-0038).** When the VodCard is rendered
in the conditional Hero-Slot (favourited streamer live), the variant
is **Hero**: larger card scale, expanded metadata at idle (no hover
needed for the metadata reveal), one primary "Watch" CTA carrying the
accent. The Hero variant is sibling to Full / Compact / Sliver, used
exclusively in `TimelineHeroSlot.tsx` — **out of scope for this ADR**;
its composition follows §3.1 Hero Row plus the Q-2 conditional
trigger logic, and lands in Wave 3 / Wave 4 alongside the
ADR-VisualSystem-v2 tokens.

**Rationale.**

1. H-16 (≥ 75 % image dominance) is binding. The 16:9 image area in a
   75-% / 25-% split sets the card's intrinsic aspect: roughly
   `card_aspect ≈ 16 / (9 / 0.75) = 16 / 12 = 4 / 3` — wider than
   square, taller than 16:9. At lane-height ~80 px this gives ~107 px
   width; at lane-height ~120 px, ~160 px. The exact lane-height
   number is for ADR-VisualSystem-v2; the *aspect* is locked here.
2. The three width-variant ladder is the same logic the Library grid
   already uses informally — full / compact / list-row — but expressed
   here as a discrete contract so the rendering layer doesn't have to
   re-derive it per surface.
3. Hero variant kept separate because the Hero-slot is a §3.1 pattern
   not a §3.2 pattern; trying to overload the Thumbnail Card to act
   as the Hero would compromise both.

**Fallback for engineering implementation.** If the Sliver variant
is unreadable at typical viewer-densities (a < 30 px sliver carries
no information beyond "something happened here"), drop the Sliver
variant to a pure tick mark (1-px coloured by streamer-tier) and
let the stack-marker (D4) handle aggregation. This trade-off
delegates to ADR-VisualSystem-v2 along with the threshold numerics.

### DV2 — Idle-State Composition

**Decision.** Idle-state composition has two layers:

**Image area** (overlays in order):
1. Background image: local `thumbnailPath` from `getVodAssets`
   (served via Tauri asset protocol per ADR-0019/0027). Fallback
   chain: local `thumbnailPath` → remote `vod.thumbnailUrl` (Twitch
   CDN) → skeleton.
2. *(conditional)* Bottom-right corner: retention badge at < 48 h
   stages (Q-7; see DV5).
3. *(conditional)* Top-right corner: selection checkmark at
   `selected === true` (D7 of ADR-0038; see DV7).
4. *(conditional)* Bottom edge: 3-px continue-watching stripe at
   `0 < watched_fraction < 1` (Q-10; see DV6).

**Metadata strip** — single column, two lines (in Full variant):

```
┌─────────────────────────────────────────────┐
│                                             │
│        16:9 image area                      │
│        (Full variant: ≥ 75 % card-h)        │
│                                             │
│  [⬛ selection ↗]      [retention ↙]        │
│  ▆▆▆▆▆░░░░░░░░░░░░░░░░░░░░░░░  3-px stripe │
├─────────────────────────────────────────────┤
│ 🟡 Display Name · Title (1 line, ellipsis)  │  ← line 1
│ 4h 12m · expires in 4d                      │  ← line 2 (tabular)
└─────────────────────────────────────────────┘
```

Line 1 of the strip is a 3-column flex:

- **Column 1** (auto-width, ~16 px): streamer avatar (Single-VOD) **or**
  row of up to 4 avatar-dots (Stack-Card, DV4). Avatars resolved from
  `streamer.profileImageUrl`; the row reads left-to-right in
  overlap-length-descending order (matches `getCoStreams` ordering
  invariant).
- **Column 2** (auto-width, no shrink): streamer display name, neutral
  body weight per H-09.
- **Column 3** (1fr, shrinks, ellipsis on overflow): VOD title,
  neutral body weight per H-09 + H-11. Title separator
  ("·") sits between Column 2 and Column 3.

Line 2 is a 2-column flex:

- **Column 1** (1fr): duration in `tabular-nums` (formatDurationSeconds
  → "4h 12m"), neutral muted text per H-11.
- **Column 2** (auto, right-aligned): retention text label *if and only
  if* Q-7 stage 2 (7 d ≥ X > 48 h) — "expires in 4d". At Q-7
  stages 3 (48 h) and 4 (24 h) the badge moves to the image-area
  overlay (see DV5); at Q-7 stage 1 (> 7 d) this column is empty.

**Compact variant** drops Line 2 entirely; Line 1 stays, but Column 1
(avatar) is hidden (it's the only thing easily droppable without
losing the title).

**Sliver variant** drops the metadata strip altogether; the image
area is the entire card.

**Skeleton fallback.** When `thumbnailPath` is null (post-ingest
backfill incomplete) AND `vod.thumbnailUrl` is also null
(extremely rare — Twitch returns a thumbnail for every VOD
post-publish), render a **static neutral skeleton** at the image
area's bounds. The skeleton is H-22-compliant (no shimmer, no
breathing — anchor §3.5 "Loading state" prescribes unanimated
skeletons). The metadata strip in this state stays populated (we
have `vod.title`, `streamerDisplayName`, `duration` from the row
already), so the card is still readable; only the image is
placeholder-filled. **No generic placeholder image** (e.g., the
streamer's profile-image): that would mis-signal which VOD this is
when multiple of the same streamer's VODs are loading at once.

**Forward reference.** Exact corner-radius (H-13 range 2–6 px),
selection-checkmark size (≈ 16 px square), retention-badge size,
streamer-avatar size, stripe height (≈ 3 px) — all delegated to
**ADR-VisualSystem-v2**. The compositional invariants (relative
hierarchy, no-collision corner positions, H-16 ≥ 75 % image share)
are locked here.

### DV3 — Hover-Preview Sequence

**Decision.** The hover preview is a deterministic time-choreographed
animation, gated by a sustained 800 ms delay on either mouse-hover or
Tab-focus (CEO-A4 cascade), with a `prefers-reduced-motion` fallback
and an explicit no-trigger touch path.

**Trigger gates.**

- **Mouse**: pointer enters card → timer starts → if pointer remains
  inside the card hit-area for **800 ms**, trigger fires.
- **Keyboard (CEO-A4)**: focus arrives on card → timer starts → if
  focus remains on the card for **800 ms**, trigger fires.
- **Touch**: never triggers preview. A single tap is interpreted as
  the default Click action (open Detail Overlay per D5 of ADR-0038).
  Long-press is not a hover-preview gesture in v2.1; deferred to v2.2
  if user research surfaces a touch-affordance need.

**Default delay = 800 ms.** Picked as the midpoint of the H-18
0.6–1.0 s range. The same value is used for both mouse and keyboard
per CEO-A4 — there is no separate "keyboard delay". The 800 ms is a
locked constant in `src/features/timeline/use-hover-preview.ts`; the
Wave-3 ADR may revisit if perceptual testing surfaces a different
midpoint.

**Choreography (mouse or keyboard, identical).**

```
t = 0     :  trigger gate engaged (pointer enter / focus arrive)
            |
            |  for keyboard-only path: H-20 outline renders immediately at t=0
            |  (focus is visible without preview-trigger; see DV7)
            |
t = 800 ms:  delay expires → entry animation begins
            ├─ card scale interpolates 1.0 → 1.08–1.5× over 180–240 ms (H-17, H-21)
            ├─ expanded metadata panel fades / slides in over 180–240 ms (H-21)
            └─ frame-strip crossfade begins:
                ├─ static thumbnail opacity 1 → 0 over ~250 ms
                └─ frame-strip opacity 0 → 1 over ~250 ms
                └─ frame-strip loop starts at frame 0

t ≥ 800 + 250 ms = ~1050 ms:  full preview state
            └─ frame-strip loops at 400 ms / frame, 6 frames, 2.4 s cycle
            └─ loop continues until exit-trigger
```

**Exit.**

```
mouse leaves card AND focus is not on card  → exit-trigger
            |
            ├─ frame-strip → static thumbnail crossfade (~250 ms)
            ├─ card scale 1.X → 1.0 over 180–240 ms (H-21)
            └─ expanded panel fades out over 180–240 ms (H-21)
            └─ ~250 ms total exit duration (parallel)

resume to idle-state composition (DV2)
```

**Cascade semantics (CEO-A4).** When mouse-hover and Tab-focus
overlap on the same card, the trigger-state is **OR**: any active
trigger keeps the loop running. Concretely:

- **Path A.** Mouse-hover starts → 800 ms passes → loop starts.
  User Tab-focuses *to the same card*. Loop continues (focus is now
  also active). User moves mouse off → loop *continues* (focus
  still active). User Tab-focuses elsewhere → loop ends (both
  triggers cleared).
- **Path B.** Tab-focus arrives → 800 ms passes → loop starts.
  User moves mouse over the card. Loop continues. User Tab-focuses
  elsewhere (mouse still on card) → loop *continues* (hover still
  active). Mouse leaves → loop ends.
- **Delay restart.** Each trigger source has its own 800-ms timer.
  If mouse-hover starts and the timer is at 400 ms, then Tab-focus
  arrives, the focus timer starts at 0 ms; the loop fires at the
  *minimum* of (mouse-timer-elapsed, focus-timer-elapsed) crossing
  800 ms — i.e., the first source to clock 800 ms wins, and the
  loop persists until *both* sources are absent.

**Reduced-motion (`prefers-reduced-motion: reduce`).**

- No scale: card stays at 1.0.
- No frame-strip loop: the static thumbnail does *not* crossfade
  out; the frame-strip is never mounted.
- Expanded-metadata panel **does** appear (information must stay
  reachable per H-22), but **instantly** or via an opacity-only fade
  of ≤ 100 ms — no scale, no slide.
- H-20 focus outline behaves identically to allow-motion (it's not a
  motion-bearing element).
- Touch path is unchanged (still no preview; tap → Detail Overlay).

**Mute toggle (H-19).** Not applicable in any state. The
frame-strip carries no audio (Q-3 cascade). Confirmed binding in
§6.2 of the anchor; no H-19 affordance is rendered.

**Race / abort semantics.** If the trigger gate engages and the user
exits *before* the 800-ms delay, the timer is cancelled cleanly; no
loop is ever started; no crossfade is ever performed. This is the
"quick-pass" guard from H-18.

**Singleton invariant.** At most **one card** on a page-mount is in
the "loop is playing" state at any given instant. When a new card's
trigger gate fires its loop, any other card currently looping
**aborts to idle immediately** (no exit animation — instant cut,
because the previous card is no longer the user's focus). This is
the implementation contract DV10 (Performance Architecture)
formalises; mentioned here because it is part of the sequence
semantics.

### DV4 — Multi-Perspective Stack Card

**Decision.** A multi-perspective moment (per ADR-0038 D5: ≥ 50 %
time-overlap of the shorter interval, both streamers in the tracked
set) renders as a **single VodCard at the position of the
longest-running interval** with three additional visual / behavioural
layers on top of DV2's base composition.

**Layer 1 — Stack silhouette (idle).**

- **2 layers visible** behind the frontmost card. The bottom edge of
  the second-layer card peeks ~2 px below the frontmost card's
  bottom edge; the third-layer card peeks an additional ~2 px below
  the second. No horizontal offset (a horizontal offset would
  resemble the lateral fan-out trajectory and create premature
  confusion).
- **Maximum 2 silhouette layers** (= 3 cards visually implied total:
  frontmost + 2 behind). Groups of 4+ perspectives use this same
  3-layer silhouette plus avatar-dot truncation (Layer 2 below).
- Exact "~2 px" offset value delegated to ADR-VisualSystem-v2; the
  *count* (2 silhouette layers max) is locked here.

**Layer 2 — Avatar-dots row in the metadata strip.**

- Replaces the single streamer avatar in DV2's strip Line 1
  Column 1.
- Up to **4 avatar dots** visible, then **"+N more"** as a 5th
  inline element if the group has > 4 perspectives.
- Each dot resolves from the corresponding streamer's
  `profileImageUrl`; falls back to streamer initials in a neutral
  circle if the image fails.
- **Order: by overlap length descending** — matches `getCoStreams`
  IPC sort. The leftmost dot is the most-overlapping
  perspective (the "tightest" co-perspective); the rightmost is the
  shortest-overlapping included perspective. The frontmost-card's
  streamer is included (it *is* the longest-running interval; its
  avatar is in the row to keep the set complete and signal "the
  card you see is also one of these perspectives").
- The "+N more" element, when shown, is **non-interactive in the
  idle state** — it expands only via the fan-out gesture (Layer 3).
  It does not open a popover or tooltip; the affordance is the
  same as the rest of the stack.

**Layer 3 — Hover fan-out (Layer 1's animation, Layer 2's expansion).**

- **Trigger:** hover or Tab-focus sustained 800 ms (same gate as
  DV3; the trigger gates and timers are shared, not duplicated).
- **Trajectory: lateral right (default).** The stacked cards fan
  out to the **right** of the frontmost card. Each fanned card
  has the same width as the frontmost; the inter-card gap is
  `card_width × 0.25` (illustrative; exact value delegated to
  ADR-VisualSystem-v2).
- **Edge-of-viewport mirror (accepted per VOd-A5, CEO Decisions
  §below).** If the right-side fan would exceed the viewport
  right edge (`frontmost_x + n_fanned × card_width × 1.25 >
  viewport_width`), the fan trajectory mirrors to **lateral
  left**. Vertical fan-out is rejected (it would collide with the
  next lane's cards above / below the axis; ADR-0038 D4's
  vertical lane packing makes vertical fan-out a layout-corrupting
  move). Pure suppression (`no fan-out at edge, click-only`) is
  rejected (it denies the affordance Q-5 promises and makes
  edge-card behaviour divergent from the rest of the lane).
  Auto-mirror is the smart-direction pattern shared with dropdowns
  and tooltips — users learn it in a single interaction; CEO
  Decisions §below records the trade-off.
- **Frame-strip loops only on the frontmost card.** During fan-out,
  the fanned cards remain static thumbnails. Parallel loops on
  all fanned cards would compete for attention and pull the user
  away from the "select which perspective" decision that the fan-out
  exists to enable.
- **Each fanned card is independently clickable and focusable.**
  Tab navigation: focus on the frontmost card → ArrowRight steps
  through the fanned cards left-to-right (in fan order, which is
  overlap-descending); ArrowLeft reverses. Tab outside the fan exits
  the fan-set entirely.

**Disambiguation from D4's stack-marker (ADR-0038).**

| Surface           | Image present?     | Metadata strip                       | Trigger origin                                    |
| ----------------- | ------------------ | ------------------------------------ | ------------------------------------------------- |
| D5 stack-card     | Yes (longest int.) | Avatar-dots row (Layer 2)            | Multi-perspective grouping (overlap ≥ 50 %)       |
| D4 stack-marker   | No (text "+K")     | None                                 | Lane-overflow on a single side (> N_max)          |

The user's visual signal: **does the front "slot" show an image?**
If yes → multi-perspective; the image *is* the longest-running
perspective's thumbnail, and the avatar-dots tell you "and others
joined." If no, just a "+K more" plate → lane-overflow; click to see
all overflowed cards as a list.

**Click semantics (ADR-0038 D5; this ADR specifies the hit-targets).**

| Gesture                              | Effect                                                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Click frontmost (idle, no fan-out)   | Open Detail Overlay with multi-perspective list (D5 surface; not specified here).                       |
| Click frontmost (during fan-out)     | **Same as idle.** Frontmost retains its hit-target footprint during fan-out (no shrinking on hover).    |
| Click individual fanned card         | Open Detail Overlay for that single perspective (Single-VOD).                                           |
| Shift-Click frontmost                | Bulk-add **all** perspectives in the group to multiselect (D7 of ADR-0038).                             |
| Shift-Click individual fanned card   | Add **only that perspective** to multiselect.                                                           |
| Cmd/Ctrl-Click {frontmost / fanned}  | OS-conventional alias for Shift-Click (same effects).                                                   |
| Tab → Enter on frontmost             | Same as Click frontmost.                                                                                |
| Tab → Enter on fanned card           | Same as Click that fanned card.                                                                         |

**Hit-target safety.** During fan-out, the visual gap of
`card_width × 0.25` between fanned cards is the **hit-target
boundary** — there is no ambiguous "edge of frontmost vs. start of
fanned" region. The frontmost's bounding rect ends at its visual
edge; the first fanned card's bounding rect starts at the gap edge.
A click in the gap hits neither (no-op).

**Forward references.** Exact silhouette offset, avatar-dot pixel
size, inter-card gap ratio numerics, "+N more" badge styling — all
ADR-VisualSystem-v2.

### DV5 — Retention-Countdown Composition

**Decision.** The Q-7 three-stage escalation is realised as
follows, with the visual surface migrating from metadata strip to
image-overlay at the 48-h threshold so the more-urgent stages live
where attention lands during hover:

| Stage          | Time remaining (X)   | Visual                                                                             | Surface                                |
| -------------- | -------------------- | ---------------------------------------------------------------------------------- | -------------------------------------- |
| 1 (none)       | X > 7 d              | Nothing rendered. Strip Line 2 Column 2 is empty.                                  | —                                      |
| 2 (text)       | 7 d ≥ X > 48 h       | Small neutral text label ("expires in 4d") in strip Line 2 Column 2 (DV2 layout).  | Metadata strip                         |
| 3 (badge, neutral) | 48 h ≥ X > 24 h | Small neutral badge with **clock icon** + tabular-nums text ("48h", "32h").       | Image-area overlay, bottom-right       |
| 4 (badge, accent) | X ≤ 24 h         | Small **accent-coloured** badge with **clock icon** + tabular-nums text ("18h").  | Image-area overlay, bottom-right (same as stage 3) |

**No layout shift between stages 3 and 4.** Both occupy the same
image-overlay slot (bottom-right of the image area, *not* top-right
which is the selection-checkmark, *not* bottom-edge which is the
3-px stripe). Only the badge's *fill colour* changes at the
24-h threshold; size and position are identical. This guarantees a
silent flip when the threshold crosses — no visual reflow.

**Icon choice: clock.** Universally readable at small sizes
(~16-20 px badge height). Alternatives considered:

- **Hourglass:** too ambiguous at small sizes — readers can't
  tell the orientation, and the metaphor "running out" requires more
  cognitive parse than clock.
- **Warning triangle:** too alarming for the neutral stage 3 (where
  the message is "this is getting close, not gone yet"). Reserving
  warning-iconography for actual-error states.
- **Calendar / countdown digit-only:** loses the recognisable
  symbol; a glance must read text rather than icon-then-text.

**Computation source — VOd-A2 (CEO Decisions §below): client-side
compute is the v2.1 binding default.**

The retention countdown is computed in the frontend, from data
already available via `getVod` + `listStreamers`:

```ts
function retentionDays(broadcasterType: string | null): number {
  switch (broadcasterType) {
    case 'partner':   return 60;
    case 'affiliate': return 14;
    default:          return 7;     // non-affiliate / empty
  }
}

const retentionAt   = vod.streamStartedAt + retentionDays(streamer.broadcasterType) * 86_400;
const remainingSec  = retentionAt - nowSec;
const stage         = remainingSec > 7 * 86400 ? 1
                    : remainingSec > 48 * 3600 ? 2
                    : remainingSec > 24 * 3600 ? 3
                    : remainingSec > 0         ? 4
                    : /* already expired */     'expired';
```

**Backend touch deliberately not in v2.1 ADR-VodCard scope.** Adding a
`retention_at` column to the `vods` schema would be an additive
migration but the computation is pure-derivation from existing fields
(per Twitch's public retention tiers). Storing the derived value
would *introduce* a freshness problem (the retention countdown
becomes stale as `nowSec` advances; we'd need to recompute
remaining-seconds anyway). The only thing a `retention_at` column
would buy is robustness against `broadcaster_type` changes between
poll cycles — a real-but-bounded issue (countdown could read 7 d
when the streamer was just promoted to Partner mid-VOD, until next
poll updates `broadcaster_type`). VOd-A2 (CEO Decisions §below)
accepts client-side compute as the v2.1 default and moves the
backend `retention_at` STORED-column addition to the v2.2 backlog
with an explicit trigger: real-use-drift reports. The drift is also
**conservative-falsch** (the card shows *less* retention than
actually remains during a mid-VOD tier promotion), so no user-visible
data-loss risk.

**Update cadence.** Stage transitions happen at hour granularity
(48 h → 24 h is a single threshold). Per-render recompute is enough
— the math is trivial — and the live-tick that drives the
TimelinePage's Now-indicator (ADR-0038 D3: 10 s at Levels 1–2, 30 s
at Levels 3–6) is the same heartbeat that re-evaluates the stage.
**No separate `setInterval`** for retention: piggyback on
TimelinePage's existing tick. On surfaces *without* a tick (e.g.
LibraryPage's Continue-Watching rail), retention recomputes only on
component re-mount or store-triggered re-render — acceptable because
Library is browsed less often per session and the user-visible drift
is at most 30 s.

**Render rules across composite states.**

| Selected? | Stage | Rendering                                                                                               |
| --------- | ----- | ------------------------------------------------------------------------------------------------------- |
| No        | 1     | No retention element.                                                                                   |
| No        | 2     | Strip Line 2 text label.                                                                                |
| No        | 3     | Image-overlay bottom-right neutral badge. Strip Line 2 Column 2 empty.                                  |
| No        | 4     | Image-overlay bottom-right accent badge. Strip Line 2 Column 2 empty.                                   |
| Yes       | 1     | Selection checkmark (top-right). No retention element.                                                  |
| Yes       | 2     | Selection checkmark (top-right) + Strip Line 2 text label. No image-overlay collision.                  |
| Yes       | 3     | Selection checkmark (top-right) + neutral retention badge (bottom-right). Diagonal corners; no overlap. |
| Yes       | 4     | Selection checkmark (top-right) + accent retention badge (bottom-right). Diagonal corners; no overlap.  |

The corner-positions guarantee selected + retention stage-3/4 do
not overlap. **Both badges may carry accent in stage 4** (selection
checkmark is accent per D7; retention badge is accent per Q-7) —
this is two distinct discrete-marker surfaces, not surface-paint.
Total accent coverage on the card stays well under H-06's 5 % budget
(the badges are ≈ 16 × 16 px and ≈ 32 × 20 px; a 240-×-180-px image
area = 43 200 px²; the two badges sum to ≤ ~1 % of the image area).

### DV6 — Continue-Watching Progress Stripe

**Decision.** Q-10's 3-px stripe is rendered at the **bottom edge of
the image area** (*not* the metadata strip; *not* the card's
absolute bottom edge if those are different in a layout variant —
they are not in Full / Compact / Sliver, but the contract is "image
area bottom"). Width is proportional to `watchedFraction`; visibility
gates on watch state.

**Position.** `bottom: 0` of the image area; `left: 0`; `width:
watchedFraction × imageAreaWidth`. Height: ≈ **3 px order of
magnitude** (exact px delegated to ADR-VisualSystem-v2). Z-index:
above the image; below the badges; below the selection checkmark.

**Width math.**

```ts
const visible = watch.state !== 'unwatched'
             && watch.state !== 'manually_watched'
             && watch.state !== 'completed'
             && watch.watchedFraction > 0
             && watch.watchedFraction < 1;
const widthPct = Math.min(100, watch.watchedFraction * 100);
```

**Visibility rules.**

| `watch.state`         | `watchedFraction`     | Stripe rendered?                                |
| --------------------- | --------------------- | ----------------------------------------------- |
| `unwatched`           | 0                     | No                                              |
| `in_progress`         | (0, 1)                | **Yes**                                         |
| `in_progress`         | ≈ 0 (< 0.01)          | No (avoids 0-px sliver during initial-seek)     |
| `completed`           | typically ≥ threshold | No — see VOd-A3 (CEO Decisions §below) on the completed marker |
| `manually_watched`    | usually = 1.0         | No                                              |
| `null` (never opened) | —                     | No                                              |

**Hover behaviour.** The stripe scales with the card (H-17 applies
to the whole image area uniformly; the stripe is a child element and
moves with its parent's transform). No separate stripe-animation;
the stripe is a static-shape element in idle and during hover.

**Selected interaction.** Selection checkmark (top-right of image
area) and the 3-px stripe (bottom edge of image area) are
spatially separated; no collision possible. Both render in their
nominal positions regardless of each other's state.

**Reduced-motion.** Stripe is non-animated; reduced-motion has no
effect on its rendering. (The card's hover-scale is muted under
reduced-motion per DV3, so the stripe stays at its idle scale, but
that's a consequence of the parent's transform suppression, not a
DV6-specific rule.)

**Data source.** `useVodSummary(vodId).watchProgress: WatchProgressRow
| null` — the composite hook from ADR-0038 D8 (formalised in DV8
below). `WatchProgressRow.watchedFraction` is the `STORED` generated
column from ADR-0018; no client-side recomputation needed.

**Completed-state marker — VOd-A3 (CEO Decisions §below): option
(b), neutral checkmark glyph in the strip.**

At 100 % watched (`state === 'completed'` or `'manually_watched'`),
the stripe is hidden. The anchor's Q-10 decision was silent on what
*replaces* it; VOd-A3 binds the v2.1 default to **option (b)**: a
small **neutral checkmark glyph** in the metadata strip
(Line 2 Column 2, right-aligned), reading as "watched" without
consuming accent budget. Alternatives considered (and rejected for
the v2.1 default) in DV6's alternatives section below; VOd-A3
records the sign-off rationale and reserves a Wave-3 thumbnail-
opacity-reduction follow-up if real-use feedback surfaces
background-noise from "every-card-has-checkmark."

### DV7 — State Matrix

**Decision.** The complete state matrix below enumerates every
composition the VodCard must support. Cells reference the DV2 / DV5 /
DV6 base layers; visual particulars (sizes, hex, opacities) are
delegated to ADR-VisualSystem-v2.

**Definitions of axes.**

- **Interactive trigger state:**
  - `idle` — no hover, no focus, no touch.
  - `hover-pre` — pointer inside card hit-area, < 800 ms elapsed.
  - `hover-triggered` — pointer inside card hit-area, ≥ 800 ms
    elapsed (or singleton-transferred).
  - `focus-pre` — Tab-focus on card, < 800 ms elapsed.
  - `focus-triggered` — Tab-focus on card, ≥ 800 ms elapsed.
  - `hover+focus-pre` — both active, neither has reached 800 ms yet
    (the first to reach 800 ms transitions to `triggered`).
  - `triggered` (combined) — at least one of (hover, focus) has
    reached 800 ms.
- **Selection:** `unselected` / `selected`.
- **Motion preference:** `allow` / `reduce` (`prefers-reduced-motion: reduce`).
- **Stack:** `single` / `stack-idle` / `stack-fanned`.
- **Retention stage:** `1` (none) / `2` (text) / `3` (neutral badge) / `4` (accent badge).
- **Watch fraction:** `0` / `(0, 1)` / `1`.
- **Loading state:** `has-thumbnail` / `no-thumbnail-yet`.

**Primary matrix** (Interactive × Selection × Motion). Stack /
Retention / Watch / Loading are *layers* rendered conditionally on
top of each primary state — listed separately to avoid combinatorial
blow-up.

| # | Interactive          | Selection    | Motion  | Image                | Strip            | Outline (H-20) | Scale (H-17) | Frame-strip loop | Expanded panel |
|---|----------------------|--------------|---------|----------------------|------------------|----------------|--------------|------------------|----------------|
| 1 | `idle`               | `unselected` | allow   | Static thumb         | Base strip       | none           | 1.0          | no               | no             |
| 2 | `hover-pre`          | `unselected` | allow   | Static thumb         | Base strip       | none           | 1.0          | no               | no             |
| 3 | `hover-triggered`    | `unselected` | allow   | Frame-strip          | Expanded panel   | none           | H-17         | yes              | yes            |
| 4 | `focus-pre`          | `unselected` | allow   | Static thumb         | Base strip       | **H-20**       | 1.0          | no               | no             |
| 5 | `focus-triggered`    | `unselected` | allow   | Frame-strip          | Expanded panel   | **H-20**       | H-17         | yes              | yes            |
| 6 | `hover-triggered`    | `unselected` | reduce  | Static thumb         | Expanded panel*  | none           | 1.0          | no               | yes (instant)  |
| 7 | `focus-triggered`    | `unselected` | reduce  | Static thumb         | Expanded panel*  | **H-20**       | 1.0          | no               | yes (instant)  |
| 8 | `idle`               | `selected`   | allow   | Static thumb         | Base strip       | **H-20**       | 1.0          | no               | no             |
| 9 | `hover-triggered`    | `selected`   | allow   | Frame-strip          | Expanded panel   | **H-20**       | H-17         | yes              | yes            |
| 10| `focus-triggered`    | `selected`   | allow   | Frame-strip          | Expanded panel   | **H-20**       | H-17         | yes              | yes            |
| 11| `idle`               | `selected`   | reduce  | Static thumb         | Base strip       | **H-20**       | 1.0          | no               | no             |
| 12| `hover-triggered`    | `selected`   | reduce  | Static thumb         | Expanded panel*  | **H-20**       | 1.0          | no               | yes (instant)  |
| 13| `focus-triggered`    | `selected`   | reduce  | Static thumb         | Expanded panel*  | **H-20**       | 1.0          | no               | yes (instant)  |

\* "Expanded panel*" under reduced-motion is the same panel content
as `allow`, but rendered instantly (no slide, no fade > 100 ms).

**Loading-state row.** `no-thumbnail-yet` AND fallback URL also
absent → image area renders a static skeleton; the metadata strip
stays populated from `vod.title` + `streamerDisplayName` + duration.
Selection / hover / focus interactions still apply; the skeleton
replaces the image source without altering the rest of the
composition. Reduced-motion neutral (skeleton is non-animating).

**Stack variant — additional layer on top of #1–#13 when the card
represents a multi-perspective moment:**

| Stack state    | Idle layer                                | Triggered layer                                                       |
| -------------- | ----------------------------------------- | --------------------------------------------------------------------- |
| `stack-idle`   | Silhouette (2 layers) behind card. Avatar-dots row in strip Line 1 Column 1 (replaces single avatar). | (same as idle if not triggered)                                       |
| `stack-fanned` | (transient — only during fan-out animation) | Frontmost card runs frame-strip loop. Fanned cards lateral right (or auto-mirror left at viewport edge — VOd-A5). Each fanned card is independently focusable / clickable. |

**Selection × stack.** Shift-Click on the frontmost selects *all*
perspectives in the group; the selection-checkmark badge then renders
on the frontmost (per D7) — fanned cards inherit "selected" state via
the group-multiselect but their individual badges render only when
the fan is open (rows #8–#13 apply on a per-fanned-card basis once
fan-out is active).

**Retention × stack.** Stack-card renders **only the longest-running
interval's retention stage** in the image overlay. Individual fanned
perspectives may have different retention windows, but they share the
same wall-clock origin — within an overlap-of-≥-50 %-shorter-interval
group, retention spans are within ~few hours of each other.
ADR-VisualSystem-v2 may revisit if user testing surfaces
per-perspective retention mismatch as a confusion source.

**Watch fraction × stack.** Stripe renders on the frontmost
according to the frontmost interval's `watchProgress`. Individual
fanned cards' own watch-progress would only become visible during
fan-out (each fanned card is a full DV2-composition once fanned), so
stripes can appear differently per fanned card during fan-out — this
is intentional, lets the user see "which perspectives I've already
watched."

**No state is unreachable.** Every cell in the matrix is a
reachable composition; the renderer does not need defensive guards
for "impossible combinations."

### DV8 — Data-Layer Touchpoints

**Decision.** The VodCard renders entirely from data exposed by
existing IPC commands plus the new composite hook `useVodSummary`
defined in ADR-0038 D8. **No new IPC endpoint is added by this ADR.**
Backend retention-source addition is deferred to v2.2 per VOd-A2
(CEO Decisions §below) with a real-use-drift trigger; the
composite-batch endpoint is deferred to v2.2 per ADR-0038 D8 with a
profiling-driven trigger (200 ms first-paint target reach); GIF-strip
generation reuses the existing ingest pipeline (DV10; the Anti-Smell
audit confirmed this is engineering-default-by-existing-infrastructure,
no OQ surface).

**Composite hook signature** (new file
`src/features/timeline/use-vod-summary.ts`, also used by LibraryPage's
v2.1 VodCard mount):

```ts
export type VodSummary = {
  vod: Vod;                              // from getVod
  chapters: Chapter[];                   // from getVod (not rendered by card; reserved)
  streamer: {                            // from listStreamers (cached cross-card)
    displayName: string;
    profileImageUrl: string | null;
    broadcasterType: string;
    favorite: boolean;
  };
  assets: VodAssets;                     // from getVodAssets
  watchProgress: WatchProgressRow | null; // from getWatchProgress
  coStreams: CoStream[];                 // from getCoStreams, client-filtered ≥ 50 % overlap (D5)
};

export function useVodSummary(vodId: string): UseQueryResult<VodSummary>;
```

**Field-to-touchpoint mapping.**

| DV2 / DV5 / DV6 field                | Source                                                                  |
| ------------------------------------ | ----------------------------------------------------------------------- |
| Image — `thumbnailPath`              | `assets.thumbnailPath` (local; `convertFileSrc` for asset protocol)     |
| Image — remote fallback              | `vod.thumbnailUrl` (Twitch CDN; pre-Phase-3 row, no local cached)       |
| Image — frame-strip (hover preview)  | `assets.previewFramePaths[0..5]`                                        |
| Strip — streamer avatar              | `streamer.profileImageUrl`                                              |
| Strip — streamer display name        | `streamer.displayName`                                                  |
| Strip — title                        | `vod.title`                                                             |
| Strip — duration                     | `vod.durationSeconds` (formatted via `formatDurationSeconds`)           |
| Strip — retention text (stage 2)     | computed from `streamer.broadcasterType` + `vod.streamStartedAt` (DV5)  |
| Image-overlay — retention badge      | same as above                                                           |
| Image-overlay — selection checkmark  | `timeline-multiselect-store.selectedVodIds.has(vod.twitchVideoId)` (D7) |
| Image-overlay bottom — 3-px stripe   | `watchProgress.watchedFraction` (gated by `watchProgress.state`)        |
| Stack — avatar-dots row              | `coStreams.map(cs => cs.streamer)` (filtered ≥ 50 % per D5; max 4 + "+N") |
| Stack — silhouette layer count       | derived: `min(2, coStreams.length)`                                     |
| Stack — fan-out cards                | `coStreams` ordered by overlap-length descending (max 4)                |

**TanStack-Query keys (TS pseudo-types).**

```ts
const keys = {
  vod:        (id: string) => ['vod', 'detail', id]      as const,  // getVod
  vodAssets:  (id: string) => ['vod', 'assets', id]      as const,  // getVodAssets
  watch:      (id: string) => ['watch', 'progress', id]  as const,  // getWatchProgress
  coStreams:  (id: string) => ['timeline', 'co', id]     as const,  // getCoStreams
  streamers:  ()           => ['streamers', 'list']      as const,  // listStreamers (shared)
};
```

**Cache invalidation (composes with ADR-0038 D8's table).** No new
event topics; the v2.1 events `vod:updated`, `watch:progress_updated`,
`watch:state_changed`, `watch:completed` invalidate the existing keys
the card subscribes to. Per-card re-render triggers:

| Event                       | Card-impacting cache invalidation                          |
| --------------------------- | ---------------------------------------------------------- |
| `vod:updated`               | `keys.vod(id)`, downstream `useVodSummary(id)`             |
| `watch:progress_updated`    | `keys.watch(id)` only — no card re-pack                    |
| `watch:state_changed`       | `keys.watch(id)` — stripe visibility may flip              |
| `watch:completed`           | `keys.watch(id)` — stripe disappears, watched-glyph (VOd-A3) appears |

`vod:ingested` does not target individual cards (an existing card
mount won't represent a fresh VOD; the parent TimelineLanes /
LibraryGrid handles list-level invalidations).

**GIF-strip asset format and pipeline.**

- Asset format: **6 PNG frames** at fixed `PREVIEW_FRAME_PERCENTS`
  (10 %, 30 %, 50 %, 70 %, 90 %, 95 % — per
  `services/downloads.rs:978`). Already serves both LibraryPage and
  TimelinePage uniformly.
- Pipeline: **pre-ingest** — frames are extracted by ffmpeg during
  the VOD download (`services/downloads.rs:985
  extract_preview_frames`) and by the `backfill_preview_frames`
  background task (`services/media_assets.rs:173`) for any
  pre-Phase-5 VODs without frames.
- Serving: via the existing Tauri asset protocol with the
  `library_root` scope from ADR-0027 — paths returned by
  `getVodAssets()` are absolute, pass `convertFileSrc` to get the
  webview-loadable URL.
- **No new IPC endpoint.** A `cmd_get_vod_preview_strip` variant
  (e.g., to return an animated WebP) is **not** added in this ADR.
  PNG-rotation is the proven path (already shipping in v2.0.x
  Library VodCard); WebP migration is a Wave-3+ Engineering item
  evaluated only if PNG paint thrashes (DV10).

**Retention source backend-touch.** Deferred to v2.2 per VOd-A2's
real-use-drift trigger. The client-side computation in DV5 uses
`streamer.broadcasterType` + `vod.streamStartedAt`, both already in
`useVodSummary`.

**Composite-batch endpoint.** Deferred to v2.2 per ADR-0038 D8
precedent (engineering default — Anti-Smell audit confirmed
profiling-driven trigger is the right gate, no OQ surface).
TanStack-Query's cross-component cache deduplication is the v2.1
mitigation:

- On cold load of TimelinePage with N cards in the render-buffer,
  `useVodSummary` fires up to N × {`getVod`, `getVodAssets`,
  `getWatchProgress`, `getCoStreams`, `listStreamers`}.
- `listStreamers` is keyed once (`keys.streamers()`), so it's a
  single fetch regardless of N.
- `getVod`, `getVodAssets`, `getWatchProgress`, `getCoStreams` are
  per-id, so they are N parallel fetches.
- For N = 100 simultaneous mounts, that's ~400 IPC calls in a
  single tick. The Tauri IPC bridge handles ~1-2 k calls/s on
  consumer hardware; perceptible but not catastrophic at N = 100.
- The batch endpoint becomes preemptively useful only when the
  render-buffer routinely contains > 200 cards, which would imply
  the user is at Zoom Level 1 with 30-day extended past data —
  outside the typical envelope.
- Profiling trigger: if first-paint at the v2.1 release exceeds
  the ADR-0038 D6 target of 200 ms at N ≥ 100, the batch endpoint
  lands in v2.2 as `cmd_get_timeline_vod_summary({ vodIds: string[] })`
  returning `Vec<VodSummary>`. Shape compatible with the composite
  hook's return type — drop-in replacement at the fetcher boundary.

### DV9 — Accessibility and Keyboard Behaviour

**Decision.** The VodCard is a focusable interactive element with
explicit ARIA semantics, deterministic tab-order through the
TimelineLanes, and screen-reader announcements that cover the visual
composition without depending on visual cues.

**Tab order (within TimelineLanes).**

- **Lane order:** above-axis lanes in outermost-to-innermost order
  (top to centre), then below-axis lanes in innermost-to-outermost
  order (centre to bottom). This matches the visual reading order
  for the time-axis layout: the eye starts at the top, sweeps to the
  axis, then continues downward.
- **Within a lane:** chronological order by `start_at` ascending —
  left to right, matching the spatial layout.
- **Between cards across lanes:** tab does not jump back and forth
  in time; it completes one lane top-to-bottom before moving to the
  next, in chronological order within each. This gives a stable
  mental model: "Tab walks each lane in time order; lanes are
  traversed top-to-bottom."
- **Stack-card focus:** Tab visits the **frontmost** only. Fanned
  cards are reached via ArrowRight / ArrowLeft *once the fan-out
  has triggered* (per DV4 Tab rules) — they are *not* part of the
  primary Tab order, because adding them would re-shuffle the Tab
  order when stack-grouping changes between renders, which would be
  cognitively jarring.

**Card ARIA attributes.**

```html
<article
  role="article"
  aria-label="<screen-reader-label>"
  aria-selected="{selected ? 'true' : 'false'}"
  tabindex="0"
  data-vod-id="{vodId}"
  data-stack="{single | stack-idle | stack-fanned}"
>
  ...
</article>
```

**Screen-reader label format** (Single-VOD):

```
"<title>, <streamer_display_name>, <duration_human>, <retention_phrase>[, <watch_phrase>][, <selected_phrase>]"
```

**Examples.**

- Idle, no selection, no retention, no watch: `"Wolfsterben
  Rampage, John Smith, 4 hours 12 minutes"`
- Retention stage 2: `"Wolfsterben Rampage, John Smith, 4 hours 12
  minutes, expires in 4 days"`
- Retention stage 4 + selected + 45 % watched: `"Wolfsterben
  Rampage, John Smith, 4 hours 12 minutes, expires in 18 hours,
  watched 45 percent, selected"`

**Stack-card label**:

```
"<title>, <perspective_count> perspectives from <streamer_names_joined>, <duration_of_longest>, <retention_phrase>"
```

Example: `"Wolfsterben Rampage, 3 perspectives from John, Jane,
Alex, 4 hours 12 minutes, expires in 4 days"`

**Selection announcement.** The TimelineSelectionBar (ADR-0038 D7)
already hosts a polite live region announcing "N selected" on
selection-set changes. The per-card `aria-selected="true"` attribute
is the secondary signal; screen-readers read it on focus. **No
per-card `aria-live`** — that would over-announce on every
selection toggle.

**Frame-strip cascade announcement (CEO-A4).** The frame-strip loop
plays on Tab-focus after 800 ms, identical to mouse-hover. The
**screen-reader announcement does not change** when the loop
starts — the loop is a visual animation, not a semantic state
change. The expanded-metadata panel that appears on trigger does
*not* duplicate the existing aria-label; instead, the panel's
content (action buttons like "Play", "Add to Multiview"; tag list)
gets its own role / aria semantics inside the panel. The Tab order
inside the expanded panel is documented in DV9-companion (Wave 4
engineering, not specified here).

**Focus on a stack-card (cascade detail).**

- Tab arrives on frontmost → H-20 outline immediate → 800 ms timer
  starts.
- At 800 ms: loop fires + fan-out animates open.
- ArrowRight: focus moves to first fanned card; the first fanned
  card's frame-strip loop fires (the singleton transfers — per
  DV3); the frontmost reverts to idle thumbnail (but stays
  on-screen as the "anchor" of the fan group). H-20 outline
  follows focus.
- ArrowLeft from fanned card 1: focus returns to frontmost; loop
  fires there.
- Tab from any fanned card: focus exits the fan group entirely;
  the fan collapses; the loop ends; next focusable element gets
  H-20.

**ARIA-expanded for the fan-out state.** The frontmost card carries
`aria-expanded="{fanned ? 'true' : 'false'}"` when it represents a
stack. Screen-readers announce expansion-state on the trigger
moment.

**Keyboard shortcuts (this ADR locks the contract; rendering in
Wave 4).**

| Key                                | Effect                                                                          |
| ---------------------------------- | ------------------------------------------------------------------------------- |
| Tab / Shift+Tab                    | Move card-focus along the chronological-within-lane / lane-by-lane order.       |
| Enter or Space (focus on card)     | Equivalent to Click — open Detail Overlay (or, on stack-card, the multi-perspective Detail Overlay). |
| Shift+Enter or Shift+Space         | Equivalent to Shift+Click — toggle selection.                                   |
| Esc                                | Clear selection (delegated to TimelineSelectionBar; D7 of ADR-0038).            |
| ArrowRight / ArrowLeft on stack-card frontmost (during fan-out) | Navigate within the fanned set.                                                 |

**Touch / pointer.** Touch is interpreted as a click; long-press
does not open the fan-out (DV4 affordance), it falls through to the
default click action (Detail Overlay). Long-press as a fan-out
gesture is deferred to v2.2 if user research surfaces a
touch-affordance need.

**Color-blindness / contrast.** All accent uses on the card are
**discrete marker surfaces** (selection checkmark, retention stage-4
badge). Both must satisfy 4.5:1 contrast against the underlying
image (badges have a neutral backdrop scrim per the
H-08 surface ladder); ADR-VisualSystem-v2 pins the contrast values.
The 3-px stripe carries information by *length* (proportion of card
width); colour-blind users perceive it equivalently to colour-sighted
users.

### DV10 — Performance Architecture

**Decision.** The VodCard is `React.memo`-boundary-isolated; hover state
is local; frame-strip frames are eager-mounted but `display: none`
until hover trigger (browser-cache handles the bytes); a singleton
guard limits in-flight frame-strip loops to **one card per page
mount**; the existing pre-ingest pipeline (no new ffmpeg invocation
in this ADR) provides the asset bytes.

**Memoization boundaries.**

- **`VodCard` component**: `React.memo` with a custom equality function
  that compares `(vodId, isSelected, vodSummary)`. `vodSummary` is
  the TanStack-Query result object — its identity is stable across
  renders if the underlying cache entry is unchanged.
- **`isSelected`**: subscribed via Zustand selector
  `useTimelineMultiselectStore((s) => s.selectedVodIds.has(vodId))`.
  Each card re-renders only when *its own* selection state flips —
  not when *any* card's selection flips.
- **`isHovered`**: local `useState` in the VodCard. Lifting it to a
  store would cause unnecessary re-renders of unrelated cards (every
  card subscribed to "the hover store" would re-render on every
  hover-source change). Lokaler State is the correct boundary.

**Frame-strip loading.**

- The 6 frames are referenced via `<img src={previewFramePaths[i]} />`
  elements mounted in the DOM but `style="display: none"` until the
  hover-preview trigger fires.
- The browser pre-fetches the bytes when the first such `<img>` is
  parsed in the DOM (the asset-protocol fetch happens lazily but
  early — typically within ~50 ms of the card mounting).
- On trigger, the `display: none` is replaced by `opacity: {0..1}`
  cycled to the current frame index; the bytes are already cached.
- **Lazy strategy considered, rejected**: gating frame mounts on
  hover-trigger would mean the first cycle of frames sees the
  browser fetching mid-loop, causing visible blank frames. The
  6 × ~30 KB per VOD is negligible network cost; eager-mount with
  CSS-hidden render is the cleaner choice.
- **IntersectionObserver-based strategy considered, rejected**:
  the render-buffer (ADR-0038 D6) already does the same job at a
  coarser granularity — a card outside the buffer doesn't mount at
  all.

**Singleton-preview invariant (DV3 mentioned, here formalised).**

- Implementation: a module-scoped ref in `TimelineLanes.tsx` (or
  equivalent parent) tracks the currently-looping `vodId`.
- On trigger fire: write `vodId` to the ref; emit a "stop" event to
  the previously-tracked card (if any), which immediately reverts
  its frame-strip to opacity 0 and stops its 400-ms interval.
- On trigger end: clear the ref (only if it still matches this card —
  guard against race with concurrent trigger transitions).
- **One active loop per page-mount.** Across surfaces (TimelinePage
  vs. LibraryPage), the singleton is per-mount, so a Multiview
  preview-thumbnail in another open window does not interfere.

**Frame cycle cadence.** 400 ms per frame; 6 frames; 2.4 s loop. Matches
the existing VodCard.tsx precedent — no new pacing decision. If
Wave-3 perceptual testing surfaces a snappier preferred cadence
(e.g., 300 ms), the constant lives in
`src/features/timeline/use-vod-preview.ts` and is a single-line
change.

**`prefers-reduced-motion`.** The 6 frame `<img>` elements remain
mounted (the cost is paid; revoking it would change re-mount cost
on motion-preference toggle), but the cycle interval is never
started; the expanded panel renders instantly per DV3.

**N+1 round-trip mitigation (DV8 cross-reference).**

- Cold-load worst case: 100 cards × 4 per-id calls = 400 IPC calls.
- TanStack-Query's automatic deduplication: within a single render
  pass, identical keys fire once. Across cards, distinct `vodId`s
  produce distinct keys; no dedup at the per-vod level.
- The `listStreamers` key is shared cross-card (a single fetch).
- Profiling target: TimelinePage first-paint at N = 100 cards
  measured ≤ 200 ms (ADR-0038 D6 target). If exceeded, lift to
  v2.2 with the batch endpoint per ADR-0038 D8.

**Render-buffer composability (ADR-0038 D6).** The render-buffer
keeps ~viewport-width × 2 of cards mounted. A user-pan that scrolls
the buffer past a card unmounts it cleanly — `<img>` references go
out of scope, browser cache holds the bytes, re-mount on scroll-back
is fast (the bytes are warm).

**Eager vs. lazy chamber for the static thumbnail.** Always eager
(`<img loading="lazy">` on the static thumbnail — the native lazy-load
attribute is the right pacing, defers fetch until card enters
viewport, no React-level wiring needed). The 6 frame-strip `<img>`s
do not get `loading="lazy"` because we want them pre-cached for the
hover trigger.

**Storage cost (cross-reference).** Pre-ingest pipeline produces 6
frames × ~30 KB average × N VODs in library. At N = 1 000 VODs:
~180 MB. Already accounted for in Storage Forecast (ADR-0032). No
new storage budget needed.

**WebP migration path (out of v2.1 scope).** If PNG paint thrashes
or storage footprint becomes a concern, a follow-up Engineering wave
can: (1) add a `services/downloads.rs` flag to extract WebP frames
in addition to (or instead of) PNG; (2) update `useVodSummary` to
prefer WebP paths when present; (3) keep PNG fallback for back-compat
during the migration. None of this affects the ADR contract; the
composite hook's surface is format-agnostic.

---

## Consequences

### Positive

1. The §3.2 / §3.3 / §6.2 anchor lands as runnable composition
   without re-litigating Q-3 / Q-5 / Q-7 / Q-10 / CEO-A4.
2. **No new IPC endpoint.** The existing `getVodAssets`,
   `getVod`, `getWatchProgress`, `getCoStreams`, `listStreamers`
   commands carry the v2.1 VodCard end-to-end. Migration risk is
   zero on backend, IPC, and data-model.
3. **No new asset pipeline.** The 6-PNG-frame pre-ingest pipeline
   already deployed in v2.0.x carries the Q-3 GIF-strip surface
   directly; no new ffmpeg flags, no new sidecar jobs, no new
   storage budget needed.
4. **No new event topics.** The existing `vod:updated`,
   `watch:progress_updated`, `watch:state_changed`,
   `watch:completed` events cover all per-card cache-invalidation
   needs.
5. The state matrix (DV7) is exhaustive — every reachable
   composition is enumerated. Wave-4 engineering can implement
   directly against the table; component snapshot tests pin every
   row.
6. The Tab-order (DV9) gives a stable mental model: lane-by-lane,
   chronological-within-lane. Keyboard users get the same temporal
   navigation as visual users.
7. **No backend touch.** Retention computation is client-side;
   composite-batch endpoint deferred to v2.2; GIF-pipeline reused.
   Matches the ADR-0038 D8 no-backend-touch policy.
8. Existing `VodCard.tsx` from ADR-0033 is replaced (Wave-4 work);
   no two-variant codebase after replacement. Anchor-violations in
   the v2.0.1 component (accent border, accent focus outline,
   accent-fill ✓-badge, 1-px stripe) are corrected in the rewrite.

### Costs accepted

1. **Per-render retention recompute.** Cheap (a single subtraction),
   but every card re-evaluates retention stage on each parent
   re-render. With memoization (DV10) this is the equivalent of one
   floating-point op per card per parent tick. Acceptable.
2. **Backend retention drift between poll cycles.** A streamer
   promoted from Affiliate to Partner mid-VOD shows the old
   retention (14 d → 60 d) until the next poll cycle refreshes
   `broadcaster_type`. Bounded by poll interval (10–120 min). Per
   VOd-A2 (CEO Decisions §below), the drift is also
   conservative-falsch (countdown shows *less* time than actually
   remains; no data-loss risk) — v2.2 backend `retention_at`
   addition is gated on real-use-drift reports.
3. **N+1 IPC at cold load with high N.** ~400 IPC calls at N = 100
   cards. Within budget per ADR-0038 D6. Batch endpoint deferred to
   v2.2 (VOd-Q4) on profiling-driven trigger.
4. **Stack fan-out edge-mirror is a single-direction choice.**
   Auto-mirror to lateral left at right-viewport-edge is the
   binding default per VOd-A5 (CEO Decisions §below). The mirrored
   reading direction violates the user's "right cards open right"
   expectation for the cards close to the right edge; the trade-off
   is accepted because auto-mirror is the standard smart-direction
   pattern (dropdowns, tooltips) and users learn it in a single
   interaction. Vertical and suppress alternatives stay documented
   as rejected for reference.
5. **Existing VodCard.tsx rewrite in Wave 4.** The v2.0.1 component
   gets replaced; its tests will need to migrate to the new
   component's interface. Mitigation: keep the high-level "card
   renders title / thumb / progress" assertions identical; only the
   internal composition changes.
6. **The 800-ms delay constant.** Picked as midpoint of H-18
   0.6–1.0 s range. Wave-3 perceptual testing may want to revisit;
   the value is a single-line constant change in
   `use-hover-preview.ts`.

### Neutral (forward references)

1. **Exact pixel values** — card-corner-radius (H-13 range 2–6 px),
   selection-checkmark size (~16 px), retention-badge size,
   streamer-avatar dimensions, stripe height (~3 px order of
   magnitude), focus-outline width (~2 px order of magnitude),
   silhouette offset (~2 px), inter-card-gap-in-fan-out
   (~card_width × 0.25), hover-zoom factor (1.08–1.5×) → **ADR-VisualSystem-v2** (Wave 3).
2. **Exact hex / opacity** — accent for selection checkmark, accent
   for retention stage-4 badge, accent for 3-px stripe, neutral
   white opacity for H-20 outline, surface tints for skeleton →
   **ADR-VisualSystem-v2**.
3. **Card-width-variant thresholds** — `card-readable-width-px`
   (Full ↔ Compact) and `card-image-only-width-px` (Compact ↔
   Sliver) → **ADR-VisualSystem-v2**.
4. **Hero-slot variant composition** — when the VodCard is rendered
   in the ADR-0038 D9 hero-slot, the variant is "Hero". Composition
   follows §3.1 Hero Row plus Q-2 conditional logic. **Out of scope
   for this ADR**; lands in Wave 3 / Wave 4 alongside
   `TimelineHeroSlot.tsx`.
5. **Expanded panel contents under hover-preview.** The inline
   expanded-metadata that appears at trigger (1–2 line summary, tag
   row, action buttons) follows §3.3's pattern; specific button
   wiring (Play / Add-to-Multiview / Like) per surface lands in
   Wave 4 engineering. This ADR commits to "the panel exists, it
   contains action buttons, it composes with H-20 / H-22"; not to
   the specific button set.
6. **GIF-format WebP migration** — an Engineering Wave after
   Wave 4, only if PNG-cycle proves problematic. Format-agnostic
   composite hook surface keeps the migration cheap.
7. **Batch endpoint `cmd_get_timeline_vod_summary`** — v2.2 work,
   gated by ADR-0038 D6 first-paint-200ms profiling-trigger.
   Drop-in compatible with the composite hook's return shape.
8. **Backend `retention_at` field** — v2.2 work, gated by VOd-A2's
   real-use-drift trigger.
9. **Detail Overlay composition** — opened by Click on a card (D5 of
   ADR-0038); composition follows §3.4. Out of scope for this ADR;
   the contract is "Click opens Detail Overlay; ADR-VisualSystem-v2
   pins the overlay envelope" (per ADR-0038 risk #3).

### Risks

1. **Frame-strip cycle perceived as too fast / too slow.** 400 ms /
   frame is the v2.0.x Library precedent. Wave-3 perceptual testing
   may surface 300 ms or 500 ms as the preferred cadence. Mitigation:
   single-constant change; not an ADR re-open.
2. **Singleton race at cascade boundary.** Mouse-hover and Tab-focus
   on different cards in rapid succession could leak an active loop
   if the cleanup race is bug-prone. Mitigation: explicit "stop
   event" emission to the previous holder, plus a defensive
   no-op-if-stale guard in each card's loop interval.
3. **Stack fan-out collision with neighbouring cards in dense lanes.**
   Fanned cards extend laterally; if the next card on the same lane
   is < 4 card-widths to the right, the fan can occlude it. Q-5
   anchor decision accepted occlusion-on-hover as the trade-off for
   "cards become individually clickable." Mitigation: ArrowRight
   focus-cascade through fanned cards is unaffected by occlusion;
   the user can still navigate. Visual-System-v2 may revisit
   z-order layering of the fan-out region.
4. **Reduced-motion expanded-panel feel.** Showing the expanded
   panel "instantly" on hover-trigger may feel jarring (no
   animation). Mitigation: the optional ≤ 100 ms opacity-only fade
   stays available per DV3.
5. **Browser asset-protocol cache eviction.** A long session with
   thousands of cards mounting / unmounting could evict frame-strip
   bytes from the browser cache, re-triggering fetches on re-mount.
   Mitigation: the cards are bounded by the render-buffer (ADR-0038
   D6); typical session sees the same ~200 VODs cycled through, well
   within asset-protocol cache headroom.
6. **VOd-A3 default ("watched" glyph in strip)** may turn out to
   read as background noise — every completed VOD shows a small
   checkmark in its strip. **Mitigation (Wave-3 reserved path):**
   ADR-VisualSystem-v2 (Wave 3) may evaluate an optional
   thumbnail-opacity reduction (~75–80 %) as a visual-token
   reinforcement of the "watched" signal if real-use feedback
   surfaces the background-noise pattern. Implementation would be
   a single-line CSS change in the visual-token layer (no ADR
   re-litigation required); the VOd-A3 glyph stays as the primary
   marker.

---

## Alternatives considered

### DV1 (Card-geometry)

- **Single fixed card width, no Sliver / Compact variants** —
  rejected. Cards at Level 5–6 (week / month zoom) would be 6 px
  wide regardless and unreadable. The three-variant ladder is the
  only way to honestly render across all zoom levels.
- **Metadata as overlay scrim on the image (not below)** —
  rejected. §3.2 allows it, but at Sightline's expected card sizes
  (smaller than Netflix's), the scrim reduces image legibility.
  Below-image is the safer choice.
- **Square cards (1:1) for browse density** — rejected. H-16 +
  Twitch native 16:9 thumbnails are binding; square cards would
  letterbox or crop, both anchor-noncompliant.

### DV2 (Idle composition)

- **Avatar + name on a separate "header" strip above the image** —
  rejected. Anchor §3.2 places metadata strictly *below* the image
  area. Header strip would compete with the conditional hero-slot
  visually.
- **Title overlaid on the image bottom with gradient scrim** —
  considered (§3.2 allows it). Rejected for the Full variant
  because at Sightline's card sizes the title scrim eats more
  image area than the below-strip composition. Visual-System-v2 may
  re-evaluate for the Compact variant.
- **Single-line strip combining streamer + title** — used in the
  Compact variant; rejected as the Full default because the
  duration / retention line is part of the standard browse
  hierarchy.
- **Generic placeholder image for loading state (streamer profile
  pic)** — rejected. Mis-signals which VOD is loading when multiple
  same-streamer VODs mount; neutral skeleton is unambiguous.

### DV3 (Hover sequence)

- **No delay (immediate hover preview)** — rejected. H-18 binding;
  immediate preview is touch-trigger, anchor-prohibited.
- **Longer delay (1.5 s)** — rejected. Outside the H-18 range and
  feels sluggish on intentional hovers.
- **Click-to-toggle preview instead of hover-sustained** — rejected.
  Conflicts with click=open-Detail-Overlay default (D5 of
  ADR-0038). Two-finger / right-click reserved for context-menu
  affordances v2.2+.
- **Auto-play on focus regardless of duration** — rejected per
  CEO-A4 cascade.
- **Different keyboard delay than mouse delay** — rejected per
  CEO-A4 cascade; the keyboard trigger replicates the mouse trigger.
- **Mute toggle on hover (anchor H-19)** — N/A because Q-3 cascade;
  no audio in the frame-strip.

### DV4 (Stack-card)

- **Numeric corner badge "+N perspectives" instead of stack
  silhouette** — rejected. Q-5 anchor binding: stack silhouette
  + avatar-dots, no corner badges.
- **Tooltip on stack-card** — rejected. Q-5 anchor binding: no
  tooltips. Fan-out is the affordance.
- **Vertical fan-out** — rejected. Would collide with neighbouring
  lanes above / below the time-axis; corrupts ADR-0038 D4 lane
  packing.
- **Auto-suppress fan-out at edge of viewport (click-only)** —
  rejected per VOd-A5. Denies the affordance Q-5 specifies; users
  at the right edge of the viewport would behaviourally diverge
  from users in the middle.
- **All fanned cards loop simultaneously** — rejected. Competes
  for attention; defeats the "select a perspective" affordance.
- **Avatar-dots ordered alphabetically** — rejected. Loses the
  "most-overlapping first" signal; the overlap-descending order
  surfaces the tightest co-perspective for quick scan.
- **Stack-silhouette = 3 visible layers behind** — rejected. Adds
  visual noise without information gain; 2 layers reads as "depth"
  enough.

### DV5 (Retention)

- **Always-visible retention countdown** — rejected. Q-7 anchor
  binding: nothing > 7 d.
- **Continuous-fade from neutral to accent** — rejected. Anchor
  H-07 ("accent restraint") prefers discrete escalation steps over
  gradient surfaces.
- **Retention badge at top-left of image area** — rejected. Top-left
  is reserved by anchor convention for content-type badges (e.g.,
  "Live" indicators); Sightline could host that there later.
- **Retention as a numeric digit-only "4d" or "18h"** — adopted
  inside the badge (alongside the clock icon) — see DV5 table.
- **Hourglass icon** — rejected. Ambiguous at small sizes.
- **Warning-triangle icon** — rejected. Too alarming for stage 3.
- **Update via dedicated 1-min `setInterval`** — rejected.
  Piggyback on TimelinePage's Now-indicator tick avoids duplication.

### DV6 (Stripe)

- **6-px stripe** — rejected. Anchor §3.6 + Q-10 anchor binding:
  3-px order of magnitude. 6 would dominate the image bottom edge.
- **Stripe in white instead of accent** — rejected. Q-10 anchor
  binding: 3-px stripe in `#d4a14a`.
- **Stripe at the card's absolute bottom (under the metadata strip)**
  — rejected. Anchor binding: stripe at image-area bottom, not the
  card's outer bottom.
- **Hide stripe at < 1 % progress** — adopted. Avoids the 0-px
  sliver during initial seek.
- **Show stripe at 100 % as a full-width bar** — rejected. The
  full-width bar carries the same information as the stripe's
  absence ("watched all"); rendering both is redundant. The
  completed-state marker is the neutral checkmark glyph per VOd-A3.

### DV7 (State matrix)

- **Single "hover" state without pre/triggered distinction** —
  rejected. The 800-ms delay is a binding part of the state
  transition; the matrix has to enumerate the pre-trigger state.
- **Separate matrix rows per retention stage** — rejected. Retention
  is an orthogonal *layer* on top of the primary states; merging
  produces a 13-row × 4-stage = 52-row matrix without information
  gain.

### DV8 (Data)

- **New `cmd_get_vod_preview_strip(vodId)` endpoint returning
  animated WebP** — rejected for v2.1. The PNG-frame pre-ingest
  pipeline already produces the bytes; a new endpoint would
  duplicate the asset and require a backend touch.
- **Composite-batch `cmd_get_timeline_vod_summary({ vodIds })`** —
  deferred to v2.2 per ADR-0038 D8 precedent. Drop-in compatible
  with the composite-hook signature.
- **Backend `retention_at` STORED column on `vods`** — deferred to
  v2.2 per VOd-A2's real-use-drift trigger; client-side compute is
  the v2.1 binding default.
- **`useVodSummary` as separate hooks combined at render time** —
  rejected (would be `useVod` + `useVodAssets` + `useWatchProgress`
  + ...). Composite hook is cleaner for the consumer and equivalent
  for the cache (TanStack-Query stores each underlying key
  separately regardless of composition).

### DV9 (Accessibility)

- **Time-interleaved Tab order across lanes** — rejected.
  Cognitively jarring (user's focus jumps spatially between top and
  bottom lanes within a single tab-press sequence). Lane-by-lane
  with chronological-within preserves spatial reading.
- **Stack-card frontmost + fanned cards in primary Tab order** —
  rejected. Would re-shuffle Tab order when stack-grouping changes;
  ArrowRight cascade inside the fan is the safer model.
- **Per-card aria-live for selection changes** — rejected.
  Over-announces; the toolbar's "N selected" live region carries
  the global signal.

### DV10 (Performance)

- **Lazy-load frame-strip `<img>` elements on hover-trigger only** —
  rejected. First cycle shows blank frames during fetch; eager-mount
  with CSS-hidden render is cleaner.
- **Hover state in a Zustand store** — rejected. Cross-card
  re-renders when no other card cares about the hover state.
  Local `useState` is correct.
- **Use Web Workers for frame-strip cycle timing** — rejected.
  `setInterval` at 400 ms is well within the main-thread budget;
  Web Workers add complexity without measurable benefit at this
  cadence.
- **Animated WebP instead of PNG-rotation** — out of v2.1 scope.
  Format-agnostic composite hook keeps the migration cheap.
- **Canvas-based frame compositing** — rejected. Accessibility
  cost (Canvas needs parallel DOM scaffolding for screen-readers);
  same reasoning as ADR-0038 D6.

---

## Implementation Notes

### Component tree (one level of detail)

```
VodCard                              (new — replaces v2.0.1 VodCard.tsx)
├── VodCardImage                     (16:9 image area, all overlays)
│   ├── ThumbnailLayer               (static thumbnail; remote / local / skeleton)
│   ├── FrameStripLayer              (6 <img>, opacity-cycled)
│   ├── RetentionBadge               (stage 3 / 4 — image-overlay variant)
│   ├── SelectionCheckmark           (D7 — top-right when selected)
│   └── ProgressStripe               (3-px bottom edge)
├── VodCardStrip                     (metadata strip; Full / Compact)
│   ├── StreamerAvatar               (single OR avatar-dots row for stack)
│   ├── TitleAndStreamer             (Line 1: name · title with ellipsis)
│   └── DurationAndRetentionText     (Line 2: duration · stage-2 text)
└── VodCardStackOverlay              (silhouette + fan-out region; conditional)
```

Stack-card fan-out is handled by a separate `VodCardStackOverlay`
that wraps the base `VodCard` when `coStreams.length > 0`. The
overlay renders sibling cards laterally during the triggered state.

### New files

- `src/features/vods/VodCard.tsx`              — v2.1 successor (rewritten)
- `src/features/vods/VodCardImage.tsx`         — image area + overlays
- `src/features/vods/VodCardStrip.tsx`         — metadata strip
- `src/features/vods/VodCardStackOverlay.tsx`  — silhouette + fan-out
- `src/features/vods/use-vod-summary.ts`       — composite hook
- `src/features/vods/use-hover-preview.ts`     — 800-ms delay + cascade state
- `src/features/vods/retention.ts`             — pure compute function
- `src/features/vods/VodCard.test.tsx`         — Vitest unit + snapshot per state-matrix row
- `src/features/vods/retention.test.ts`        — Vitest pure-function tests

### Files replaced

- `src/features/vods/VodCard.tsx` (v2.0.1 / ADR-0033 version) →
  fully replaced; tests migrated to the new component's interface.
- `src/features/player/ContinueWatchingRow.tsx` (the embedded
  inner card) → switches to `<VodCard variant="compact">`; the
  outer rail composition stays.

### Files unchanged

- All `src-tauri/**` code. No new IPC commands; no migration; no
  schema change; no new ffmpeg invocation.
- `src/ipc/bindings.ts`. Read-only; nothing to regenerate.
- `src/features/timeline/TimelinePage.tsx` (it gets rewritten by
  ADR-0038 Wave 1 implementation; that work consumes this ADR but
  is out of this ADR's scope).
- `src/stores/nav-store.ts`. The `openMultiView` call from the
  selection toolbar uses the existing surface.
- `src-tauri/migrations/`. No new migration in this ADR.

### Test strategy

**Pure-function tests** (Vitest):

- `retention.test.ts` — boundary tests around 7 d, 48 h, 24 h, 0 s,
  past-expired; per `broadcaster_type` value. Property-based
  (fast-check): stage monotonically decreases as remaining-time
  decreases.
- `use-vod-summary.test.ts` — mock `getVod`, `getVodAssets`,
  `getWatchProgress`, `getCoStreams`, `listStreamers`; assert
  composite shape; assert client-side filter to ≥ 50 % overlap.

**Component tests** (Vitest + React Testing Library):

- `VodCard.test.tsx` — one test per state-matrix row (DV7):
  - Snapshot of rendered output.
  - Asserts presence / absence of badges, stripe, outline per the row.
  - Asserts ARIA-label content matches DV9 format.
- `VodCard.test.tsx` — hover sequence:
  - 800-ms delay before frame-strip mounts (using fake timers).
  - Cancel-on-leave-before-delay (timer cleared).
  - Cascade: mouse-hover + Tab-focus on same card; loop persists
    across one trigger ending.
  - Singleton: two cards triggered → only one runs the loop at any
    instant.
- `VodCard.test.tsx` — reduced-motion:
  - `matchMedia('(prefers-reduced-motion: reduce)').matches = true`
    → no scale, no frame-strip loop, panel rendered instantly.
- `VodCard.test.tsx` — stack variant:
  - Silhouette renders when `coStreams.length > 0`.
  - Avatar-dots row at ≤ 4 dots, "+N" at > 4.
  - Fan-out trigger gates same as hover.
  - Shift-Click frontmost selects all perspectives.
  - ArrowRight navigation through fanned cards.

**E2E (Playwright)** — gated on the parallel E2E mission landing
per v2.1 backlog (from v2.0.3). When E2E ships:

- Hover preview cascade across mouse-hover → keyboard-Tab and back.
- Stack-card fan-out at viewport edge (auto-mirror test).
- Selection state syncs into TimelineSelectionBar.
- `prefers-reduced-motion` integration test.

### Migration path

The v2.1 release ships the rewritten VodCard end-to-end. Replacement
is a UI-only change; no schema migration, no IPC migration, no
re-ingest. The v2.0.1 component's tests are migrated to assert the
same high-level semantics against the new interface; if any tests
asserted the v2.0.1-specific accent-border / accent-focus-outline /
1-px-stripe (anchor-noncompliant) behaviour, those assertions are
inverted (anchor-compliant assertions in the rewrite).

---

## CEO Decisions (2026-05-12)

CEO accepted ADR-0039 on 2026-05-12 with three engineering defaults
adopted as a block, no DV substance change. Each block records the
original question (for the audit trail), the decision in one
sentence, the rationale (geglättet aus CTO-Empfehlung), and the
ADR-Folge that binds the decision into the DV-Sektionen. From this
date the three decisions below are binding spec for the Wave-4
VodCard engineering pass. Full per-decision audit-trail in
`docs/decision-log/v2.1-adr-0039-vodcard.md` §7.

### VOd-A2 — Retention source

- **Original question.** Should retention be a backend `retention_at`
  STORED column on `vods` (single-source-of-truth) or computed
  client-side from `streamer.broadcasterType` + `vod.streamStartedAt`
  (no-backend-touch)?
- **Decision.** Client-side compute is the **v2.1 binding default**;
  the backend `retention_at` STORED-column addition moves to the
  **v2.2 backlog** with explicit trigger: **real-use-drift reports**.
- **Rationale.** Tier-promotion frequency per streamer is low; the
  drift is conservative-falsch (card shows *less* retention than
  actually remains during a mid-VOD tier promotion, so no
  user-visible data-loss risk); a backend touch in v2.1 would break
  ADR-0038 D8's no-backend-touch policy unnecessarily. The v2.2
  trigger is data-driven (real-use reports), not speculative.
- **Folge for ADR.** DV5 Computation-source paragraph locked to
  client-side compute; DV8 "Retention source backend-touch" paragraph
  locked to v2.2 deferral with VOd-A2 trigger; Consequences > Costs
  accepted #2 and > Neutral #8 updated to reference VOd-A2;
  Alternatives Considered DV8 "Backend `retention_at` STORED column"
  references VOd-A2's trigger.

### VOd-A3 — Completed-watch indicator at 100 %

- **Original question.** When `watchedFraction === 1` the 3-px stripe
  (DV6) hides; Q-10 was silent on what replaces it. The proposed
  options were (a) nothing, (b) neutral checkmark glyph in the
  metadata strip, (c) accent ✓-badge in image-overlay top-right,
  (d) image-overlay scrim.
- **Decision.** **Option (b)** — a small **neutral checkmark glyph
  in the metadata strip's Line 2 Column 2, right-aligned** — is the
  binding default.
- **Rationale.** (a) "nothing" loses information for Sightline's
  core VOD-aggregator function (watched-vs-unwatched should be
  recognisable at scroll-glance without opening Library filters);
  (c) accent-checkmark in the image-overlay top-right collides with
  the D7 selection-checkmark position and borders H-07 compliance
  (accent on a non-fresh-action surface); (d) image-overlay scrim is
  H-15-borderline (introduces a non-anchor surface effect) and
  reduces thumbnail legibility. (b) is subtle, anchor-compliant
  (neutral; no accent budget consumed), and functional.
- **Wave-3 refinement option (reserved).** ADR-VisualSystem-v2
  (Wave 3) may evaluate optional thumbnail-opacity reduction
  (~75–80 %) as visual reinforcement if real-use feedback surfaces
  an "every-card-has-checkmark background noise" pattern.
  Implementation would be a single-line CSS change in the
  visual-token layer, no ADR re-litigation required.
- **Folge for ADR.** DV6 "Completed-state marker" paragraph locked
  to option (b); DV8 cache-invalidation table `watch:completed` row
  updated from "may appear" to "appears"; DV6 alternatives "Show
  stripe at 100 %" notes the VOd-A3 glyph as the marker;
  Consequences > Risks #6 retained with VOd-A3 reference plus the
  explicit Wave-3 thumbnail-opacity-reduction mitigation path.

### VOd-A5 — Stack-card fan-out edge trajectory

- **Original question.** When the right-side fan-out would exceed
  the viewport right edge, should the trajectory (i) auto-mirror to
  lateral left, (ii) fan out vertically, or (iii) suppress the
  fan-out at the edge (click-only)?
- **Decision.** **Auto-mirror to lateral left** is the binding
  default.
- **Rationale.** Smart-direction is the standard pattern for
  edge-adaptive UI elements (dropdowns, tooltips, context menus);
  users learn it in a single interaction. Breaking the "right cards
  open right" expectation only at the right edge of the viewport is
  the acceptable price for keeping the fan-out affordance functional
  everywhere. Alternative **(ii) vertical** is a layout collision
  with ADR-0038 D4 lane packing (rejected by Engineering); alternative
  **(iii) suppress** makes edge-cards behaviour-divergent from the
  rest of the lane (rejected by Engineering).
- **Folge for ADR.** DV4 "Edge-of-viewport mirror" sub-paragraph
  locked to auto-mirror-left; DV7 state-matrix `stack-fanned` row
  references VOd-A5; Consequences > Costs accepted #4 references
  VOd-A5 with the smart-direction-pattern rationale; Alternatives
  Considered DV4 "Auto-suppress fan-out at edge" notes VOd-A5
  rejection.

---

## Escalation note for CTO (resolved 2026-05-12)

This ADR carried **three** Open Questions (VOd-Q2 / VOd-Q3 / VOd-Q5)
to the CEO in the PROPOSED draft, hitting the escalation threshold
flagged by the Wave-2 mission directive ("Wenn ≥ 3 CEO-relevante
Open Questions am Ende übrigbleiben, lieber an den CTO eskalieren —
Welle-2-Pattern, das in Welle 1 mit drei versteckten Sub-Decisions
umgangen wurde"). The PROPOSED draft offered the CTO three
forward-process options: **(1) forward as-is** to CEO; **(2)
pre-process** by picking the engineering defaults locally and
recording them as "CTO-approved engineering defaults"; **(3)
subdivide** by escalating one OQ and defaulting the other two
engineering-side.

**Resolution (2026-05-12).** The CTO forwarded the three OQs **as-is**
to the CEO (Option 1). The CEO accepted **all three engineering
defaults as a block, without substance change** — see CEO Decisions
§above (VOd-A2 client-compute, VOd-A3 option (b) glyph, VOd-A5
auto-mirror-left) and decision-log §7 for the per-decision
sign-off rationale.

**Anti-Smell pattern confirmed.** The Wave-1 lesson (Hedges-im-ADR
sind eskalations-pflichtig, nicht engineering-default) was applied
preventively to this Wave-2 PROPOSED draft via the §6 Anti-Smell
audit. The audit identified exactly three genuine CEO-relevant Open
Questions and documented eleven additional sub-decisions as
engineering-defaults with explicit rationale. **No hidden
sub-decisions surfaced post-sign-off** — the audit pattern held. The
clean Option-1-forward path confirms that a well-audited
3-OQ-PROPOSED ADR does not need pre-processing; the OQs were
genuine, the engineering defaults were defensible, and the CEO had
the option to override any of the three (and didn't).

---

## References

### Spec sources (binding)

- `docs/reference/visual-language-netflix.md` §3.2 (Thumbnail Card)
- `docs/reference/visual-language-netflix.md` §3.3 (Hover Preview Card)
- `docs/reference/visual-language-netflix.md` §6.2 (VodCard Sightline mapping)
- `docs/reference/visual-language-netflix.md` §7 Q-3 (GIF strip)
- `docs/reference/visual-language-netflix.md` §7 Q-5 (Stack silhouette + fan-out)
- `docs/reference/visual-language-netflix.md` §7 Q-7 (Retention 3-stage escalation)
- `docs/reference/visual-language-netflix.md` §7 Q-10 (3-px progress stripe)
- `docs/decision-log/v2.1-anchor-acceptance.md` (Q-3, Q-5, Q-6 cascade, Q-7, Q-10)
- `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 CEO-A4 (Q-6 keyboard cascade)

### Related ADRs

- [ADR-0038](0038-timeline-layout-single-time-axis.md) — Timeline-Layout
  + D5 multi-perspective grouping (50 % threshold) + D7 selection
  visual + D9 hero-slot favourited-trigger
- [ADR-0008](0008-chapters-via-twitch-gql.md) — Twitch-GQL chapters
  (chapters not rendered by VodCard; reserved for Detail Overlay)
- [ADR-0013](0013-sidecar-bundling.md) +
  [ADR-0034](0034-tauri2-sidecar-layout.md) — sidecar layout (consumed
  by the existing pre-ingest pipeline, not invoked by the card)
- [ADR-0015](0015-timeline-data-model.md) — `stream_intervals` (feeds
  parent TimelineLanes; VOD detail via `getVod`)
- [ADR-0018](0018-watch-progress-model.md) — `watch_progress` data model
  (3-px stripe data source)
- [ADR-0019](0019-asset-protocol-scope.md) +
  [ADR-0027](0027-asset-protocol-scope-narrowing.md) — asset protocol
  (`getVodAssets` choke point; no new asset endpoint in this ADR)
- [ADR-0030](0030-pull-distribution-model.md) — VOD status enum
  (`VodStatus` used in DV2 fallback skeleton conditions)
- [ADR-0032](0032-storage-forecast.md) — Storage Forecast (frame-strip
  storage cost already accounted for)
- [ADR-0033](0033-library-ui-redesign.md) — v2.0.1 VodCard (replaced by
  this ADR's v2.1 successor in Wave 4 engineering)
- **ADR-VisualSystem-v2** (Wave 3, to be filed) — pixel / hex values
  this ADR forward-references throughout
- **ADR-Multiview-Pane-Expansion** (parallel v2.1 wave per ADR-0038
  CEO-A1) — pane-render contract for the 4-pane capacity that this
  ADR's D5 / D7 cross-reference

### Source files referenced

- `src/features/vods/VodCard.tsx` (v2.0.1 / ADR-0033 version — to be
  rewritten in Wave 4)
- `src/features/player/ContinueWatchingRow.tsx` (inner card switches to
  `<VodCard variant="compact">`)
- `src/features/timeline/TimelinePage.tsx` (Phase-4 proof-of-concept;
  the ADR-0038 Wave-4 rewrite mounts this ADR's VodCard)
- `src-tauri/src/services/downloads.rs:971..985` (existing
  `extract_preview_frames` invocation; 6 frames at
  `PREVIEW_FRAME_PERCENTS`)
- `src-tauri/src/services/media_assets.rs:173..210`
  (`backfill_preview_frames` background task)
- `src/ipc/bindings.ts` (read-only): `getVod`, `getVodAssets`,
  `getWatchProgress`, `getCoStreams`, `listStreamers`, type
  definitions for `VodAssets`, `VodWithChapters`, `WatchProgressRow`,
  `CoStream`, `Streamer`, `Interval`
- `src/stores/timeline-multiselect-store.ts` (planned new file per
  ADR-0038 D8)
