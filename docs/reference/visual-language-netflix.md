# Visual Language — Netflix Anchor for Sightline

> **Purpose.** This document is the prose-and-rules anchor for Sightline's v2.1 visual reset toward Netflix's product language. It exists so reviewers — human and synthetic — can decide, rule by rule, whether a Sightline surface is on-anchor or off-anchor without re-litigating the aesthetic. The CEO directive (2026-05-11) names the anchor; this document makes it gradable.
>
> **Status.** Spec source for v2.1. Anchor for a future `@vision-compliance-reviewer` subagent. Not a decision record — ADRs follow CEO sign-off on this document. Not a token spec — token values live downstream in `globals.css`, picked against the heuristics here.
>
> **Scope in.** The look-and-feel of Sightline's content-browse surfaces (`TimelinePage`, `VodCard`, `MultiviewPage`, `LibraryPage`) and the chrome that frames them. Color discipline, typography hierarchy, card and rail patterns, hover/preview behaviour, motion, and the anti-patterns the anchor product deliberately rejects.
>
> **Scope out.** Functional/utility surfaces (`SettingsPage`, dialog forms, error states) where Netflix language adds nothing — these stay in a more neutral utilitarian register, governed by accessibility rules but not by H-04 (hero) or H-25 (rail). Hex codes, exact font sizes, shadow values: decided downstream in v2.1 ADRs against the heuristics, not here.
>
> **Locked input.** CEO has fixed Sightline's accent to `#d4a14a` (warm amber / honey gold). Wherever this document says "accent," it means that hex. All accent-restraint heuristics apply to it identically to how they would apply to Netflix's red on `netflix.com` — the discipline transfers, the colour does not.

---

## 1. Heuristic Catalog

25 rules, each phrased as one assertion plus a `Check:` clause that is decidable from a screenshot, the running app, or the source tree. A reviewer grades each rule pass / fail / not-applicable.

### 1.1 Page & Layout

**H-01: Background dominates over chrome** | Check: In the default state of any non-modal browse view, the page background (a single dominant near-black surface) occupies ≥ 70 % of viewport area. No permanent left or right sidebar is visible.

**H-02: No permanent left navigation rail** | Check: Default state shows no left-edge column with vertical icons or labels. Primary navigation is a thin top bar only, or no chrome at all (canvas-first).

**H-03: Top bar is transparent over hero, opaque on scroll** | Check: At scroll-position 0 with a hero rendered, the top bar's fill is transparent or near-transparent so the hero image reads under it. After ~80 px of scroll, the top bar fills with the page background colour.

**H-04: Hero occupies the upper viewport when present** | Check: A rendered hero/billboard covers ≥ 60 % of the viewport height at scroll-position 0; a bottom-to-mid gradient ensures the hero's title and CTA stay legible against the underlying image.

### 1.2 Color

**H-05: Background uses a single near-black surface** | Check: Page background is a single value in the very-dark range (sRGB lightness L\* ≤ 12 against a standard reference). No multi-stop background gradients between rails; no surface-tint shifts that indicate "section" boundaries.

**H-06: Accent restraint** | Check: In any default-state screenshot, accent-coloured (`#d4a14a`) surface area is ≤ 5 % of the viewport. The accent never paints a full row, panel, sidebar, or wall.

**H-07: Accent reserved for identity, primary CTA, and discrete "new" markers** | Check: The accent appears only on: the wordmark/logo, at most one primary CTA per view (e.g. a Play button), and small badges (e.g. "New," "Expires soon" if used). Never on borders, hover outlines, focus rings, link text body, or active-tab fills.

**H-08: Surfaces stay neutral; depth comes from background layering** | Check: Card backgrounds, modal backgrounds, overlays, and rail backgrounds are all neutral greys at L\* ≤ 25. None carry the accent hue or any saturated tint. Two stacked surfaces differ in lightness by ≥ 3 L\* but ≤ 12 L\* (visibly distinct without being a contrast wall).

### 1.3 Typography

**H-09: Hierarchy is established via weight and size, not colour** | Check: A hero title is ≥ 2× the size of a card title and ≥ 600 in weight. Any two text elements that differ in priority differ in size, weight, or both. Two elements never differ in priority by colour alone.

**H-10: Single sans-serif family with system fallback** | Check: All UI text resolves through one font stack: a custom sans (if licensed) first, then `-apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif`. No serif fonts on chrome, controls, or browse-views. Monospace only inside literal-data contexts (filenames, hash IDs).

**H-11: Body and metadata stay within a 2-step neutral text ramp** | Check: Default body text is one neutral (e.g. white at 100 %); metadata text is one step muted (white at 60–70 %). No third "captionary" colour, no accent-tinted text, no tertiary grey for "less important" labels.

**H-12: No all-caps headings outside small category labels** | Check: H1/H2-level headings are mixed case. All-caps is permitted only on small (< 13 px) genre/category labels and badges; never on titles or descriptions.

### 1.4 Cards

**H-13: Square corners or a single subtle rounding** | Check: Card border-radius is either 0 or in the 2–6 px range, applied consistently across all card variants. No 12–24 px "pill" cards, no asymmetric corners, no per-corner customisation.

**H-14: No visible border in idle** | Check: In idle, card borders are absent or ≤ 1 px and within ±5 % luminance of the card surface (effectively invisible). The card silhouette is defined by content, not by chrome.

**H-15: No Material drop-shadow** | Check: Idle-state card shadow is absent or ≤ 4 px blur at ≤ 15 % opacity. The card does not appear to float above the page. Depth on hover is achieved by scale + layered backdrop, not by elevated shadow.

**H-16: Thumbnail dominates; metadata is secondary** | Check: Within a thumbnail card the image area is ≥ 75 % of the card's bounding height. Text metadata occupies ≤ 25 %, placed below the image or overlaid at the bottom edge under a gradient scrim.

### 1.5 Hover & Preview

**H-17: Hover zoom is decisive but bounded** | Check: On pointer-hover, the card scales to 1.08–1.5× of its idle size and reveals expanded metadata adjacent or below. Neighbouring cards visibly compress, push out of frame, or are occluded by the expanded card.

**H-18: Hover preview triggers after a small delay, not on touch** | Check: A trailer/preview replaces the static thumbnail only after a 0.6–1.0 s hover delay. Immediate hover or a quick mouse-pass does not trigger preview. Touch input never auto-triggers preview.

**H-19: Mute toggle visible during preview; never auto-unmuted** | Check: When preview video plays, audio defaults to muted. An unobtrusive mute/unmute icon is rendered in a corner of the previewed card and is keyboard-reachable.

**H-20: Focus state is a thin neutral outline — never accent-painted fill** | Check: Keyboard focus is indicated by a ≤ 2 px white (or 80–90 % opacity white) outline with no fill change. Accent-coloured fills, accent-coloured glows, or accent-coloured borders as focus indicators are forbidden.

### 1.6 Motion

**H-21: Hover transition is 180–240 ms with a decelerating curve** | Check: Transition duration on hover-zoom and metadata-reveal is in the 180–240 ms range; easing is `ease-out` (or a cubic-bezier equivalent that decelerates). No bounce, no spring overshoot, no perceptible delay between zoom-start and metadata-reveal-start.

**H-22: `prefers-reduced-motion` replaces zoom/preview with a static reveal** | Check: With `prefers-reduced-motion: reduce` active, hover does not scale the card and does not play preview video. Metadata-reveal still appears (so the information is reachable) but without animation — appearance is immediate or via opacity-only fade < 100 ms.

### 1.7 Chrome & Restraint

**H-23: Chrome icons are monochrome outline glyphs** | Check: Top-bar icons (search, notifications, profile, settings) are single-colour outline glyphs rendered at body-text luminance. No filled-colour icons, no multi-tone icons, no accent-tinted icons in chrome.

**H-24: Detail overlay uses a darkened scrim plus a layered card surface** | Check: When a detail overlay opens, the page is covered by a scrim of ≥ 50 % black opacity, and the detail panel sits on a surface 3–12 L\* lighter than the page background. Corner-radius and shadow rules (H-13, H-15) still apply.

**H-25: Horizontal rails snap and paginate; no free-scroll on the wheel** | Check: On pointer drag, arrow click, or arrow key, a horizontal rail advances by approximately one viewport-width (or one full rail-page). Mouse-wheel does not horizontally scroll the rail by default. Edge-arrows appear on hover, are keyboard-reachable, and disappear when no further pagination is possible.

---

## 2. Visual System

### 2.1 Color

- **Background dominance.** The page background is the loudest design choice. It is near-black, single-value, and uninterrupted across the viewport. Every other surface is read against it.
- **Surface ladder.** Two or three neutral surfaces above background, each separated by 3–12 L\* in lightness. Cards in idle sit on the lowest non-background tier; hover-expanded cards lift one tier; modals and overlays sit on the top tier, over a scrim that lets the page show through dimmed.
- **Accent restraint.** The accent (`#d4a14a` for Sightline) is a pointer, not a paint. It marks identity, *one* primary action per view, and discrete "new"/"important" badges. It is never a background, never a hover fill, never a sidebar tint, never a panel border, never running text. If a screenshot of the default view shows the accent on more than ~5 % of pixel area, the heuristic has been broken.
- **No surface drift.** No surface tints toward purple, blue, green, or any non-neutral hue. Two cards on the same page have the same surface colour to within a perceptual JND.
- **Light mode is out of scope for v2.1.** Sightline ships dark-only; this anchor describes dark. A future light-mode pass would be a separate research and re-mapping exercise.

### 2.2 Typography

- **One family.** A single sans-serif family with a system fallback chain. The system fallback exists so the app renders correctly before web fonts load and on locked-down deployments.
- **Hierarchy is dimensional.** Size and weight do the work. A hero title is large and heavy; a card title is medium and medium-heavy; a metadata line is small and regular. Color helps but never substitutes for size/weight.
- **Two-step text ramp.** Default text is one neutral; metadata/secondary text is one step muted. A third "tertiary" grey is a smell — if it seems necessary, the hierarchy is being built with colour where it should be built with size.
- **Restraint on caps and italics.** Mixed-case for titles, descriptions, body. All-caps confined to small category labels. Italics rare and never used for emphasis in body text.
- **Tabular numbers where data matters.** Durations, file sizes, dates, retention countdowns — use the `tabular-nums` font feature so columns align across rows.

### 2.3 Spacing

- **Generous around hero.** Vertical padding above and below a hero is at least ~48 px at desktop widths. Horizontal padding around hero copy is at least ~32 px from the viewport edge.
- **Tight inside the rail.** Cards within a horizontal rail sit with 4–8 px gutters between them. The rail itself has generous vertical breathing between it and the next rail (≥ 32 px).
- **Dense in metadata strips.** Within a card's metadata block, line spacing is tight (1.2–1.4 line-height), label/value pairs sit on the same line where they fit.
- **The page does not centre-clamp content at narrow widths.** Rails extend to the viewport edge (or to a small bleed-margin); the anchor product never windowboxes content into a narrow centre column on a wide monitor.

### 2.4 Elevation

- **Depth is layered backgrounds, not shadows.** A modal sits on a lighter surface over a scrim — that is the entire elevation system. Shadows, if used, are unobtrusive (≤ 4 px blur at low opacity) and serve corner-rounding readability, not "lift."
- **Hover does not elevate via shadow.** Hover lifts via scale, slight metadata reveal, and a corner-arrow / play-icon, never via a glowing shadow halo.
- **No glass-morphism, no neumorphism.** No frosted panels, no inset-and-outset toy shadows. The system is flat-plus-layered, not blurred-plus-glowing.

---

## 3. Component Patterns

For each pattern: a prose description, aspect or size relationships, the three principal states (idle / hover / focus), and the metadata composition.

### 3.1 Hero Row

A hero is the upper-viewport billboard at the top of a browse view. It is a full-bleed image (or muted auto-playing trailer) of a single piece of content, with a bottom-to-mid gradient that ensures legibility of an overlaid title, a short pitch, and two CTAs (one primary, one secondary).

- **Aspect.** Effectively 16:9 or wider (21:9 is common on Netflix's home). At narrow viewports the hero compresses vertically but keeps its image bleed.
- **Idle.** Title at h1 weight and size, 1–3 lines of pitch copy at body weight, two CTAs side-by-side (Play / More Info equivalents). The primary CTA carries the accent; the secondary CTA is a neutral pill.
- **Hover.** Subtle — the hero is already the loudest element on the page. CTA buttons gain a +5–8 % luminance bump; otherwise idle.
- **Focus.** CTAs receive H-20 outline. Hero title is non-focusable; the hero is decoration-plus-CTA, not a focusable card.
- **Metadata.** Title, pitch, rating/runtime metadata strip below the pitch, two CTAs. No timestamps. No author chrome. No social actions.

### 3.2 Thumbnail Card

A thumbnail card is the canonical browse element — one piece of content, image-dominated, minimal metadata, idle quietly until hover.

- **Aspect.** 16:9 image area (matches Twitch VOD thumbnails). Card height is image height plus a thin metadata strip (~25 % of total card height).
- **Idle.** Image fills 75–100 % of the card; below or overlaid at the bottom under a gradient: title (one line, truncated with ellipsis), a metadata line (duration, date, or chapter count — domain-dependent).
- **Hover.** See §3.3 — hover replaces the idle card with a "Hover Preview Card" variant.
- **Focus.** H-20 outline applies. Focused-without-hover renders identically to idle, plus the outline.
- **Metadata.** Title (1 line, truncated). One short metadata line. No author avatar in idle. No tags. No progress bar in idle unless this is a "Continue Watching" rail (then a thin progress bar overlays the bottom 3 px of the image area in the accent or in white).

### 3.3 Hover Preview Card

The hover preview card is the expanded variant of the thumbnail card. It is what makes the anchor product feel like itself.

- **Aspect.** The image area keeps 16:9 but the overall card grows: scale factor 1.08–1.5× of idle, with a metadata panel that extends below (and sometimes laterally beyond) the original card footprint.
- **Trigger.** Pointer-hover sustained for 0.6–1.0 s. Touch input does not trigger this card. Keyboard focus alone does not trigger preview video (see Open Q-6).
- **Transition.** Hover delay → card scale (180–240 ms, ease-out) → after delay, crossfade from static thumbnail to muted preview video over ~250 ms. Departure: reverse — preview fades back to static, card shrinks to idle.
- **Mute toggle.** Visible in the upper-right of the image area while preview plays. Keyboard-reachable.
- **Idle / Hover / Focus.** This card has no idle (it is the hover state of the thumbnail card). Focus visualises with H-20 outline around the expanded card silhouette.
- **Metadata (expanded).** Title at slightly larger size than idle thumbnail title. A 1–2 line summary. A metadata strip with rating/runtime/season-episode-equivalent. A horizontal row of icon buttons (Play, Add-to-list, Like-equivalent). Below: a comma-separated tag/genre list at metadata weight.
- **Reduced-motion fallback.** No scale animation, no preview video. The expanded metadata still appears (so all information stays reachable), but rendered instantly or with a < 100 ms opacity fade.

### 3.4 Detail Overlay

When a card is clicked or actioned, a detail overlay opens — a full-screen-feel modal with hero-style imagery at top, deep metadata below, and (where applicable) episodes / related-content rails further down.

- **Surface.** Sits on a layer slightly lighter than the page background (H-08). Covered region uses a ≥ 50 % black scrim.
- **Header.** Hero-style image or muted trailer at top, ~40–50 % of the modal height. Bottom gradient. Title and primary CTAs overlaid on the hero.
- **Body.** Long-form description, metadata table (cast / runtime / rating / etc.), then optional rails for episodes (TV) or "More Like This."
- **Dismiss.** Close button top-right (H-23 monochrome outline glyph). Esc key dismisses. Click on the scrim dismisses.
- **Focus.** First focusable element on open is the primary CTA. Focus trap inside the modal; Tab cycles within, Shift+Tab reverses. On close, focus returns to the card that triggered it.

### 3.5 Horizontal Rail

A rail is a horizontal lane of thumbnail cards grouped by some axis (category, genre, "trending," "continue watching"). It is the chief organising structure of a Netflix-language browse view.

- **Composition.** A small label at top-left of the rail (category name, all-caps if < 13 px per H-12, mixed case otherwise). Below: a row of thumbnail cards with 4–8 px gutters.
- **Pagination.** Edge-arrows fade in on rail-hover at the left and right of the rail. Clicking an arrow advances by approximately one viewport-width or one full set of cards. Arrow keys when a card in the rail has focus move focus along the rail and trigger pagination at the boundary.
- **Mouse-wheel.** Does not horizontally scroll the rail by default. Vertical-wheel scrolls the page; horizontal-wheel (or trackpad two-finger swipe) may scroll the rail.
- **Snap.** After pagination, the rail snaps so the leftmost visible card is fully visible — no half-cards at the edge.
- **Empty state.** If a rail has fewer cards than fit on screen, no pagination arrows render; the rail simply ends.
- **Loading state.** Skeleton cards at idle dimensions, no motion, no shimmer in `prefers-reduced-motion`.

### 3.6 Single-Time-Axis (Sightline-specific)

This is Sightline's sui-generis pattern. It is **not** a modification of §3.5 Horizontal Rail; the structural correction logged for Q-4 (see §7) makes that explicit. Where a Netflix rail organises cards by category along an interchangeable axis, Single-Time-Axis organises cards by **time** as the dominant spatial dimension: a horizontal time-line runs across the viewport, and VOD cards are placed at the position corresponding to their start-time, stacked into parallel lanes wherever multiple VODs overlap in the same window.

The visible result is a time-topography — wider gaps where nothing was streamed, denser parallel lanes where multiple streamers covered the same hour, an immediate read of "what happened when, in parallel."

- **Composition.** A horizontal time-axis runs across the viewport, labelled with date / hour markers depending on the active zoom. VOD cards float above and below the axis line, anchored by their start-time; cards that overlap in time slot into parallel lanes (vertically stacked) — lanes are not pre-assigned to streamers, they are a packing result of overlap.
- **Conformance.** The cards in this pattern follow §3.2 Thumbnail Card and **H-13 through H-22 unchanged**. Card visual language is identical to any other Netflix-language surface in the app. The *layout* itself is not H-25 compliant (no snap, no pagination, no edge-arrows, no rail-as-axis) — that is deliberate. H-25 governs categorical rails; Single-Time-Axis is not a rail.
- **States.**
  - *Idle.* Time-axis rendered; VOD cards static at their lane positions; the 3-px accent progress stripe (see Continue-Watching below) overlays the bottom edge of any card with watch-progress in (0 %, 100 %).
  - *Hover.* Card hover behaves per §3.3 Hover Preview Card, with the Q-3 override: the preview source is an animated GIF strip (not video), so **H-19 mute-toggle is N/A** for this pattern. Crossfade between idle thumbnail and animated GIF still applies per §4.
  - *Focus.* H-20 unchanged — thin white outline. Tab moves focus along the time-axis (chronological order); the card under focus expands per §3.3 with the same GIF playback as on mouse-hover (Q-6 collapses into Q-3 — there is no video, no separate keyboard-trigger).
- **Multi-perspective indicator (Q-5).** When a moment has 2+ available perspectives (multiple streamers cover the same window), the *frontmost* card carries a **stack silhouette** at idle — the bottom edges of 1–2 cards visible behind it suggest depth — plus a row of small streamer-avatar dots in the metadata strip. On hover, the stacked cards fan out laterally beside the frontmost card and become individually clickable. No numeric corner badges, no tooltips.
- **Continue-Watching progress (Q-10).** No separate "Continue Watching" rail above the Timeline — the chronological time-axis is the canonical browse surface and a second rail would duplicate scaffolding. Instead, every VOD card that has watch-progress in the open interval (0 %, 100 %) renders a 3-px-tall stripe in `#d4a14a` at the bottom of its image area, length proportional to progress. The stripe is the only place on the Timeline where the accent paints a continuous surface — justified by H-07 ("discrete progress where progress is the primary content").

> **Geometry disclaimer.** The concrete layout mathematics — lane-packing algorithm for overlapping cards, card-size-to-axis-scale ratio, zoom-level breakpoints (day / hour / live-window), hover fan-out trajectory — is decided downstream in **ADR-Timeline-Layout**. This §3.6 defines the *pattern* and the *conformance envelope* (H-13–H-22 yes, H-25 deliberately no, Q-5 / Q-10 surface here); it does not pin the geometry.

---

## 4. Motion System

- **Hover zoom factor.** 1.08–1.5×. Exact value picked downstream in v2.1 against the card size; both ends of the range are anchor-faithful.
- **Duration.** Hover transitions in the 180–240 ms band. Preview-crossfade ~250 ms. Modal open/close 200–280 ms. Page scroll-triggered chrome fades 150–200 ms.
- **Easing.** Decelerating curves only on hover and modal-open. `ease-out` or a cubic-bezier with a strong start and gentle landing. No `ease-in` on entrance (it makes UI feel sluggish), no `ease-in-out` on hover (it doesn't reward the gesture).
- **Crossfade for preview switch.** Static thumbnail and preview video sit in the same DOM position with `opacity` interpolated; no slide, no scale change *between them*. The card's own scale change happens before the video starts playing.
- **`prefers-reduced-motion` fallback.** Reduced motion is not "less motion" — it is "no motion that would make a sensitive viewer sick." Translation: no scale, no video preview, no parallax, no horizontal snap-animation. Cross-fades may remain if ≤ 100 ms and on opacity only. Metadata reveals appear instantly. Pagination becomes a single-shot jump.
- **No background motion.** Idle pages do not animate. The anchor product is calm at rest; motion is a reward for attention, not an attention-seeking baseline.

---

## 5. Anti-Patterns

What the anchor product deliberately does *not* do. A reviewer who sees any of these is looking at off-anchor work.

- **No coloured accent walls.** Sidebars, headers, panels, sections are never painted in the accent (or in any saturated hue). The accent is for marks and CTAs.
- **No card-border highlighting.** Hover does not draw an outline around the card. Hover scales the card.
- **No Material drop-shadows.** Cards do not float. Elevation is layering.
- **No persistent sidebar navigation.** No left-rail with icons. Navigation is top-bar or canvas-first.
- **No permanent top-toolbar painted in accent colour.** The top bar is transparent over hero and dark on scroll. Never accent-tinted.
- **No multi-tone icons.** Icons in chrome are monochrome outline glyphs. No "duotone" pairs.
- **No `<dialog>` with neutral-grey "OK / Cancel" rows that look identical.** Primary action carries the accent; secondary is neutral.
- **No skeuomorphic depth (neumorphism, glass-morphism).** Flat-plus-layered is the system; nothing should look pressed-into or frosted-over.
- **No idle-state pulse / shimmer / breathing animation.** Idle is calm. Loading uses unanimated skeletons.
- **No dense info-tables in browse views.** Browse is image-first with minimal metadata. Tables belong in detail overlays or settings.
- **No accent-coloured progress bars except where progress is the primary content** (e.g. a "Continue Watching" progress strip). General-purpose progress bars are neutral.
- **No animated brand-element on idle** (no spinning logo, no breathing wordmark). Brand sits and waits.

---

## 6. Sightline Mapping

How the anchor lands on Sightline's actual views. Per view: a one-paragraph posture, then a table of which heuristics apply and any modifications. "Adopt" = take pattern as-is. "Modify" = take pattern with a domain-specific change. "Drop" = anchor pattern does not apply here, deliberately.

### 6.1 TimelinePage

`TimelinePage` is **Single-Time-Axis** (see §3.6), not a modification of the Netflix Horizontal Rail. The structural correction logged for Q-4 makes this explicit: the question "which axis does the rail use?" was wrong-shaped — Timeline is its own pattern, with time as the dominant spatial dimension and parallel lanes as the visual result of overlapping coverage windows. The card visual language inside the pattern is anchor-faithful (H-13 through H-22); the *layout* is deliberately not H-25.

| Heuristic / Pattern | Adopt / Modify / Drop | Note |
|---|---|---|
| H-01 BG dominance | Adopt | Timeline is the canonical canvas-first view. |
| H-02 No left nav | **Modify** | Q-1 decision: hover-reveal sidebar with collapsed default = avatar-only column (~5–8 % viewport width). Hover or keyboard focus expands to full streamer-name list and filter chips. |
| H-03 Top bar transparency | Adopt | Top bar is transparent over the (conditional) hero region; opaque on scroll. |
| H-04 Hero presence | **Conditional Adopt** | Q-2 decision: hero renders only when ≥ 1 tracked streamer is currently live ("Happening now" hero). When no streamer is live, the page begins directly with the time-axis. |
| H-05 BG single value | Adopt | One near-black canvas. |
| H-06–H-08 Accent restraint | Adopt | `#d4a14a` reserved for primary "Watch" CTA on hero, the Continue-Watching 3-px progress stripe (§3.6 / Q-10), and the < 24 h retention badge (Q-7). Never as row tint, sidebar fill, or focus indicator. |
| H-09–H-12 Typography | Adopt | One sans family, weight-and-size hierarchy, two-step text ramp. |
| H-13–H-16 Cards | Adopt | VOD cards are 16:9 image-dominated, thin metadata strip. See §6.2 for the VodCard specifics. |
| H-17–H-22 Hover & motion | Adopt | Q-3 decision (GIF strip, not video) means H-19 mute-toggle is N/A on the card hover state; everything else applies. |
| H-23 Monochrome chrome icons | Adopt | Top-bar icons (search, filter, settings) become outline glyphs. |
| H-24 Detail overlay | Adopt | Click a timeline card → full-screen overlay with chapter list, multi-perspective links, retention countdown. |
| H-25 Horizontal rail | **Drop — see §3.6** | Timeline is Single-Time-Axis, a sui-generis pattern; H-25 (snap, pagination, edge-arrows, no-wheel-scroll) does not apply. No Continue-Watching rail on Timeline either (Q-10): the time-axis is the canonical browse surface, and progress lives on each card as a 3-px stripe. |
| §3.6 Single-Time-Axis | **Adopt (canonical pattern)** | See §3.6 for the pattern definition; geometry is deferred to ADR-Timeline-Layout. |
| Multi-perspective indicator | **Resolved (Q-5)** | Stack silhouette at idle (1–2 card-bottom-edges visible behind the frontmost card) plus a row of streamer-avatar dots in the metadata strip; on hover, stacked cards fan out laterally and become individually clickable. |
| Retention countdown | **Resolved (Q-7)** | Three-stage escalation. > 7 d: nothing rendered. 7 d ≥ X > 48 h: small text label in metadata strip ("Expires in 4d"). 48 h ≥ X > 24 h: small neutral badge. ≤ 24 h: small badge in accent `#d4a14a` ("Expires <1d"). |

### 6.2 VodCard

The VOD card is where the anchor lands most cleanly. Twitch VOD thumbnails are 16:9 natively. The metadata Sightline cares about (streamer name, duration, retention countdown, chapter count) fits the Netflix metadata-strip pattern. The single sharp conflict is the preview source — Netflix has trailers, Sightline does not.

| Heuristic / Pattern | Adopt / Modify / Drop | Note |
|---|---|---|
| H-13 Corner radius | Adopt | 2–6 px, consistent across the app. |
| H-14 No idle border | Adopt | Remove the current accent-purple border-on-idle. |
| H-15 No Material shadow | Adopt | Remove the current `shadow-lg` defaults. |
| H-16 Thumbnail dominance | Adopt | Streamer name + duration + retention countdown fit below the image. |
| H-17 Hover zoom 1.08–1.5× | Adopt | Bounded scale on hover. |
| H-18 Hover delay 0.6–1.0 s | Adopt | Unchanged; mouse-hover sustained for 0.6–1.0 s before the card expands and the GIF strip starts. |
| H-19 Mute toggle | **N/A (Q-3)** | Q-3 decision: hover preview is an animated GIF strip with no audio. No mute toggle exists; the rule does not apply to Sightline. |
| H-20 Focus outline | Adopt | Replace any accent-coloured focus fill with a thin white outline. Q-6 collapses into Q-3: keyboard focus behaves identically to mouse-hover (H-20 outline + inline GIF playback), since there is no video to gate. |
| Multi-perspective indicator | **Resolved (Q-5)** | Idle: stack silhouette under the frontmost card (1–2 lower edges peek behind) + a row of small streamer-avatar dots in the metadata strip. Hover: the stacked cards fan out laterally beside the frontmost card and become individually clickable. No numeric corner badges, no tooltips. |
| Retention countdown | **Resolved (Q-7)** | Three-stage escalation. > 7 d: nothing rendered. 7 d ≥ X > 48 h: small text label in metadata strip. 48 h ≥ X > 24 h: small neutral badge. ≤ 24 h: small badge in accent `#d4a14a`. |
| Continue-Watching progress | **Resolved (Q-10)** | 3-px stripe in `#d4a14a` at the bottom of the image area when watch-progress ∈ (0 %, 100 %); width proportional to progress. The only continuous-accent surface allowed on a card per H-07 "progress is the primary content." |

### 6.3 MultiviewPage

Multiview is sui generis. Netflix has no concept of watching the same moment from four angles. The anchor here is not the pattern (there is none) but the *posture*: chromeless, BG-dominant, single-purpose view, calm at rest. Whatever the four quadrants look like, the surrounding chrome obeys the anchor.

| Heuristic / Pattern | Adopt / Modify / Drop | Note |
|---|---|---|
| H-01, H-05, H-08 BG | Adopt | Canvas-first. |
| H-02 No left nav | Adopt | Multiview is already chromeless when active. |
| H-20 Focus outline | Adopt | Quadrant focus indicator is a thin white outline; never an accent fill. |
| H-23 Chrome icons | Adopt | The transport bar (play/pause, sync, leader-pick) uses monochrome outline icons. |
| H-04 Hero | Drop | No hero in Multiview — the four quadrants *are* the content. |
| H-25 Horizontal rail | Drop | No rails in Multiview. |
| Quadrant layout (2×2, leader marking) | **Resolved (Q-8)** | All four quadrants stay equal-sized in the default. The leader quadrant is marked only by an H-20 outline (thin white) and by the **audio anchor**: audio defaults to the leader, audio switches when the pointer hovers a different quadrant. A "Focus Mode" toggle (leader ~60 % of viewport, followers ~13 % each) is preserved as a **future opt-in option**, not the default — keeps the 2×2 symmetry as the canonical posture and avoids per-leader-switch re-layout. |

### 6.4 LibraryPage

Library is the boundary between content-browse and utility. It currently shows VOD cards plus storage metadata, filter chips, and bulk actions. The card visual language adopts the anchor; the surrounding chrome (filters, storage outlook, retention controls) is closer to utility — adopt anchor neutrality but not anchor browse-patterns.

| Heuristic / Pattern | Adopt / Modify / Drop | Note |
|---|---|---|
| H-01, H-05, H-08 BG / surface | Adopt | Same canvas, same surface ladder. |
| H-02 No left nav | **Modify** | Same Q-1 decision as TimelinePage: hover-reveal sidebar, collapsed default = avatar-only column. |
| H-13–H-22 VodCard rules | Adopt | Library shows the same VodCard as TimelinePage, including Q-10 3-px progress stripe and Q-5/Q-7 resolutions. |
| H-25 Horizontal rail | **Adopt (Continue-Watching rail only)** | Library Grid itself is not a rail. But see "Continue Watching rail" row below — a single H-25-compliant rail at the top of the page. |
| H-04 Hero | Drop | Library has no hero. |
| Storage outlook component | **Resolved (Q-9)** | Compact strip above the grid showing Free / Used / Forecast in one line (~32–48 px tall). No side panel, no inter-card tile. Respects H-02 (no permanent side column). |
| Continue Watching rail | **Resolved (Q-10)** | A H-25-compliant horizontal rail at the top of `LibraryPage` titled "Continue Watching," 8–12 cards, paginated per H-25 (snap, edge-arrows, no wheel-scroll). Library is not chronologically ordered, so a dedicated rail surfaces partially-watched VODs without forcing the user to scroll the grid. The cards in the rail also carry the 3-px progress stripe per §3.6 / Q-10. |
| Filter chips | **Modify** | Chips are functional, not browse-aesthetic. Anchor rules H-08 (neutral surfaces) and H-20 (focus) apply; H-04/H-17 do not. |

### 6.5 SettingsPage

Settings is utility. The anchor language applies to the chrome (BG dominance, surface neutrality, accent restraint, typography hierarchy) but not to the patterns (no hero, no card grid, no hover-preview). Settings stays utilitarian-but-on-brand.

| Heuristic / Pattern | Adopt / Modify / Drop | Note |
|---|---|---|
| H-01, H-05, H-06, H-08 | Adopt | Canvas, surface, accent restraint, no surface drift. |
| H-09–H-12 Typography | Adopt | One family, hierarchy via weight/size. |
| H-20 Focus outline | Adopt | Required by accessibility regardless. |
| H-23 Chrome icons | Adopt | Section icons (if any) are monochrome outlines. |
| H-04, H-13–H-19, H-21, H-24, H-25 | Drop / N/A | No hero, no cards, no hover preview, no overlay, no rail. |
| Form chrome (inputs, toggles, selects) | **Out of anchor scope** | Governed by separate accessibility and form-state conventions, not by this document. |

---

## 7. CEO Decisions (2026-05-12)

CEO accepted Q-1 through Q-10 as a block on 2026-05-12, with one structural correction to Q-4. From this date, the decisions below are binding spec for the v2.1 ADR phase. Each block records the original question (so the reasoning trail is preserved), the decision in one sentence, and the consequence for this anchor document. Full justifications and the ADR-sequence rationale are in [`docs/decision-log/v2.1-anchor-acceptance.md`](../decision-log/v2.1-anchor-acceptance.md).

### Q-1 — Sidebar in Sightline's chrome

- **Original question.** Keep the sidebar, hide it, or drop it entirely?
- **Decision.** Sidebar stays, but as a **hover-reveal pattern**; the collapsed default is a narrow **avatar-only column** showing only streamer avatars. Hover or keyboard focus expands the column to full names plus filter chips.
- **Consequence for anchor.** H-02 in §6.1 TimelinePage and §6.4 LibraryPage is "Modify (hover-reveal, collapsed = avatar column)" — a deliberate, documented deviation from Netflix's strict no-sidebar posture, justified by the streamer-list being Sightline's primary filter axis. The 5–8 % viewport width of the collapsed column keeps H-01 BG-dominance grossly satisfied.

### Q-2 — Hero on TimelinePage

- **Original question.** What plays the role of the Netflix-style billboard on Sightline's Timeline?
- **Decision.** **Conditional hero**: rendered if and only if ≥ 1 tracked streamer is currently live ("Happening now" hero). When no streamer is live, the page begins directly with the time-axis.
- **Consequence for anchor.** H-04 in §6.1 is "Conditional Adopt." No persistent decorative billboard; the anchor's hero-presence rule applies on the live-condition only.

### Q-3 — Hover-preview source for VOD cards

- **Original question.** What plays in the preview window — VOD scrub, chapter scrub, GIF strip, or nothing?
- **Decision.** **Animated GIF strip** of evenly-spaced frames from the VOD. No video, no audio.
- **Consequence for anchor.** H-19 (Mute toggle) is **N/A** for Sightline — no audio, no toggle. §3.6 Hover-state references §3.3 with this explicit override. The §3.3 description (which still describes Netflix's video-trailer pattern accurately) stays as-is; the Sightline-specific override lives here and in §6.2.

### Q-4 — Timeline-rail axis (structural correction)

- **Original question.** Which axis does the Timeline rail use — chronological window, perspective-group, both?
- **Structural correction.** The question was wrong-shaped. `TimelinePage` is **not** a modification of H-25 Horizontal Rail; it is a sui-generis pattern with **time** as the dominant spatial dimension, parallel lanes as a packing-result of overlapping coverage windows, and a layout that deliberately does not snap or paginate. Treating it as "which axis on the rail" would optimise at the wrong pattern.
- **Decision.** A new pattern **§3.6 Single-Time-Axis (Sightline-specific)** is added between §3.5 Horizontal Rail and §4 Motion System; §6.1 Mapping drops H-25 with a reference to §3.6.
- **Consequence for anchor.** §3.6 inserted (this document). §6.1 H-25 row → "Drop — see §3.6." Geometry (lane-packing algorithm, zoom levels, card-size-to-axis-scale ratio) is deferred to ADR-Timeline-Layout.

### Q-5 — Multi-perspective indicator on a VOD card

- **Original question.** How is "2+ angles available for this moment" surfaced on a card?
- **Decision.** Idle: **stack silhouette** (bottom edges of 1–2 cards visible behind the frontmost card) plus a row of small **streamer-avatar dots** in the metadata strip. Hover: stacked cards **fan out laterally** beside the frontmost card and become individually clickable.
- **Consequence for anchor.** §3.6 includes this as Card-state detail; §6.1 and §6.2 Multi-Perspective rows are resolved (no longer "New / Open Q").

### Q-6 — Keyboard-only users and hover preview

- **Original question.** Does Tab-focus trigger preview, or only mouse-hover?
- **Decision.** **Resolved by Q-3 cascade.** With the preview source being a GIF strip and not video, there is no separate trigger-gate that distinguishes keyboard from pointer. Tab-focus shows the H-20 outline plus the expanded card variant; the GIF plays inline in the same way it does on mouse-hover. `prefers-reduced-motion` (H-22) replaces the GIF with a static mid-frame for both input paths.
- **Consequence for anchor.** No separate Q-6 anchor section or table row. The cascade is logged in [`docs/decision-log/v2.1-anchor-acceptance.md`](../decision-log/v2.1-anchor-acceptance.md). If the CEO later wants an explicit keyboard statement (e.g. Space starts GIF, focus shows only outline without expansion), it lands in a follow-up edit.

### Q-7 — Retention countdown

- **Original question.** Where and at what threshold does the retention countdown surface?
- **Decision.** Three-stage visible escalation on the VodCard:
  - Time remaining > 7 d: nothing.
  - 7 d ≥ X > 48 h: small text label in metadata strip ("Expires in 4d").
  - 48 h ≥ X > 24 h: small **neutral badge** (white / muted, no accent).
  - X ≤ 24 h: small badge in **accent `#d4a14a`** ("Expires <1d").
- **Consequence for anchor.** §6.1 and §6.2 Retention rows are resolved. The < 24 h accent badge is H-07-compliant ("discrete important marker," not surface paint).

### Q-8 — Multiview leader-quadrant prominence

- **Original question.** Should the leader quadrant grow and followers shrink, or stay equal-sized with a marker?
- **Decision.** All four quadrants stay **equal-sized** in the default. The leader is marked only by an H-20 outline and by an **audio anchor**: audio defaults to the leader, audio switches when the pointer hovers a different quadrant. A "Focus Mode" toggle (leader ~60 % viewport, followers ~13 % each) is preserved as a **future opt-in option**, not the default.
- **Consequence for anchor.** §6.3 Quadrant-Layout row is resolved. The audio-anchor behaviour is spec input for downstream Multiview ADRs.

### Q-9 — Storage outlook on `LibraryPage`

- **Original question.** Compact strip, side-panel, or stacked card?
- **Decision.** **Compact strip** above the VOD grid showing Free / Used / Forecast on one line (~32–48 px tall).
- **Consequence for anchor.** §6.4 Storage-Outlook row is resolved. No side-panel deviation from H-02; no inter-card tile that would break the grid's reading hierarchy.

### Q-10 — "Continue Watching" rail

- **Original question.** Does Sightline have a Continue-Watching rail, and on which surface(s)?
- **Decision.** Two surfaces.
  - **On `TimelinePage`:** no dedicated rail. The chronological time-axis is already the canonical browse surface; a second rail would duplicate scaffolding. Instead, every card with watch-progress ∈ (0 %, 100 %) renders a **3-px stripe in `#d4a14a`** at the bottom of its image area, width proportional to progress.
  - **On `LibraryPage`:** a dedicated H-25-compliant rail at the top of the page titled "Continue Watching," 8–12 cards, paginated per §3.5. Library is not chronologically ordered, so a rail surfaces partially-watched VODs without forcing scroll.
- **Consequence for anchor.** §3.6 (Single-Time-Axis) documents the progress stripe as card-state detail. §6.1 confirms no separate rail on Timeline. §6.4 includes a "Continue Watching rail" row as an H-25 Adopt with 8–12 cards.

---

## Versionshistorie

- **2026-05-11** — Initial draft. CEO directive: Sightline visual reset toward Netflix language; brand accent locked at `#d4a14a`. Anchor created so future K-Reviewer / `@vision-compliance-reviewer` runs have a checkable spec; no product code changes in the same run. Open questions Q-1 through Q-10 route to CEO; v2.1 ADRs (Timeline-Layout, VOD-Card, Visual-System v2) follow on sign-off.
- **2026-05-12** — CEO Decisions Q-1 through Q-10 accepted as a block. Structural correction on Q-4 applied — Timeline is sui generis (not a modification of H-25 Horizontal Rail), new component pattern §3.6 Single-Time-Axis inserted between §3.5 and §4. Section 7 renamed from "Open Questions for the CEO" to "CEO Decisions (2026-05-12)" and rewritten as binding spec per Q-block (original question / decision / consequence). Mapping sections §6.1–§6.4 updated: H-25 dropped on Timeline (§3.6 adopted), H-19 marked N/A on VodCard (GIF strip from Q-3), Multi-Perspective and Retention rows resolved (Q-5 / Q-7), Multiview Quadrant-Layout resolved (Q-8), Storage outlook resolved (Q-9), Continue-Watching split (Q-10 — 3-px progress stripe on card / dedicated rail on Library only). Q-6 collapsed by Q-3 cascade; logged in `docs/decision-log/v2.1-anchor-acceptance.md`. Anchor is now binding spec for the v2.1 ADR phase; sequence: ADR-Timeline-Layout → ADR-VodCard → ADR-VisualSystem-v2.
