# ADR-0040 — Visual System v2: Tokens, Geometry, Motion Constants for v2.1

- **Status.** Accepted 2026-05-12
- **Date.** 2026-05-12 (drafted PROPOSED) → 2026-05-12 (accepted by CEO sign-off)
- **Acceptance.** CEO sign-off 2026-05-12 with seven engineering defaults
  adopted as a block, no TV-Decision substance change — **TV-A1**
  (`lane_height_px = 72`; 96 × 54 image readable for GTA-RP content,
  N_max 5/3 good density), **TV-A2** (card-width thresholds 160 / 96 px;
  96 px = image-natural-width is the natural Compact ↔ Sliver kipp-point,
  160 px = 1.67× image-width clean for two-line strip), **TV-A3**
  (`hover-zoom-factor = 1.10`; geometrically safe distance from
  inter-lane overlap edge), **TV-A4** (`sidebar-collapsed-width = 56 px`;
  32-px avatar is the smallest recognisable size, 48 below threshold,
  64 wastes viewport without payoff), **TV-A5** (`hero-slot-height = 280 px`;
  31 % viewport billboard-y without dominant; with TV-A1 yields
  N_max-hero = 3 acceptable), **TV-A6** (hero tie-break =
  most-recently-started; uses existing `streamers.last_live_at`, no
  new IPC), **TV-A7** (live-state indicator = `#d4a14a` accent dot;
  brand touchpoint, H-07-compliant identity marker, coverage minimal).
  Per-decision rationale and Folge in
  `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §8. The CTO
  chose **Forward-as-is** (Option 1 of the three forward-process
  options the Escalation note documented) over Subdivide (Option 3);
  the CEO accepted all seven engineering defaults as a block without
  substance change. Two documentary Pre-Sign-off-CTO-Audit findings
  (Counting-Bug "six" → "seven"; explicit TV-A1 × TV-A5 N_max
  compound-risk notation in Risks) were folded into this acceptance
  patch without altering any TV-Decision substance.
- **Lc-correction 2026-05-15 (R-ADR-01 finding from Wave-4 Mission 1).**
  § TV1 and § Risks #1 APCA Lc-values corrected from over-optimistic
  88 to empirically-measured 58 (max achievable for the locked-input
  accent). Token substance unchanged; correction is documentary per
  R-ADR-02 audit pattern. Detail in
  `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §9.
- **Wave.** v2.1 ADR Wave 3 of (Timeline-Layout → VodCard → **Visual System v2**).
  Parallel v2.1 wave: Multiview-Pane-Expansion-ADR (per ADR-0038 CEO-A1) —
  filed separately. After this ADR's sign-off and the Multiview-Pane-
  Expansion-ADR's sign-off, the v2.1 ADR phase is structurally complete
  and Wave-4 engineering can begin.
- **Related.**
  [ADR-0038](0038-timeline-layout-single-time-axis.md) (ACCEPTED 2026-05-12,
  four CEO refinements; all D-Decisions are binding inputs — D2 default
  Level 4 24 h, D3 Now-indicator 1-px neutral white at ~30 % opacity, D4
  `lane_height_px` as central variable, D7 16-px accent-checkmark, D9
  hero-slot favourited-trigger) ·
  [ADR-0039](0039-vodcard-composition-hover-stack-retention.md) (ACCEPTED
  2026-05-12, three CEO refinements; all DV-Decisions are binding inputs —
  DV1 16:9 image at ≥ 75 % share + three width variants Full/Compact/Sliver,
  DV2 idle composition position-map, DV3 800-ms hover delay + cascade,
  DV4 stack silhouette + fan-out, DV5 retention 3-stage escalation, DV6
  3-px progress stripe, DV10 400-ms frame cadence) ·
  [ADR-0033](0033-library-ui-redesign.md) (v2.0.1 VodCard surface — token
  migration replaces accent border + accent focus outline + accent ✓-badge
  + 1-px stripe with anchor-compliant equivalents) ·
  [ADR-0019](0019-asset-protocol-scope.md) +
  [ADR-0027](0027-asset-protocol-scope-narrowing.md) (asset protocol —
  unchanged; tokens do not affect path scope).
- **Spec sources (binding).**
  `docs/reference/visual-language-netflix.md` — all 25 heuristics
  (H-01..H-25), §2 (Visual System: Color/Typography/Spacing/Elevation),
  §3 (all 6 component patterns including §3.6 Single-Time-Axis), §4
  (Motion System), §5 (Anti-Patterns), §6 (Sightline Mapping for all
  pages), §7 (all CEO Decisions Q-1..Q-10) plus the
  **`#d4a14a` Locked Input** in the §11 Locked-input block (CEO
  initial decision 2026-05-11). Decision-logs:
  `docs/decision-log/v2.1-anchor-acceptance.md` (Q-3, Q-5, Q-6, Q-7,
  Q-10) and `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 +
  `docs/decision-log/v2.1-adr-0039-vodcard.md` §7 (per-CEO-decision
  audit-trails).

## Context

This ADR is the **final token-pinning ADR of the v2.1 ADR phase**. It
converts every forward-reference left dangling by ADR-0038 (Wave 1)
and ADR-0039 (Wave 2) into concrete hex, pixel, millisecond, easing,
and z-index values. After this ADR's sign-off, every visual decision
needed for the Wave-4 VodCard / TimelineLanes / HeroSlot / SidebarReveal
engineering pass is determined.

Three layers of binding inputs feed into this ADR:

1. **The Netflix anchor** (`docs/reference/visual-language-netflix.md`)
   — locks the *discipline* (25 H-NN heuristics + §2 surface ladder +
   §4 motion ranges + §5 anti-patterns) and the **brand accent
   `#d4a14a`** (CEO Locked Input, 2026-05-11).
2. **Wave-1 + Wave-2 D / DV decisions** — lock the *contracts*
   (16:9 aspect, three width variants, 800-ms hover delay, 400-ms
   frame cadence, three-stage retention, 3-px stripe, 50 %
   multi-perspective overlap, selection-cap 4, lane-by-lane Tab
   order, etc.).
3. **Existing v2.0.x token bench** (`src/styles/globals.css`) —
   the platform that gets migrated. Survey:

| Existing token (v2.0.x)              | Status in v2.1                                         |
| ------------------------------------ | ------------------------------------------------------ |
| `--color-bg: #0a0a0d`                | Kept (already anchor-conformant; L\* ≈ 4)              |
| `--color-surface: #14141a`           | Renamed `--color-surface-1` (Surface-Ladder L1)        |
| `--color-surface-elevated: #1c1c24`  | Renamed `--color-surface-2` (Surface-Ladder L2)        |
| `--color-surface-interactive: #242430` | Renamed `--color-surface-3` (Surface-Ladder L3)      |
| `--color-fg: #f5f5f7`                | Kept (body text neutral, H-11)                         |
| `--color-muted: #a1a1aa`             | Kept (muted text, H-11)                                |
| `--color-subtle: #71717a`            | **Removed** (third grey violates H-11)                 |
| `--color-accent: #7c3aed`            | **Replaced** with `#d4a14a` (Locked Input)             |
| `--color-accent-fg: #ffffff`         | Refined to `#0a0a0d` (accent is light; fg is dark bg)  |
| `--color-focus-ring: #8b5cf6`        | **Replaced** with `rgba(255,255,255,0.50)` (H-20)      |
| `--motion-fast/base/slow`            | Refactored into per-purpose `--motion-*` constants     |
| `--radius-sm/md/lg/xl`               | Refactored into `--card-corner-radius` + utility radii |
| `--shadow-sm/md/lg`                  | **Reduced** to `--shadow-modal` only (H-15 / §2.4 max 4 px blur) |
| `prefers-color-scheme: light` block  | **Removed** (dark-only is binding)                     |
| `sightline-shimmer` keyframe         | **Removed** (anti-pattern §5: no idle-state pulse)     |
| `color-scheme: dark light`           | **Changed** to `color-scheme: dark`                    |

**Out of scope for this ADR.**

- Token-migration in code (the actual `globals.css` rewrite, the
  hard-coded-hex audit across `src/`, the Tailwind utility-class
  migration). This ADR ships the spec; **Wave-4 engineering** ships
  the implementation.
- Per-component CSS migration. This ADR commits to "components
  consume tokens via CSS custom properties or Tailwind utilities";
  components that hard-code hex / px (instead of tokens) need
  individual migration in Wave 4 — inventory of those is a Wave-4
  engineering task.
- Multiview-Pane-Expansion-ADR concerns. That parallel ADR (CEO-A1
  follow-up) pins the 4-pane layout geometry within the Multiview
  surface; this ADR's tokens are referenced from there for chrome
  consistency, not the other way around.
- Light-mode tokens. Hard constraint: dark-only in v2.1. A future
  v2.x ADR could re-introduce a light variant; until then, no
  light-mode token alternatives are spec'd.
- Detail-Overlay content composition. The overlay's surface +
  scrim + corner-radius tokens are pinned here (TV1 / TV4 / TV7);
  the overlay's body composition (chapter list, multi-perspective
  links, action buttons) lands in Wave-4 engineering against the
  §3.4 anchor pattern.

---

## Decisions

### TV1 — Color System

**Decision.** A 4-tier surface ladder + 3-tier neutral text ramp +
single brand accent `#d4a14a` + neutral focus / selection outline +
50 %-black modal scrim + no live-status colour outside the accent or
neutral white (Anti-Pattern §5: no surface drift toward red/blue/green).

**Surface Ladder.**

| Token                       | Hex       | L\* (sRGB approx.) | Use                                                      |
| --------------------------- | --------- | ------------------ | -------------------------------------------------------- |
| `--color-bg`                | `#0a0a0d` | ~4                 | Page background (binding; anchor §2.1 single-value bg)   |
| `--color-surface-1`         | `#14141a` | ~7                 | Card backgrounds, metadata strips, top-bar-on-scroll     |
| `--color-surface-2`         | `#1c1c24` | ~10                | Hero-slot, sidebar-expanded panel, selection toolbar     |
| `--color-surface-3`         | `#242430` | ~13                | Detail-overlay surface, modal panels                     |
| `--color-surface-skeleton`  | `#10101a` | ~5                 | Skeleton placeholder (just above bg; subtle)             |
| `--color-scrim`             | `rgba(0,0,0,0.60)` | —         | Modal backdrop (H-24 ≥ 50 % black scrim)                 |

L\*-step ladder: bg → S1 ≈ +3, S1 → S2 ≈ +3, S2 → S3 ≈ +3. Anchor §2
prescribes 3–12 L\* between surfaces; values land at the conservative
end of the range, preserving "calm at rest" without losing surface
distinguishability. Skeleton sits just above bg (+1 L\*) — visible
during loading but not distracting; no shimmer (§5 anti-pattern).

**Text Ramp (H-11 two-step neutral; third grey removed).**

| Token            | Value                         | Use                                                  |
| ---------------- | ----------------------------- | ---------------------------------------------------- |
| `--color-fg`     | `#f5f5f7`                     | Body text, titles, headings (default reading colour) |
| `--color-muted`  | `rgba(245,245,247,0.65)`      | Metadata text, captions, secondary line (H-11 60–70 %) |

The pre-existing `--color-subtle: #71717a` is **removed**. H-11 binds
to a two-step ramp; the third grey breaks the discipline and was a
smell carried in from the pre-anchor codebase.

**Brand Accent (`#d4a14a` Locked Input).**

| Token              | Value                                | Use                                              |
| ------------------ | ------------------------------------ | ------------------------------------------------ |
| `--color-accent`   | `#d4a14a`                            | Brand accent (locked); primary CTAs, marks       |
| `--color-accent-fg` | `#0a0a0d`                            | Foreground on accent fill (dark on light accent) |

`#d4a14a` is a warm amber / honey gold:
- HSL: `40°, 60 %, 56 %`
- OKLCH: `0.74 L, 0.12 C, 82° h` (approx.)
- APCA contrast against `#0a0a0d`: Lc ≈ 58 (maximum achievable for
  the locked-input `#d4a14a` accent). Passes APCA L\*c ≥ 45 threshold
  for large fluent text at ≥ 24 px / weight ≥ 500 — Hero-CTA "Watch
  live" sits in this band (`--font-size-2xl` 24 px). Does NOT pass
  the Lc 75 small-body-text threshold; accent fill must not carry
  body text in v2.1. Documented as binding constraint.
- APCA contrast against `#f5f5f7`: Lc ≈ 44 (empirically measured in
  the Wave-4 Mission-1 APCA-test; the earlier Lc ≈ 30 claim was
  itself divergent — over-pessimistic). Still well below the Lc 75
  body-text threshold; `--color-fg` must never sit on an accent fill.
  The Hero-CTA uses `--color-accent-fg` (dark) per § Risks #1, not
  `--color-fg`.

**H-06 / H-07 conformance — coverage-based, not saturation-based.**

Anchor §1.2 H-06 ("Accent restraint") and §2.1 ("Accent restraint")
both phrase the constraint as **viewport coverage** ("≤ 5 % of the
viewport pixel area"), not as saturation ceiling. The `#d4a14a` value
is full-saturation amber; this is **anchor-compliant** because:

1. The discipline transfers from Netflix's red — full-saturation is
   fine *if* coverage is restrained.
2. Coverage-based anchor allows the brand colour to feel branded
   (not desaturated to grey-tan) while keeping discipline.
3. No secondary "desaturated UI variant" is needed because the
   accent is used only on discrete-marker surfaces (selection
   checkmark, retention stage-4 badge, 3-px stripe, hero "Watch"
   CTA).

**Coverage math verification (anti-smell).** Worst-case per-card
accent at a 200 × 72 = 14 400 px² Full-variant card:

| Element                  | Dimensions                    | Area (px²) |
| ------------------------ | ----------------------------- | ---------- |
| Selection checkmark      | 16 × 16                       | 256        |
| Retention stage-4 badge  | ~46 × 20                      | ~920       |
| Progress stripe at 50 %  | 48 × 3 (= 0.5 × 96 image-w × 3) | 144       |
| **Total worst case**     |                               | ~1 320     |

→ Per-card peak coverage: 1 320 / 14 400 = **9.2 %**.

This is above H-06's 5 % when computed per card. **But H-06 measures
viewport coverage, not per-card.** Realistic viewport-level coverage
at typical density (1 200 × 700 = 840 000 px² with ~30 cards, ~5
selected, ~3 with stage-4 retention, ~10 with progress, plus hero
"Watch" CTA accent ~200 × 40):

| Source                        | Approx. area in viewport (px²) |
| ----------------------------- | ------------------------------ |
| 5 selection checkmarks        | 1 280                          |
| 3 stage-4 retention badges    | 2 760                          |
| 10 progress stripes (mean 50 %) | 1 440                          |
| 1 hero "Watch" CTA            | 8 000                          |
| Wordmark / logo               | ~1 000                         |
| **Total**                     | ~14 480                        |

→ Viewport accent coverage: 14 480 / 840 000 = **1.72 %**.

Well within H-06's 5 % budget. The per-card peak of 9.2 % is the
**intentional cost** of carrying three independent discrete markers
on one card; reducing it would mean dropping a marker (selection,
retention, or progress), each of which is binding spec from ADR-0038
D7 / ADR-0039 DV5 / ADR-0039 DV6. **Documented as anti-smell
verification:** per-card coverage is intentional, viewport coverage
is anchor-conformant.

**Outline Colours (H-20 neutral, no accent-fill focus).**

| Token                       | Value                         | Use                                          |
| --------------------------- | ----------------------------- | -------------------------------------------- |
| `--color-outline-focus`     | `rgba(255,255,255,0.85)`      | Keyboard focus ring (H-20 binding 80–90 %)   |
| `--color-outline-selection` | `rgba(255,255,255,0.85)`      | Selection outline on idle-selected card      |
| `--color-outline-hover`     | `rgba(255,255,255,0.30)`      | Optional softer hint on hover-not-triggered (Sightline-specific helper, outside H-20 domain) |

H-20 binds focus to a ≤ 2-px white outline (anchor-explicit:
"white (or 80–90 % opacity white) outline with no fill change"). The
85 % opacity sits at the mid of the H-20-binding 80–90 % range — it
keeps the ring crisp against both `--color-bg` and `--color-surface-1`
without consuming pure-white visual weight. Selection re-uses the
same source per ADR-0038 D7 ("the same H-20 used for focus, so a
selected-and-focused card reads as one state, not two"). The
hover-not-triggered helper at 30 % opacity is a Sightline-specific
softer-hint indicator (not subject to H-20, which governs keyboard
focus only).

**Border Colours (Strip dividers, frame edges; H-14 idle invisibility).**

| Token                    | Hex       | L\* approx. | Use                                                |
| ------------------------ | --------- | ----------- | -------------------------------------------------- |
| `--color-border`         | `#2a2a34` | ~17         | Hairline dividers (top-bar bottom-edge, modal head) |
| `--color-border-strong`  | `#3a3a48` | ~22         | Optional emphasised hairline (rarely used)         |

H-14 says card border in idle is "absent or ≤ 1 px and within ±5 %
luminance of the card surface (effectively invisible)". `--color-border`
sits at L\* ~17 against `--color-surface-1` at L\* ~7 — that's +10
luminance, which is *visible*. Use:
- **Cards in idle: NO border** (use Surface-1 surface against bg
  to define silhouette; H-14 binding).
- `--color-border` is reserved for non-card chrome (top-bar-divider,
  modal-head-divider, form-field-divider).

**Status Colours (functional, not browse).**

| Token             | Hex       | Use                                                |
| ----------------- | --------- | -------------------------------------------------- |
| `--color-success` | `#10b981` | Form success states (Settings)                     |
| `--color-warning` | `#f59e0b` | Form warning states (Settings)                     |
| `--color-danger`  | `#ef4444` | Form error states (Settings); never on browse      |
| `--color-info`    | `#3b82f6` | Form info states (Settings)                        |

These exist in v2.0.x and stay; their use is constrained to functional
surfaces (Settings forms, validation). Per §5 anti-patterns, they do
**not** paint browse-view surfaces. The previous `--color-info`
appearance as a fallback bar-tint in TimelinePage (Phase-4 proof-of-
concept) gets removed in Wave-4 (the bar is replaced by the v2.1
VodCard rendering).

**Live-state indicator colour.** Out of the status colours. Sightline's
"streamer is currently live" mark sits inside the avatar ring; per
anchor §2.1 ("no surface drift toward purple/blue/green/red"), it
**does not** carry a saturated non-neutral hue. **Binding per TV-A7
(CEO Decisions §below):** small `--color-accent` dot inside the avatar
ring, interpretable as "this person's identity is currently live"
(H-07 identity-or-new-marker compliant). Brand-touchpoint rationale:
Sightline-Honiggold becomes a persistent live-state signal in the
sidebar. Coverage minimal (64 px² per dot × typical 5–10 visible
streamers = negligible against TV1 5 %-viewport budget). Alternative
neutral white dot considered and rejected.

**No-Red-in-UI verification.** The anchor does **not** carry an
explicit "no red" constraint (unlike some other anchors with stronger
chromaticity rules); the rule is the more general §5 "no coloured
accent walls" + §2.1 "no surface drift". The status palette above
(`--color-danger #ef4444`) is OK because it's confined to functional
form states, never browse. **Verified: no anchor rule forbids
`#ef4444` in form-error contexts; the constraint is purely scope
(functional, not browse).**

**Rationale.** This palette is a minimal anchor-compliant set: one
brand accent (locked), neutral surfaces with a 4-step ladder, two-step
text ramp, neutral outlines, dark scrim. Status colours stay only for
functional surfaces. No additional tertiary tints, no light-mode
fallback, no hover-glow halo (§2.4 + H-15).

### TV2 — Typography System

**Decision.** Single system-font stack, dimensional hierarchy via
weight + size only, tabular numerics for data columns, two-step text
ramp (TV1 above). Type scale runs from 12 px (meta) to 32 px (hero
title), four weights (400 / 500 / 600 / 700).

**Font stack.**

```css
--font-stack: ui-sans-serif, system-ui, -apple-system, "Segoe UI",
              Roboto, "Helvetica Neue", Arial, sans-serif;
--font-stack-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
```

System-font-stack only (anchor §2.2 "single sans-serif family with
system fallback"). No custom font ships in v2.1; the stack is
optimised for native OS rendering and zero font-load latency. Mono
stack reserved for filenames / hashes / IDs in Settings; not used in
browse surfaces.

**Weight ladder.**

| Token                    | Value | Use                                                |
| ------------------------ | ----- | -------------------------------------------------- |
| `--font-weight-body`     | 400   | Body text, captions, metadata                      |
| `--font-weight-mid`      | 500   | Card title (subtle emphasis vs. body)              |
| `--font-weight-title`    | 600   | Section labels, modal headers                      |
| `--font-weight-heading`  | 700   | Hero title, page headings                          |

Four steps; H-09 binding ("hierarchy via weight and size, not colour").
A hero title at 32 px / 700 reads decisively larger than a card title
at 14 px / 500.

**Type scale.**

| Token              | Value (rem)  | px (at 16 px base) | Use                                          |
| ------------------ | ------------ | ------------------ | -------------------------------------------- |
| `--font-size-xs`   | `0.75rem`    | 12                 | Metadata line, retention text, badge text    |
| `--font-size-sm`   | `0.875rem`   | 14                 | Card title, button label, body-dense         |
| `--font-size-base` | `1rem`       | 16                 | Body text, modal copy                        |
| `--font-size-lg`   | `1.125rem`   | 18                 | Section labels, sidebar streamer-name        |
| `--font-size-xl`   | `1.25rem`    | 20                 | Modal header, page title (non-hero)          |
| `--font-size-2xl`  | `1.5rem`     | 24                 | Hero CTA label                               |
| `--font-size-hero` | `2rem`       | 32                 | Hero title                                   |

H-09 binding: a hero title at 32 px is 2× the card title at 14 px and
≥ 600 weight (700 in our ladder) — satisfies the H-09 explicit
example ("hero title ≥ 2× card title and ≥ 600 weight"). H-12 binding:
all titles mixed case; all-caps only on < 13 px category labels (handled
case-by-case in components, not at the token layer).

**Line-height ladder.**

| Token                   | Value | Use                                                  |
| ----------------------- | ----- | ---------------------------------------------------- |
| `--line-height-tight`   | 1.2   | Metadata strip line 1 (title with ellipsis)          |
| `--line-height-body`    | 1.4   | Body / modal copy                                    |
| `--line-height-loose`   | 1.6   | Long-form paragraphs (rare; Settings descriptions)   |

§2.3 anchor binds metadata strip to "tight" line-height (1.2–1.4).

**Letter-spacing.** Default `normal` everywhere. Headings get
`-0.01em` (`tracking-tight` in Tailwind) — convention for large
display text. All other tokens stay at default.

**Tabular numerics.** Applied via `font-variant-numeric: tabular-nums`
on:
- `--font-feature-duration` (4h 12m, 45 min, etc.)
- `--font-feature-retention` (Expires in 4d, 18h, etc.)
- `--font-feature-file-size` (Settings storage display)

The CSS class `.font-tabular { font-variant-numeric: tabular-nums; }`
gets defined in `globals.css` for component-level application.

**Rationale.** System-font is anchor-binding (§2.2) plus zero
load-latency; weight + size hierarchy keeps H-09 explicit; tabular
numerics keep duration / retention readable across rows (anchor
§2.2 binding). Four weights, seven sizes — minimal and
expressive enough for every surface from card-meta to hero-title.

### TV3 — Spacing & Layout Scale

**Decision.** 4-px base grid, 10-step spacing scale; tight gutters
within rails (4–8 px per anchor §2.3), generous spacing around hero
(≥ 32 px per anchor §2.3), no page centring-clamp (anchor §2.3
"never windowboxes content into a narrow centre column").

**Spacing scale (4-px base).**

| Token         | Value (rem) | px  | Common use                                          |
| ------------- | ----------- | --- | --------------------------------------------------- |
| `--space-0`   | `0`         | 0   | Reset / no gap                                      |
| `--space-1`   | `0.25rem`   | 4   | Card-gutter inside lane (tight; anchor §2.3 4–8 px) |
| `--space-2`   | `0.5rem`    | 8   | Inter-lane vertical spacing, badge inner-padding    |
| `--space-3`   | `0.75rem`   | 12  | Sidebar collapsed-padding, strip inner-padding-h    |
| `--space-4`   | `1rem`      | 16  | Page-padding (narrow), card outer-padding default   |
| `--space-5`   | `1.25rem`   | 20  | Settings form field gap                             |
| `--space-6`   | `1.5rem`    | 24  | Section spacing, page-padding (mid)                 |
| `--space-8`   | `2rem`      | 32  | Hero-padding-h, page-padding (wide)                 |
| `--space-12`  | `3rem`      | 48  | Hero-padding-v, between-section spacing             |
| `--space-16`  | `4rem`      | 64  | Hero-to-content gap; rarely larger spacing          |

8-px grid is the dominant rhythm (most common values are 8, 16, 24,
32, 48). The 4-px and 12-px steps fill gaps where the strict 8-px
grid would feel too coarse.

**Specific layout values (composed from scale).**

| Surface                    | Spacing                                              |
| -------------------------- | ---------------------------------------------------- |
| Card-gutter (inside lane)  | `--space-1` (4 px)                                   |
| Inter-lane vertical        | `--space-2` (8 px)                                   |
| Hero outer padding         | `--space-8` horizontal × `--space-12` vertical       |
| Page outer padding         | `--space-4` (narrow) / `--space-6` (mid) / `--space-8` (wide) |
| Strip inner padding        | `--space-2` horizontal × `--space-1` vertical        |
| Badge inner padding (retention) | `--space-2` (= 8 px? — see TV5 for refined value) |
| Modal outer padding        | `--space-6` (24 px on all sides)                     |
| Settings field gap         | `--space-4` (16 px between adjacent inputs)          |

**Page-width policy.** No max-width clamp (anchor §2.3 binding). At
ultra-wide monitors (> 1920 px), content extends to a small bleed
margin (`--space-8` on each side) — the timeline-lanes go right to
the edge of that margin. No centre-windowboxing.

**Rationale.** The 4-px-base + 10-step ladder is sufficient for every
documented surface in §6 mapping; the inter-lane-spacing at 8 px
(2 × card-gutter) lets the eye separate lanes without breaking the
time-axis-as-spine invariant of §3.6. The hero-padding at 32×48 is
on the generous side of the §2.3 prescription, supporting H-04
(hero occupies ≥ 60 % viewport height when present).

### TV4 — Card Geometry Numerics

**Decision.** `lane_height_px = 72` (binding per **TV-A1**, CEO
Decisions §below). Three width variants: Full ≥ 160 px, Compact
96–160 px, Sliver < 96 px (binding per **TV-A2**). Card corner radius
4 px (mid-range of H-13 2–6 px). Image aspect 16:9 (binding from
ADR-0039 DV1). Image share 0.75 (binding from H-16 + ADR-0039 DV1).
Hover zoom factor 1.10 (binding per **TV-A3** within H-17 1.08–1.5×).

**Card geometry table.**

| Token                           | Value      | Source / Rationale                                                    |
| ------------------------------- | ---------- | --------------------------------------------------------------------- |
| `--card-lane-height`            | `72px`     | TV-A1 binding (CEO Decisions §below)                                  |
| `--card-image-share`            | `0.75`     | H-16 binding via ADR-0039 DV1 (≥ 75 % image share)                    |
| `--card-aspect`                 | `16 / 9`   | Twitch native, anchor §3.2 + ADR-0039 DV1 binding                     |
| `--card-corner-radius`          | `4px`      | Mid of H-13 (2–6 px); applied consistently to card, badge, hero       |
| `--card-width-readable-px`      | `160px`    | TV-A2 binding — Full ↔ Compact threshold                              |
| `--card-width-image-only-px`    | `96px`     | TV-A2 binding — Compact ↔ Sliver threshold                            |
| `--card-hover-zoom-factor`      | `1.10`     | TV-A3 binding within H-17 1.08–1.5×                                   |

**Derived dimensions (computed at runtime, not free tokens).**

- **Image natural height** = `0.75 × 72 = 54 px`
- **Image natural width** = `54 × (16/9) = 96 px`
- **Strip height** = `0.25 × 72 = 18 px` (slightly tight — strip is two-line in Full at 14 + 12 font-sizes with `--line-height-tight`)
- **Lane area per side at hero-absent** = `(viewport_h - top-bar 56 - bottom-chrome 64) / 2 = (880 - 120) / 2 = 380 px` → `N_max-per-side = floor(380 / 72) = 5` lanes
- **Lane area per side at hero-present** = `(880 - 120 - 280) / 2 = 240 px` → `N_max-per-side = floor(240 / 72) = 3` lanes

The N_max ladder (5 / 3) closely matches ADR-0038 D4's illustrative
prediction (6 / 4) — the slight difference traces to the chosen
`lane_height_px = 72` vs. ADR-0038's illustrative 60 px. ADR-0038 D4
explicitly states "Exact numbers depend on tokens — ADR-VisualSystem-v2
pins them. The algorithm above is the *contract*: N_max scales
linearly with available lane-area." So 5 / 3 is the v2.1 binding
result; 6 / 4 would arise if we picked `lane_height_px = 60`
(see TV-A1 alternatives).

**Width variants — visual semantics.**

| Variant   | Card width range            | Renders                                                       |
| --------- | --------------------------- | ------------------------------------------------------------- |
| Full      | ≥ `--card-width-readable-px` = 160 px | Image (96 × 54, left-anchored) + strip with title-line + duration-line   |
| Compact   | 96 px ≤ w < 160 px          | Image (96 × 54) + strip with title-only (one line, truncated) |
| Sliver    | < 96 px (= image-natural-width) | Image-only sliver, vertically thin streamer-tinted bar (no metadata strip) |

At Level 4 default zoom (24 h / 1 200 px ≈ 50 px/h), card-widths land
typically in:
- 30-min stream: 25 px → Sliver
- 1 h stream: 50 px → Sliver
- 2 h stream: 100 px → Compact
- 4 h stream: 200 px → Full
- 8 h stream: 400 px → Full (extra width to the right of the 96-px image is rendered as a faint duration-indicator tail in the strip area)

Engineering note for the Wave-4 implementation: when card-width >
image-natural-width (96 px), the image stays at its natural size,
left-anchored within the card; the rest of the card-width is rendered
as a slim duration-indicator bar in `--color-surface-1` tint, drawing
the eye along the time extent without competing with the image.

**Hover zoom — geometry implication.** With `--card-hover-zoom-factor: 1.10`,
a 72-tall Full card scales to 79.2 tall on hover. Inter-lane spacing
(`--space-2 = 8 px`) absorbs the 7.2 px growth without overlapping
the next lane. A 1.20× zoom would scale to 86.4 (8.4 px growth), still
within the inter-lane budget. A 1.30× would scale to 93.6, overlapping
the next lane by 5.6 px — visually messy. The H-17 anchor range
1.08–1.5× is wider than what Sightline's dense timeline can absorb;
the v2.1 binding is 1.10 (conservative end). See TV-A3 for
alternatives evaluated.

**Rationale.** `lane_height_px = 72` is the central trade-off in
this ADR; lower values produce more lanes but smaller-thumbnails,
higher values produce fewer lanes but more legible thumbnails.
72 sits at a "small but recognisable" thumbnail size (96 × 54 image
shows visible action / streamer-face for GTA-RP-style content) while
preserving ADR-0038 D4's N_max ladder closely. Width-variant thresholds
160 / 96 are aligned with the image-natural-width: Compact is the
minimum width that renders the full image plus title, Sliver kicks
in when card-width falls below the image's natural footprint. Hover
zoom 1.10 is bounded by inter-lane spacing math; aggressive zoom
breaks neighbouring-lane legibility on dense days.

### TV5 — Card Element Numerics (finalizes ADR-0039 DV2–DV6)

**Decision.** Token sizes for every per-card overlay element, anchored
to ADR-0039's binding decisions.

| Token                              | Value      | Anchor / Source                                                  |
| ---------------------------------- | ---------- | ---------------------------------------------------------------- |
| `--selection-checkmark-size`       | `16px`     | ADR-0038 D7 binding; top-right of image area, padding 4 px from edges |
| `--selection-checkmark-padding`    | `4px`      | Distance from image-corner to badge-edge                         |
| `--retention-badge-height`         | `20px`     | Engineering default; readable at 12-px clock-icon + tabular text |
| `--retention-badge-padding-x`      | `6px`      | Horizontal padding inside badge                                  |
| `--retention-badge-icon-size`      | `12px`     | Clock-icon glyph size                                            |
| `--retention-badge-text-size`      | `--font-size-xs` (12 px) | Tabular numerics for "48h" / "18h" / "<1d"           |
| `--retention-badge-gap`            | `4px`      | Between icon and text                                            |
| `--avatar-size-single`             | `16px`     | Engineering default; single-VOD avatar in strip Line 1 Col 1     |
| `--avatar-dot-size`                | `12px`     | Engineering default; smaller than single avatar (signals "row")  |
| `--avatar-dot-gap`                 | `2px`      | Engineering default; tight grouping reads as set                 |
| `--avatar-dot-row-max`             | `4`        | ADR-0039 DV4 binding (then "+N")                                 |
| `--progress-stripe-height`         | `3px`      | Q-10 binding; "order of magnitude 3 px" final-pinned here        |
| `--focus-outline-width`            | `2px`      | H-20 binding (≤ 2 px)                                            |
| `--focus-outline-offset`           | `2px`      | Space between card edge and outline                              |
| `--selection-outline-width`        | `2px`      | Same as focus (single visual per ADR-0038 D7)                    |
| `--silhouette-offset`              | `2px`      | ADR-0039 DV4 binding ("~ 2 px" final-pinned here)                |
| `--silhouette-layer-count`         | `2`        | ADR-0039 DV4 binding (2 silhouette layers behind frontmost)      |
| `--fan-out-gap-ratio`              | `0.25`     | ADR-0039 DV4 binding (card-width × 0.25)                         |
| `--stack-marker-padding`           | `8px`      | Engineering default; "+K more" plate inner-padding               |

**Composite size computations.**

- Retention badge total width (Stage 3 / 4): `12 (icon) + 4 (gap) + ~24 (text "18h" or "<1d") + 12 (2×padding-x) = ~52 px`
- Retention badge area: `52 × 20 = 1 040 px²`
- Selection checkmark area: `16 × 16 = 256 px²`
- Progress stripe at 50 % on 96-px image: `48 × 3 = 144 px²`
- Worst-case accent-per-card sum: `256 + 1 040 + 144 = 1 440 px²` (≈ 10 % of a 200 × 72 Full card — see TV1 viewport-budget verification)

**Position-map (no-collision invariant).**

| Position on image area  | Element                                | Z-order (TV7)        |
| ----------------------- | -------------------------------------- | -------------------- |
| Top-right corner        | Selection checkmark (when selected)    | `--z-card-hover`     |
| Bottom-right corner     | Retention stage-3/4 badge              | `--z-card-hover`     |
| Bottom edge (full-width) | 3-px progress stripe                  | `--z-card-hover`     |
| Top-left corner         | (Reserved — content-type badge future) | n/a                  |

Diagonal placement of selection (top-right) and retention badge
(bottom-right) guarantees no overlap. The 3-px stripe at bottom-edge
runs below the retention badge's vertical extent — verify: badge
sits at `bottom: 4 px` from image edge (with 4 px padding) and is
20 px tall, occupying y = 4..24 from the bottom. Stripe sits at
`bottom: 0` and is 3 px tall, occupying y = 0..3. No overlap.

**Rationale.** Sizes pinned at the smallest legible values to maximise
card-image dominance (H-16); position-map enforces diagonal corner
discipline so no two overlays collide regardless of state combinations.
The "+N more" stack-marker padding at 8 px matches the `--space-2`
gutter rhythm.

### TV6 — Motion System Constants

**Decision.** Per-purpose motion tokens, all in the H-21 180–240 ms
zoom range or the H-21 modal-open 200–280 ms range, with a strong
`ease-out` decelerating default and a complete reduced-motion
override block.

**Motion constants.**

| Token                                 | Value                                | Source / Use                                            |
| ------------------------------------- | ------------------------------------ | ------------------------------------------------------- |
| `--motion-hover-delay`                | `800ms`                              | ADR-0039 DV3 binding (H-18 midpoint of 0.6–1.0 s)      |
| `--motion-zoom-duration`              | `200ms`                              | Mid of H-21 180–240 ms band (card scale on hover-trigger) |
| `--motion-crossfade-duration`         | `250ms`                              | Anchor §4 binding (thumbnail ↔ frame-strip)             |
| `--motion-panel-slide-duration`       | `200ms`                              | Mid of H-21 180–240 ms band (expanded-metadata-panel)   |
| `--motion-frame-cadence`              | `400ms`                              | ADR-0039 DV3 binding (v2.0.x precedent)                 |
| `--motion-easing-out`                 | `cubic-bezier(0.16, 1, 0.3, 1)`      | Anchor §4 "decelerating curve" — strong start, gentle landing |
| `--motion-easing-in-out`              | `cubic-bezier(0.4, 0, 0.6, 1)`       | Sparingly used; modal-open only                         |
| `--motion-fade-instant`               | `100ms`                              | H-22 binding (reduced-motion fallback ≤ 100 ms)         |
| `--motion-modal-open-duration`        | `220ms`                              | Mid of §4 200–280 ms band                               |
| `--motion-sidebar-reveal-duration`    | `200ms`                              | Q-1 sidebar avatar-column → expanded reveal             |
| `--motion-toolbar-fade-duration`      | `180ms`                              | Selection-toolbar entry / exit                          |

**Cycle math — Hover trigger total time.**

```
t = 0      : pointer enters card / focus arrives
t = 800 ms : delay expires → entry animation begins
            ├─ card scale 1.0 → 1.10 over 200 ms (zoom-duration)
            ├─ expanded panel slide-in over 200 ms (panel-slide-duration)
            └─ crossfade thumbnail → frame-strip over 250 ms
t ≈ 1050 ms: full preview state (frame-strip looping at 400-ms / frame)
```

**Reduced-motion override block.** When `prefers-reduced-motion: reduce`
is active, the following overrides apply (all motion timings collapse
to instant or near-instant):

| Token (overridden)                  | Reduced-motion value          | Effect                                          |
| ----------------------------------- | ----------------------------- | ----------------------------------------------- |
| `--motion-hover-delay`              | `0ms`                         | No delay; expanded panel appears instantly      |
| `--motion-zoom-duration`            | `0ms`                         | No card-scale animation                         |
| `--motion-crossfade-duration`       | `0ms`                         | No crossfade (frame-strip never starts)         |
| `--motion-panel-slide-duration`     | `0ms`                         | Panel appears instantly                         |
| `--motion-frame-cadence`            | `0ms`                         | No loop                                         |
| `--motion-modal-open-duration`      | `100ms`                       | Quick fade-in (anchor §4: ≤ 100 ms opacity)     |
| `--motion-sidebar-reveal-duration`  | `0ms`                         | Sidebar reveals instantly                       |
| `--motion-toolbar-fade-duration`    | `0ms`                         | Toolbar appears instantly                       |
| `--card-hover-zoom-factor`          | `1.0`                         | No zoom (DV3 reduced-motion path)               |

Implementation pattern (Tailwind v4):

```css
@media (prefers-reduced-motion: reduce) {
  :root {
    --motion-hover-delay: 0ms;
    /* ... */
    --card-hover-zoom-factor: 1.0;
  }
}
```

This is the chosen pattern over a dual-variable scheme
(`--motion-zoom-on` / `--motion-zoom-off`) because: (a) the @media-
override is the conventional CSS-Custom-Property pattern; (b) it
keeps consumer-component code identical (`transition-duration:
var(--motion-zoom-duration)` works for both states); (c) it puts
the reduced-motion decision in a single place (globals.css), not
scattered across component CSS.

**Rationale.** All durations land squarely inside the anchor §4
ranges, with the zoom + panel-slide picked at the mid of 180–240 ms
to feel decisive without rushing. The easing function
`cubic-bezier(0.16, 1, 0.3, 1)` is a strong decelerating curve
(commonly known as "ease-out-expo-like") that matches anchor §4's
"strong start and gentle landing". Reduced-motion overrides collapse
everything to instant except the modal-open fade (kept at 100 ms
per H-22 explicit allowance for opacity-only fades ≤ 100 ms).

### TV7 — Elevation System

**Decision.** No card shadow in any state (H-15 binding; depth comes
from surface-ladder). Single modal shadow at most 4 px blur (anchor
§2.4 binding). Explicit z-index stack to make layering predictable
across components.

**Shadows.**

| Token             | Value                                  | Use                                          |
| ----------------- | -------------------------------------- | -------------------------------------------- |
| `--shadow-card`   | `none`                                 | Cards in idle and hover (H-15 binding)       |
| `--shadow-modal`  | `0 2px 4px rgba(0, 0, 0, 0.3)`         | Detail overlay + modal panels (anchor §2.4 unobtrusive) |
| `--shadow-toolbar` | `0 -2px 4px rgba(0, 0, 0, 0.3)`       | Floating selection-toolbar (drop shadow upward) |

§2.4 binds shadows to ≤ 4 px blur at low opacity. The previous
`--shadow-lg: 0 12px 36px rgb(0 0 0 / 0.5)` violated this (12 px
blur, 50 % opacity); v2.1 reduces to a single 4-px-blur shadow used
sparingly. Cards explicitly carry **no shadow** in either idle or
hover state (anchor §2.4: "Hover does not elevate via shadow. Hover
lifts via scale, slight metadata reveal").

**Z-index stack.**

| Token                  | Value | Layer                                              |
| ---------------------- | ----- | -------------------------------------------------- |
| `--z-base`             | `0`   | Page background, default content                   |
| `--z-axis`             | `5`   | Time-axis line, lane backgrounds                   |
| `--z-card-idle`        | `10`  | Card silhouettes in idle state                     |
| `--z-card-hover`       | `20`  | Hover-triggered card (above neighbours)            |
| `--z-fan-out`          | `30`  | Stack-card fanned-out members                      |
| `--z-now-indicator`    | `35`  | Now-indicator line (above cards, below overlays)   |
| `--z-selection-bar`    | `40`  | Floating selection toolbar (bottom of viewport)    |
| `--z-tooltip`          | `45`  | Future tooltip layer (reserved)                    |
| `--z-scrim`            | `49`  | Modal backdrop                                     |
| `--z-modal`            | `50`  | Detail overlay, settings modal                     |
| `--z-toast`            | `60`  | Future toast notifications (reserved)              |

Step size 5–10 keeps the ladder readable. The Now-indicator at 35
sits above all cards but below selection-bar / modal — when a modal
opens, the Now-indicator (a time-axis decoration) is hidden under
the scrim, which is the correct behaviour. Fan-out cards at 30 sit
above the hover-triggered card at 20 so the fan-out arc reads as
"opening out *from* the frontmost".

**Rationale.** Removing card shadows entirely is the strictest H-15
reading; the surface-ladder (`--color-surface-1/2/3` against bg)
carries all depth. A single modal shadow at 4 px blur is the
minimum needed for corner-rounding readability (anchor §2.4
purpose). Z-stack designed so any one consumer can pick the right
layer without consulting other components.

### TV8 — Now-Indicator Visual

**Decision.** Pins the ADR-0038 D3 forward-reference to concrete
token values: 1-px stroke, neutral white at 30 % opacity, no accent
fill, "Jump to now" CTA in a neutral pill.

**Now-indicator tokens.**

| Token                         | Value                          | Source / Use                              |
| ----------------------------- | ------------------------------ | ----------------------------------------- |
| `--now-indicator-color`       | `rgba(255, 255, 255, 0.30)`    | ADR-0038 D3 binding (~30 % neutral white) |
| `--now-indicator-width`       | `1px`                          | ADR-0038 D3 binding                       |
| `--now-indicator-z`           | `var(--z-now-indicator)` (35)  | TV7 z-stack                               |

**"Jump to now" CTA visual.**

Neutral pill button surfacing in the zoom chrome (top-right corner
of TimelinePage), only when the current viewport position has the
Now-instant out of range:

- Background: `--color-surface-2` at idle
- Border: none (H-14)
- Text: `--color-fg`, `--font-size-xs` (12 px), `--font-weight-mid` (500)
- Icon: a small `arrow-right` or `clock` glyph (TV11 icon-system),
  size `--icon-size-sm` (14 px), stroke `--icon-stroke-width` (1.5),
  colour `currentColor`
- Padding: `--space-2` horizontal × `--space-1` vertical
- Corner radius: `--card-corner-radius` (4 px)
- Hover state: background brightens to `--color-surface-3`
- Focus: `--focus-outline-width` outline in `--color-outline-focus`

The button is **never accent-painted** (no `--color-accent` fill);
it's neutral chrome per anchor §5 anti-pattern ("No permanent
top-toolbar painted in accent colour") generalised to all chrome
buttons that are not primary CTAs.

**Rationale.** Now-indicator stays neutral so it doesn't consume
the accent budget (§7 / Q-10 reserves the accent for the progress
stripe as the only continuous-accent surface; the Now-indicator
would compound coverage if accent-coloured). The "Jump to now" CTA
is functional chrome, not a primary action — neutral pill is correct.

### TV9 — Sidebar Numerics (Q-1)

**Decision.** Sidebar collapsed-width 56 px (binding per **TV-A4**,
CEO Decisions §below). Expanded-width 240 px. Hover-reveal default
trigger with optional keyboard-pin. Avatar composition: profile-image
with accent dot inside ring when the streamer is **favourited AND
live** (per ADR-0038 D9).

**Sidebar tokens.**

| Token                              | Value                  | Source / Use                                    |
| ---------------------------------- | ---------------------- | ----------------------------------------------- |
| `--sidebar-collapsed-width`        | `56px`                 | TV-A4 binding                                   |
| `--sidebar-expanded-width`         | `240px`                | Wide enough for streamer name + filter chip     |
| `--sidebar-reveal-duration`        | `var(--motion-sidebar-reveal-duration)` (200 ms) | Q-1 reveal animation |
| `--sidebar-avatar-size`            | `32px`                 | Larger than card avatar (16-px) for legibility  |
| `--sidebar-avatar-padding`         | `12px`                 | (56 - 32) / 2 = 12 px each side                 |
| `--sidebar-live-dot-size`          | `8px`                  | Small accent dot, top-right of avatar           |
| `--sidebar-live-dot-offset`        | `2px`                  | Inset from avatar circle edge                   |

**Width math.** 56 = 32 (avatar) + 12 × 2 (padding). The 32-px avatar
size is the smallest at which a streamer's face is recognisable on
typical monitors at typical reading distance. Alternative collapsed
widths considered in TV-A4 alternatives.

**Reveal-trigger.**

- **Default trigger:** pointer-hover sustained 100 ms over any pixel
  in the collapsed sidebar column. (Quicker than card-hover-delay
  because the reveal is a navigation affordance, not a preview.)
- **Pin trigger:** click on a small "pin" icon at the bottom of the
  expanded sidebar fixes it open; click again unpins. Pin state lives
  in `nav-store` or a new `sidebar-store` — Wave-4 engineering decides.
- **Keyboard trigger:** Tab-focus to any avatar reveals (focus enters
  the avatar; reveal expands the column); Shift-Tab off the last
  filter-chip retracts.

**Tab-order interaction (DV9 of ADR-0039 compatible).** When the
sidebar reveals, focus order goes: top-bar → sidebar avatars (top-to-
bottom) → main content (TimelineLanes → cards → SelectionBar). When
the sidebar is collapsed, Tab still visits avatar-only buttons in the
collapsed column.

**Avatar composition.**

| State                                       | Visual                                                          |
| ------------------------------------------- | --------------------------------------------------------------- |
| Streamer not favourited, not live           | Plain profile-image, no decoration                              |
| Streamer favourited, not live               | Plain profile-image + small star-icon overlay (TV11 icon)       |
| Streamer not favourited, live               | Profile-image + small accent dot top-right (live indicator)     |
| Streamer favourited AND live (Hero-trigger) | Profile-image + accent dot + subtle ring around avatar          |

The accent-dot live-indicator is **binding per TV-A7** (CEO Decisions
§below): `--color-accent` (`#d4a14a`) inside a 2-px-inset on the
top-right of the avatar circle. Anchor-compliant (H-07 discrete
marker). Alternative neutral white dot considered and rejected.

**Rationale.** 56 px collapsed is tight but functional (32-px avatar
is recognisable). 240 px expanded is the conventional sidebar width
for a streamer-name + filter-chip layout. Hover-reveal at 100 ms is
quick enough that the user feels in control without accidental
triggering on cursor-pass-through. The pin option respects power-users
who keep the sidebar open during long browsing sessions.

### TV10 — Hero-Slot Numerics (Q-2 + ADR-0038 D9 + CEO-A3)

**Decision.** Hero-slot height 280 px when active (binding per
**TV-A5**, CEO Decisions §below). Total collapse (height: 0) when no
favourited streamer is live (per ADR-0038 D9). Hero-card aspect
inherits 16:9. Layout-reservation trigger is favourited-AND-live
(binding from ADR-0038 D9). When multiple favourited streamers are
live, the Hero-slot selects one per **TV-A6** (binding:
most-recently-started).

**Hero-slot tokens.**

| Token                            | Value                          | Source / Use                                          |
| -------------------------------- | ------------------------------ | ----------------------------------------------------- |
| `--hero-slot-height`             | `280px`                        | TV-A5 binding; 0 when no favourited live              |
| `--hero-slot-padding-v`          | `var(--space-12)` (48 px)      | Anchor §2.3 generous around hero (≥ 48 px)            |
| `--hero-slot-padding-h`          | `var(--space-8)` (32 px)       | Anchor §2.3 ≥ 32 px from edge                         |
| `--hero-card-aspect`             | `16 / 9`                       | Inherits from `--card-aspect`                         |
| `--hero-cta-padding-v`           | `var(--space-3)` (12 px)       | CTA pill button padding                               |
| `--hero-cta-padding-h`           | `var(--space-6)` (24 px)       | CTA pill button padding                               |
| `--hero-title-size`              | `var(--font-size-hero)` (32 px) | Hero title — H-09 ≥ 2× card title                    |
| `--hero-pitch-size`              | `var(--font-size-base)` (16 px) | Pitch copy                                           |

**Layout-reservation behaviour (ADR-0038 D9 + CEO-A3 binding).**

- `favouritedStreamerLive === true` → `--hero-slot-height` resolved
  to 280 px, slot renders with hero content.
- `favouritedStreamerLive === false` → `--hero-slot-height` resolved
  to 0, slot collapsed entirely (not just `display: none`; the DOM
  node either unmounts or has height: 0 with no padding). N_max-per-
  side recomputes against the larger lane-area.

This is **total collapse**, not visibility-toggle with reserved
height. The ADR-0038 D9 trade-off explicitly accepts the one-time
layout shift when a non-favourited live transition happens.

**Hero variant composition (forward-completing ADR-0039 DV1 Hero variant).**

The VodCard rendered inside `TimelineHeroSlot.tsx` uses a `variant="hero"`
composition that differs from Full / Compact / Sliver:

| Element                | Hero variant                                                                  |
| ---------------------- | ----------------------------------------------------------------------------- |
| Card height            | `--hero-slot-height` - padding (≈ 184 px after vertical padding)              |
| Card width             | Spans hero-slot horizontal interior (viewport_w - 2× hero-padding-h)          |
| Image area             | 16:9 of card-height, no width-cap (image extends right to card edge)          |
| Title                  | `--font-size-hero` (32 px) / weight 700 — overlaid bottom-left                |
| Pitch copy             | 1–2 lines `--font-size-base` (16 px), below title                             |
| Primary CTA            | "Watch live" — `--color-accent` fill, `--color-accent-fg` text, `--font-size-2xl` (24 px) / weight ≥ 500, `--hero-cta-padding-*`. The 24-px size is binding: it places the CTA in the APCA large-fluent-text band where the Lc 58 accent contrast is acceptable (see § Risks #1). |
| Secondary CTA          | "More info" — neutral pill, same padding                                      |
| Hover behaviour        | No frame-strip preview (hero already shows live thumbnail); only CTA luminance bump |
| Metadata strip         | Expanded at idle (no hover gate); shows streamer name + duration + viewers   |

The primary "Watch live" CTA is the **one anchor-permitted accent
fill per view** (H-07: "at most one primary CTA per view"). This is
the only continuous-accent surface in the hero region.

**Tie-break for multiple favourited-live streamers (TV-A6 binding).**
**Most-recently-started** (descending by
`twitch_stream.started_at` from `getStreamerLiveStatus` IPC). Once
a favourited stream is selected as the hero, it remains until the
stream ends or another favourited stream starts more recently. Uses
existing `streamers.last_live_at` from DB schema (no new IPC).
Matches the "Happening now" mental model. Alternatives
(most-watched-by-user, user-pinned, random-rotation) considered and
rejected — see TV-A6 alternatives.

**Rationale.** 280 px is just over a third of a typical 880-px
viewport, enough to feel "billboard-y" (H-04: hero ≥ 60 % at scroll
position 0, but Sightline's hero is conditional and smaller-than-
Netflix) without consuming so much that the timeline loses lane
density. CEO-A3's total-collapse semantic prevents persistent
layout-shift cost; the trade-off (one-time shift on non-favourited
live transition) is accepted at the ADR-0038 level. Tie-break by
recency surfaces "the freshest thing happening right now" — the
mental model that matches "Happening now" hero copy.

### TV11 — Icon System

**Decision.** **Install `lucide-react`** in Wave-4 engineering
(currently no icon library is in `package.json`). Lucide-react is
an outline-glyph SVG icon library, monochrome-by-default, anchor-
conformant per H-23 ("Chrome icons are monochrome outline glyphs").
Four icon sizes; stroke-width 1.5; colour inherits via `currentColor`.

**Icon tokens.**

| Token                  | Value      | Use                                              |
| ---------------------- | ---------- | ------------------------------------------------ |
| `--icon-size-sm`       | `14px`     | Inline badge icons (retention clock-icon)        |
| `--icon-size-base`     | `16px`     | Default chrome icons (top-bar, sidebar pin)      |
| `--icon-size-lg`       | `20px`     | Section labels, modal close                      |
| `--icon-size-xl`       | `24px`     | Hero CTA icon (play, etc.)                       |
| `--icon-stroke-width`  | `1.5`      | Lucide default; matches anchor outline glyph     |
| `--icon-color`         | `currentColor` | Inherits from text-colour context (H-23)     |

**Inventory of glyphs needed for Wave 4.**

| Icon             | Library import (Lucide name)  | Used in                                                      |
| ---------------- | ----------------------------- | ------------------------------------------------------------ |
| Clock            | `Clock`                       | Retention badge (Stage 3 / 4)                                |
| Check            | `Check`                       | Selection checkmark; "watched" glyph (VOd-A3)                |
| Star             | `Star`                        | Favourited streamer marker in sidebar                        |
| Live-dot         | `Circle` (filled)             | Live-state indicator on avatar                               |
| Play             | `Play`                        | Hero CTA, card hover-action                                  |
| Pause            | `Pause`                       | Player controls                                              |
| Pin              | `Pin`                         | Sidebar pin toggle                                           |
| ChevronRight     | `ChevronRight`                | "Jump to now" CTA                                            |
| Search           | `Search`                      | Top-bar search                                               |
| Settings         | `Settings`                    | Top-bar gear                                                 |
| Close (X)        | `X`                           | Modal dismiss, selection bar "Clear"                         |
| ArrowDown        | `ArrowDown`                   | Download action                                              |
| MoreHorizontal   | `MoreHorizontal`              | Stack-marker "+K more" icon (alongside text)                 |

The Wave-4 implementation imports these by name; tree-shaking under
Vite + `lucide-react` will produce a minimal bundle (~2 kB per icon
used).

**Alternative considered (briefly).** Continue with the v2.0.x
Unicode-glyph approach (✓ ↻ ▶ × ↓). Rejected because:
- Glyph rendering is OS-dependent (a "✓" looks different on macOS
  vs. Windows vs. Linux); icon library guarantees identical
  pixel-perfect rendering.
- Stroke width inconsistency (some Unicode glyphs are filled, some
  outlined); Lucide is uniformly outline + uniform stroke.
- Icon size control via Unicode is brittle (font-size adjustment
  changes glyph relative size).
- No incremental cost — `lucide-react` is ~50 kB, less than 1 % of
  the app bundle.

**Rationale.** H-23 binds chrome icons to monochrome outline glyphs;
Lucide is the smallest, most-standard library matching this
discipline in a React + Tailwind ecosystem. The four-size ladder
covers every documented icon use; `currentColor` inheritance keeps
icons in sync with text-colour context.

### TV12 — Token Implementation and Migration Spec

**Decision.** All tokens live in `src/styles/globals.css` inside the
existing `@theme {}` block (Tailwind v4 native pattern). Reduced-motion
overrides live in an `@layer base` block under the same file.
Migration is **value-swap and structure-extend only** in Wave-4
engineering — no Tailwind config file is added (v4 doesn't use one),
no per-component CSS file is created (utility classes only).

**Token naming conventions (preserved from v2.0.x where possible).**

| Prefix         | Domain                                | Example                              |
| -------------- | ------------------------------------- | ------------------------------------ |
| `--color-*`    | Hex / RGBA colour values              | `--color-accent`, `--color-bg`       |
| `--space-*`    | Pixel / rem spacing values            | `--space-2`, `--space-12`            |
| `--font-*`     | Typography (size, weight, family)     | `--font-size-xs`, `--font-weight-body` |
| `--line-height-*` | Line-height values                 | `--line-height-tight`                |
| `--card-*`     | Card geometry                         | `--card-lane-height`, `--card-image-share` |
| `--motion-*`   | Durations + easings                   | `--motion-zoom-duration`             |
| `--z-*`        | Z-index ladder                        | `--z-card-hover`, `--z-modal`        |
| `--radius-*`   | Border-radius values (general utility) | `--radius-sm`                       |
| `--shadow-*`   | Box-shadow values                     | `--shadow-modal`                     |
| `--icon-*`     | Icon system tokens                    | `--icon-size-base`                   |
| `--sidebar-*`  | Sidebar geometry                      | `--sidebar-collapsed-width`          |
| `--hero-*`     | Hero-slot geometry                    | `--hero-slot-height`                 |

**Tailwind v4 binding.** Tailwind v4 reads tokens defined in `@theme {}`
and automatically generates utility classes:
- `--color-accent: #d4a14a` → `bg-accent`, `text-accent`, `border-accent`
- `--space-2: 0.5rem` → `p-2`, `gap-2`, etc. (8 px)
- `--font-size-xs: 0.75rem` → `text-xs`
- Custom prefixes (`--card-*`, `--icon-*`, `--sidebar-*`, `--hero-*`)
  generate utility classes per Tailwind v4's `@theme` discovery (e.g.,
  `--card-lane-height: 72px` generates `h-card-lane-height` or
  similar — exact utility-name pattern depends on Tailwind v4's
  conventions, to be verified in Wave-4).

No `tailwind.config.ts` / `.js` file is added. The project remains
Tailwind v4 native (current state).

**Complete Wave-4 migration list (token-layer changes only).**

| Operation           | Old value                              | New value                                  |
| ------------------- | -------------------------------------- | ------------------------------------------ |
| REPLACE             | `--color-accent: #7c3aed`              | `--color-accent: #d4a14a`                  |
| REPLACE             | `--color-accent-fg: #ffffff`           | `--color-accent-fg: #0a0a0d`               |
| REPLACE             | `--color-focus-ring: #8b5cf6`          | `--color-outline-focus: rgba(255,255,255,0.50)` (rename + recolour) |
| REPLACE             | `--color-surface: #14141a`             | `--color-surface-1: #14141a` (rename)      |
| REPLACE             | `--color-surface-elevated: #1c1c24`    | `--color-surface-2: #1c1c24` (rename)      |
| REPLACE             | `--color-surface-interactive: #242430` | `--color-surface-3: #242430` (rename)      |
| REPLACE             | `--motion-fast: 120ms ...`             | Split into per-purpose `--motion-*-duration` tokens (TV6) |
| REPLACE             | `--motion-base: 200ms ...`             | Replaced by `--motion-zoom-duration` etc.  |
| REPLACE             | `--motion-slow: 320ms ...`             | (Not directly mapped — slow durations now context-specific) |
| REPLACE             | `--radius-md: 0.5rem`                  | (Kept as utility radius) + new `--card-corner-radius: 4px` |
| REPLACE             | `--shadow-sm/md/lg`                    | Reduced to single `--shadow-modal: 0 2px 4px rgba(0,0,0,0.3)` |
| REMOVE              | `--color-subtle: #71717a`              | (deleted; H-11 two-step ramp only)         |
| REMOVE              | `@media (prefers-color-scheme: light) { :root { ... } }` block | (deleted; dark-only) |
| REMOVE              | `color-scheme: dark light;`            | Replace with `color-scheme: dark;`         |
| REMOVE              | `@keyframes sightline-shimmer`         | (deleted; anti-pattern §5)                 |
| ADD                 | (none)                                 | Full set of new tokens per TV1–TV11        |

**Hard-coded-hex audit (Wave-4 engineering task, scoped here).** A
fresh `grep -rn "#" src/ --include="*.tsx" --include="*.ts"` reveals
which components use literal hex values instead of tokens. Those
components need individual migration to the new token system. Known
candidates from v2.0.x include:

- `src/features/vods/VodCard.tsx` (v2.0.1 component, fully rewritten in Wave 4 against ADR-0039)
- `src/features/timeline/TimelinePage.tsx` (Phase-4 proof-of-concept, fully rewritten in Wave 4 against ADR-0038)
- Any `bg-blue-500`, `text-yellow-400`, etc. Tailwind utility classes (status badges in VodCard.tsx) — these get migrated to `bg-info` / `text-warning` etc. utilities

The complete audit is Wave-4 engineering scope; this ADR commits to
the tokens being correct and migration-ready.

**Test strategy hints for Wave 4.**

- **Token-snapshot test**: a Vitest test imports `globals.css` and
  asserts the `:root` computed-style contains the expected tokens
  at the expected values. Catches accidental drift.
- **APCA contrast test**: a Vitest test computes APCA contrast for
  the principal text/bg combinations (`--color-fg` on `--color-bg`,
  `--color-muted` on `--color-surface-1`, `--color-accent` on
  `--color-accent-fg`, etc.) and asserts they exceed Lc 75 (body
  text) or Lc 60 (non-text glyphs).
- **Reduced-motion override test**: a Vitest test simulates
  `prefers-reduced-motion: reduce` and asserts `--motion-zoom-duration`
  resolves to `0ms`.
- **Visual regression test** (Playwright, gated as in ADR-0038):
  snapshot every card state from ADR-0039 DV7 state matrix; assert
  pixel-perfect equality before / after Wave-4 migration.

**Rationale.** The token-naming convention preserves v2.0.x prefixes
where possible (lower migration friction); new prefixes (`--card-*`,
`--sidebar-*`, `--hero-*`, `--z-*`, `--icon-*`) follow the same
pattern. Tailwind v4 native pattern is preserved (no new config
file). The migration list is explicit and limited to token-layer
edits — components consume tokens via variables or utility classes,
so a one-place edit in `globals.css` propagates everywhere correctly.

---

## Consequences

### Positive

1. **All forward-references from ADR-0038 + ADR-0039 are now
   resolved.** Wave-4 engineering can implement against concrete
   token values without re-litigating any visual decision.
2. **`#d4a14a` Locked Input is installed** as the sole brand accent;
   all anchor heuristics around accent restraint apply directly.
3. **Anchor compliance verified at the token level.** Surface ladder
   matches §2 L\* steps; type scale matches H-09 weight + size
   discipline; motion durations land inside §4 ranges; shadows
   collapse to §2.4 maximum; H-25 not constrained at the token
   layer (token layer is page-agnostic).
4. **H-06 coverage math is positive at viewport level**, verified
   by explicit calculation (~1.72 % viewport accent under realistic
   conditions). Per-card peak coverage (~9.2 %) documented as
   intentional design cost.
5. **No backend touch, no IPC change, no schema migration.** Token
   migration is `globals.css`-scoped plus per-component utility-class
   updates in Wave 4.
6. **Tailwind v4 native preserved.** No config file added; existing
   `@theme {}` pattern extended.
7. **Reduced-motion override is centralised** in `globals.css`,
   avoiding the v2.0.x pattern of per-component `@media` blocks.
8. **Light-mode block + shimmer keyframe + tertiary grey + lila
   focus-ring are removed**, eliminating anchor-incompatible
   technical debt in one pass.
9. **Lucide-react icon library is named explicitly** so Wave 4
   doesn't have to make the install-decision under deadline pressure.

### Costs accepted

1. **Lane-height = 72 px means smaller thumbnails than typical
   Netflix-card sizes.** GTA-RP-style content (high-action; visible
   from streamer's facecam and overlay UI) remains recognisable at
   96 × 54 px; static-image-heavy content (movies, art) would lose
   information at this density. Sightline is a streamer-VOD app,
   not a movie app — trade-off accepted. See TV-A1 alternatives
   (80 / 96 px) for the values evaluated.
2. **No card shadow in any state** (H-15 strict reading). Some users
   may interpret unshadowed-cards-on-dark-bg as "harder to scan".
   Mitigation: hover-zoom-factor 1.10 and surface-ladder distinction
   carry the depth signal. If real-use feedback shows poor scanning,
   v2.2 ADR can re-introduce a subtle hover-shadow (≤ 4 px blur per
   §2.4) without breaking H-15 (idle remains shadowless).
3. **Lucide-react adds ~50 kB to bundle.** Acceptable trade-off for
   cross-OS icon consistency.
4. **Per-card peak accent coverage at 9.2 %** is above the literal
   per-card 5 % reading. Anchor §1.2 H-06 measures viewport coverage,
   so this is anchor-compliant, but documented explicitly as an
   anti-smell consideration.
5. **Sidebar reveal-trigger at hover-100-ms** may produce accidental
   reveals when cursor passes through the column en-route to
   TimelineLanes. Mitigation: 100 ms is short enough to be tolerable;
   if real-use feedback shows over-triggering, raise to 150–200 ms
   in v2.2.
6. **Hero-slot tie-break by recency (TV-A6 binding)** means the
   "Happening now" hero swaps when a fresher favourited stream
   starts. This is the accepted interpretation of "Happening now";
   alternative semantics documented as rejected — see TV-A6
   alternatives.

### Neutral (forward references and follow-ups)

1. **Token migration is Wave-4 engineering work.** This ADR commits
   the spec; the `globals.css` rewrite + component utility-class
   migration ships in Wave 4.
2. **Hard-coded-hex audit across `src/`** is a Wave-4 engineering
   task. Pre-known candidates listed in TV12; full audit produced
   during Wave 4.
3. **Hero-variant composition skeleton** is sketched in TV10; the
   `TimelineHeroSlot.tsx` component implementation in Wave 4 fills
   in the action-button wiring (Watch live / More info / etc.).
4. **Live-state indicator** uses accent dot as TV-A7 binding;
   if real-use feedback shows confusion with selection-checkmark,
   alternative semantics (neutral white dot) is a single token
   change. See TV-A7 alternatives.
5. **Lucide-react icon imports** are spec'd here; the actual import
   statements + tree-shaking verification live in Wave-4 engineering.
6. **APCA contrast verification** at runtime is a Wave-4 test target.
   This ADR documents the expected Lc values; the test asserts them.
7. **Visual regression baseline** for the Wave-4 implementation is
   established by the Wave-4 test pass, not pre-spec'd here.
8. **Multiview-Pane-Expansion-ADR** consumes tokens from here for
   chrome (transport bar, leader-outline, audio-anchor cue) but pins
   pane-render geometry separately. Tokens flow ADR-0040 → Multiview
   ADR, not the reverse.

### Risks

1. **APCA contrast on accent foreground.** `--color-accent-fg: #0a0a0d`
   on `--color-accent: #d4a14a` yields **Lc ≈ 58** (empirically
   measured in the Wave-4 Mission-1 APCA-test — the earlier Lc ≈ 88
   claim was an over-optimistic ADR-internal estimate; corrected
   2026-05-15, see § Acceptance Lc-correction note and decision-log
   §9). Lc 58 is the **maximum achievable** for the locked-input
   `#d4a14a` accent — no better `--color-accent-fg` value exists
   without violating the Locked-Input. **Mitigation:** the Hero-CTA
   "Watch live" renders at `--font-size-2xl` (24 px) / weight ≥ 500,
   which lands in the APCA **large-fluent-text band** (L\*c ≥ 45) —
   accent fill is acceptable at this size. `--color-fg: #f5f5f7` on
   accent measures Lc ≈ 44 and is likewise below the Lc 75 body-text
   threshold; the Hero-CTA explicitly uses `--color-accent-fg` (the
   dark variant), never `--color-fg`. **Binding constraint:**
   accent-fill surfaces in v2.1 carry only large text (≥ 24 px /
   weight ≥ 500); body-text-size text (< 24 px) on an accent fill is
   excluded. If a future UI surface needs accent-fill with
   body-text-size text, a second accent tone (a lighter variant
   raising contrast above Lc 75) is required — its own ADR wave, not
   a v2.1 token edit.
2. **Tailwind v4 utility generation from custom-prefixed tokens.**
   Tailwind v4's `@theme {}` discovery generates utilities for
   `--color-*`, `--space-*`, `--font-size-*`, `--font-weight-*`,
   `--line-height-*` automatically. For custom prefixes
   (`--card-*`, `--icon-*`, `--sidebar-*`, `--hero-*`), the utility
   names depend on Tailwind v4's conventions, which may evolve.
   Mitigation: Wave-4 engineering verifies utility names against
   the installed Tailwind v4 version; if utilities are missing for
   custom prefixes, components consume the variables directly via
   `var(--card-lane-height)` instead of utility classes.
3. **System-font rendering inconsistencies across OSes.** macOS
   uses `-apple-system` (SF Pro), Windows uses `Segoe UI`, Linux
   uses `Roboto` or `sans-serif`. Visual baseline differs per OS.
   Mitigation: this is anchor §2.2 binding (single sans-serif family
   with system fallback); cross-OS differences are accepted. The
   alternative (custom web-font) breaks anchor §2.2 + load-latency.
4. **Reduced-motion override doesn't cover all anchor-§4 motion.**
   The override block lists motion tokens but doesn't cover
   `--motion-easing-out` (a curve, not a duration). Mitigation:
   easing is meaningless when duration is `0ms`; the override is
   complete for behavioural purposes.
5. **Lucide-react version pinning.** The Wave-4 install must pin a
   specific version to keep icon renderings stable across releases.
   Mitigation: `lucide-react ^0.x` semver pin in `package.json`,
   bumped only on intentional review.
6. **TV-A1 × TV-A5 N_max-Compound-Wechselwirkung.**
   `lane_height_px` (TV-A1 = 72) and `hero-slot-height` (TV-A5 = 280)
   compound in the N_max math; at alternative value pairs the two
   knobs are **not orthogonal**. For the accepted defaults the
   N_max-ladder resolves to **5 / 3** (no-hero / hero-present).
   Hypothetical combinations:
   - `lane=60` + `hero=240` → **6 / 4** (more lanes both states).
   - `lane=80` + `hero=320` → **4 / 2** (fewer lanes, especially
     hero-present where the trade-off compounds).
   Mitigation: documentation here makes the compound dependency
   explicit. If Wave-4 real-use feedback suggests re-tuning, **both
   token values must be evaluated jointly** — single-token tunes can
   produce an N_max asymmetry between the two layout states. A
   future v2.x ADR could revisit both values together based on a
   real-use heatmap of "how often does the hero render" × "how many
   parallel streams do users typically browse".

---

## Alternatives considered

### TV1 (Colour)

- **`--color-bg: #000000`** (true black) — rejected. Anchor §2.1 L\*
  ≤ 12 allows true black, but real-content thumbnails (Twitch streams
  with high-contrast UI overlays) read harsher against pure black
  than against `#0a0a0d`. Engineering judgement.
- **Multi-stop surface gradient between rails** — rejected. Anchor
  §2.1 anti-pattern ("no multi-stop background gradients between
  rails").
- **Desaturated accent variant for UI surfaces** — rejected. Anchor
  H-06 is coverage-based, not saturation-based; a desaturated
  variant would add a second accent and confuse the brand identity.
- **Light-mode tokens as additional set** — rejected. Dark-only is
  binding for v2.1.
- **`rgba(255,255,255,0.65)` vs. solid hex for `--color-muted`** —
  the rgba form is preferred because it composes correctly against
  any surface (surface-1 / surface-2 / surface-3) without per-surface
  re-derivation. The solid `#a1a1aa` works fine on `--color-bg`
  but reads slightly different on `--color-surface-3` (less muted).
- **`--color-info: #3b82f6` left as-is** — kept for Settings forms,
  not used in browse surfaces.

### TV2 (Typography)

- **Custom web-font (e.g., Inter)** — rejected. Anchor §2.2 binding
  + load latency; system-stack is the discipline.
- **3-step text ramp (`--color-subtle: #71717a` kept)** — rejected.
  H-11 binding: two-step ramp only.
- **Five-weight ladder (200 / 300 / 400 / 600 / 800)** — rejected.
  Four weights cover every documented hierarchy; adding more invites
  the kind of weight-soup hierarchy that H-09 explicitly warns against.
- **9-step type scale (10 / 11 / 12 / 13 / 14 / 16 / 18 / 20 / 24 / 32)** —
  rejected. 7-step scale is sufficient; the 10/11/13 sizes invite
  over-granular hierarchy that H-09 + H-11 reject.

### TV3 (Spacing)

- **Loose card-gutter (12 px)** — rejected. Anchor §2.3 binds rail
  gutters to 4–8 px ("tight inside the rail"). 4 px is the tighter
  end, matching Sightline's dense-timeline density.
- **Page-max-width clamp at 1 440 px** — rejected. Anchor §2.3
  binding ("never windowboxes content"). Ultra-wide monitors get
  edge-to-edge timeline with a small bleed margin.

### TV4 (Card geometry)

- **`lane_height_px = 60`** — produces 6/4 N_max ladder matching
  ADR-0038 D4's illustrative numbers exactly, but image at 45 × 80
  px is too small to read GTA-RP content reliably. See TV-A1 alternatives.
- **`lane_height_px = 96`** — image at 72 × 128 reads beautifully
  but N_max drops to 3/2, severely limiting how many overlapping
  streams the user can see at one zoom. See TV-A1 alternatives.
- **Variable lane-height per zoom level** — rejected. Breaks
  H-13 ("consistent across all card variants"). The Sliver-variant
  fallback at narrow widths is the v2.1 answer to information density.
- **Fixed card-width (anchor-by-image, not by-duration)** — rejected.
  Would break ADR-0038 §3.6 time-axis semantics (card width = duration
  × pixels-per-second is the binding contract).
- **Hover zoom factor 1.20 / 1.30** — see TV-A3 alternatives.
  1.20 fits inter-lane spacing budget; 1.30 overlaps neighbouring
  lane by ~5.6 px.

### TV5 (Card elements)

- **Selection checkmark in white** (instead of accent) — rejected.
  ADR-0038 D7 binding ("accent-coloured checkmark badge").
- **Larger retention badge (24 px tall)** — rejected. Anchor §2 +
  H-16 image dominance; 20 px badge is the minimum that's legible.
- **Avatar dots at 16 px** (= single-avatar size) — rejected.
  Smaller dots (12 px) differentiate "this is a row" from "this is
  the single avatar"; user reads the size as semantic.
- **Progress stripe 6 px tall** — rejected. Q-10 binds to "3-px
  order of magnitude"; 6 px would dominate the image bottom edge.

### TV6 (Motion)

- **Hover zoom 300 ms** — outside H-21 180–240 ms range; rejected
  as anchor-noncompliant.
- **Hover zoom 180 ms** — at the tight end; feels slightly rushed
  in informal testing. 200 ms is the safer mid.
- **Frame cadence 300 ms** — quicker, more cinematic; rejected
  because ADR-0039 DV10 binds to 400 ms v2.0.x precedent. Could
  revisit in v2.2.
- **Easing `cubic-bezier(0.25, 0.1, 0.25, 1)`** (CSS `ease`) —
  rejected. Anchor §4 requires "decelerating" curve; standard `ease`
  is mostly-linear-with-slight-decel. `cubic-bezier(0.16, 1, 0.3, 1)`
  is strongly decelerating per §4 spec.
- **Reduced-motion via dual-variable scheme** (`--motion-zoom-on` /
  `--motion-zoom-off`) — rejected. Adds complexity to consumer
  components; @media override is the conventional CSS-Custom-Property
  pattern.

### TV7 (Elevation)

- **Subtle card hover-shadow** (`0 2px 8px rgba(0,0,0,0.2)`) —
  rejected. Anchor §2.4 ("Hover does not elevate via shadow") +
  §5 ("No skeuomorphic depth"). Hover lifts via scale + metadata
  reveal only.
- **Multi-layer modal shadow** (`0 2px 4px ... + 0 8px 16px ...`) —
  rejected. Anchor §2.4 single-shadow at ≤ 4 px blur is the
  discipline.
- **Z-index ladder in steps of 100** — rejected. Steps of 5–10 keep
  the ladder readable in a single screen; steps of 100 invite
  ad-hoc-z-values to fit between layers.

### TV8 (Now-indicator)

- **Accent-coloured Now-indicator** — rejected. ADR-0038 D3 binding
  (neutral white). Accent-coloured would consume accent budget on
  what is a temporal marker, not a content marker.
- **2-px stroke Now-indicator** — rejected. ADR-0038 D3 binding (1 px).
- **"Jump to now" CTA accent-painted** — rejected. Not a primary
  CTA; neutral pill per anchor §5 (no accent on chrome buttons).

### TV9 (Sidebar)

- **Always-expanded sidebar at default** — rejected. Q-1 binding
  (collapsed-default + hover-reveal pattern).
- **Slider-out drawer** (push content right) — rejected. Anchor §6.1
  binds to hover-reveal pattern + ≤ 8 % viewport-width collapsed
  column.
- **Sidebar collapsed-width 48 px** — see TV-A4 alternatives.
  48 = 24 (avatar) + 12 × 2 (padding); 24-px avatar is small. 64 px
  is the alternative at the upper end.

### TV10 (Hero)

- **Hero-slot height 240 px** — TV-A5 alternative. Smaller hero,
  more lane density.
- **Hero-slot height 320 px** — TV-A5 alternative. Larger hero,
  more billboard feel; reduces visible lane area by ~16 %.
- **Hero-slot tie-break by user-pinning** — TV-A6 alternative.
  User explicitly pins one favourited-live streamer as "the hero";
  others surface as smaller "live now" entries. Adds UI complexity.
- **Hero-slot tie-break by most-watched** — TV-A6 alternative.
  Uses `watch_progress.total_watch_seconds` to rank. Stable but
  doesn't surface freshness.

### TV11 (Icon)

- **Continue with Unicode-glyph approach** — rejected (per main
  TV11 discussion). Cross-OS inconsistency + size brittleness +
  fill-vs-outline mismatch.
- **Phosphor Icons** — alternative considered. Slightly larger
  bundle than Lucide; visually similar outline glyphs. Lucide picked
  for tighter Tailwind ecosystem integration.
- **Heroicons** — alternative considered. Anchor-conformant outline
  style. Lucide picked for broader icon coverage (Phosphor and
  Lucide both ship 1 000+ icons; Heroicons ~300).
- **In-house SVG icon set** — rejected. Design / maintenance
  overhead; Lucide is the well-established standard.

### TV12 (Implementation)

- **Add `tailwind.config.ts` with token bindings** — rejected.
  Tailwind v4 doesn't need a config file; the `@theme {}` pattern is
  the v4 native approach. Adding a config file would invite
  config / @theme drift.
- **Per-component CSS modules** — rejected. Project uses Tailwind
  utility classes + CSS variables; per-component CSS modules would
  introduce a third token-consumption pattern.
- **Token migration via codemod (jscodeshift)** — out of scope for
  this ADR; Wave-4 engineering decides whether to use a codemod or
  manual migration based on the hard-coded-hex audit's volume.

---

## Implementation Notes

### Complete CSS-Custom-Property block (binding spec for Wave-4 `globals.css`)

```css
@import "tailwindcss";

@theme {
  /* ============================================================
   * COLOUR SYSTEM (TV1)
   * ============================================================ */

  /* Surfaces — 4-tier ladder (L* 4 → 7 → 10 → 13) */
  --color-bg: #0a0a0d;
  --color-surface-1: #14141a;
  --color-surface-2: #1c1c24;
  --color-surface-3: #242430;
  --color-surface-skeleton: #10101a;
  --color-scrim: rgba(0, 0, 0, 0.60);

  /* Text — H-11 two-step ramp (third grey removed) */
  --color-fg: #f5f5f7;
  --color-muted: rgba(245, 245, 247, 0.65);

  /* Brand accent — #d4a14a Locked Input */
  --color-accent: #d4a14a;
  --color-accent-fg: #0a0a0d;

  /* Status — functional surfaces only (Settings forms, not browse) */
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-danger: #ef4444;
  --color-info: #3b82f6;

  /* Borders — chrome dividers (cards in idle carry NO border per H-14) */
  --color-border: #2a2a34;
  --color-border-strong: #3a3a48;

  /* Outlines — H-20 binding "white (or 80–90 % opacity white)"; 85 % = mid */
  --color-outline-focus: rgba(255, 255, 255, 0.85);
  --color-outline-selection: rgba(255, 255, 255, 0.85);
  --color-outline-hover: rgba(255, 255, 255, 0.30);  /* Sightline-specific helper, outside H-20 */

  /* Live indicator (TV-A7 binding) */
  --color-live: var(--color-accent);

  /* ============================================================
   * TYPOGRAPHY (TV2)
   * ============================================================ */

  --font-stack: ui-sans-serif, system-ui, -apple-system, "Segoe UI",
                Roboto, "Helvetica Neue", Arial, sans-serif;
  --font-stack-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;

  --font-weight-body: 400;
  --font-weight-mid: 500;
  --font-weight-title: 600;
  --font-weight-heading: 700;

  --font-size-xs: 0.75rem;    /* 12 px — meta, badges */
  --font-size-sm: 0.875rem;   /* 14 px — card title, button */
  --font-size-base: 1rem;     /* 16 px — body */
  --font-size-lg: 1.125rem;   /* 18 px — section label */
  --font-size-xl: 1.25rem;    /* 20 px — modal header */
  --font-size-2xl: 1.5rem;    /* 24 px — hero CTA */
  --font-size-hero: 2rem;     /* 32 px — hero title */

  --line-height-tight: 1.2;
  --line-height-body: 1.4;
  --line-height-loose: 1.6;

  /* ============================================================
   * SPACING — 4-px base grid (TV3)
   * ============================================================ */

  --space-0: 0;
  --space-1: 0.25rem;   /*  4 px */
  --space-2: 0.5rem;    /*  8 px */
  --space-3: 0.75rem;   /* 12 px */
  --space-4: 1rem;      /* 16 px */
  --space-5: 1.25rem;   /* 20 px */
  --space-6: 1.5rem;    /* 24 px */
  --space-8: 2rem;      /* 32 px */
  --space-12: 3rem;     /* 48 px */
  --space-16: 4rem;     /* 64 px */

  /* ============================================================
   * CARD GEOMETRY (TV4 — TV-A1 / TV-A2 / TV-A3 binding)
   * ============================================================ */

  --card-lane-height: 72px;
  --card-image-share: 0.75;
  --card-aspect: 16 / 9;
  --card-corner-radius: 4px;
  --card-width-readable-px: 160px;
  --card-width-image-only-px: 96px;
  --card-hover-zoom-factor: 1.10;

  /* ============================================================
   * CARD ELEMENT NUMERICS (TV5)
   * ============================================================ */

  --selection-checkmark-size: 16px;
  --selection-checkmark-padding: 4px;
  --retention-badge-height: 20px;
  --retention-badge-padding-x: 6px;
  --retention-badge-icon-size: 12px;
  --retention-badge-text-size: var(--font-size-xs);
  --retention-badge-gap: 4px;
  --avatar-size-single: 16px;
  --avatar-dot-size: 12px;
  --avatar-dot-gap: 2px;
  --progress-stripe-height: 3px;
  --focus-outline-width: 2px;
  --focus-outline-offset: 2px;
  --selection-outline-width: 2px;
  --silhouette-offset: 2px;
  --fan-out-gap-ratio: 0.25;
  --stack-marker-padding: 8px;

  /* ============================================================
   * MOTION — durations + easing curves (TV6)
   * ============================================================ */

  --motion-hover-delay: 800ms;
  --motion-zoom-duration: 200ms;
  --motion-crossfade-duration: 250ms;
  --motion-panel-slide-duration: 200ms;
  --motion-frame-cadence: 400ms;
  --motion-easing-out: cubic-bezier(0.16, 1, 0.3, 1);
  --motion-easing-in-out: cubic-bezier(0.4, 0, 0.6, 1);
  --motion-fade-instant: 100ms;
  --motion-modal-open-duration: 220ms;
  --motion-sidebar-reveal-duration: 200ms;
  --motion-toolbar-fade-duration: 180ms;

  /* ============================================================
   * ELEVATION (TV7)
   * ============================================================ */

  --shadow-card: none;
  --shadow-modal: 0 2px 4px rgba(0, 0, 0, 0.3);
  --shadow-toolbar: 0 -2px 4px rgba(0, 0, 0, 0.3);

  /* Border-radius utility scale (separate from --card-corner-radius
   * which is the canonical card radius) */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-pill: 9999px;

  /* ============================================================
   * Z-INDEX STACK (TV7)
   * ============================================================ */

  --z-base: 0;
  --z-axis: 5;
  --z-card-idle: 10;
  --z-card-hover: 20;
  --z-fan-out: 30;
  --z-now-indicator: 35;
  --z-selection-bar: 40;
  --z-tooltip: 45;
  --z-scrim: 49;
  --z-modal: 50;
  --z-toast: 60;

  /* ============================================================
   * NOW-INDICATOR (TV8 — ADR-0038 D3 binding)
   * ============================================================ */

  --now-indicator-color: rgba(255, 255, 255, 0.30);
  --now-indicator-width: 1px;

  /* ============================================================
   * SIDEBAR (TV9 — Q-1 binding; TV-A4 binding)
   * ============================================================ */

  --sidebar-collapsed-width: 56px;
  --sidebar-expanded-width: 240px;
  --sidebar-avatar-size: 32px;
  --sidebar-avatar-padding: 12px;
  --sidebar-live-dot-size: 8px;
  --sidebar-live-dot-offset: 2px;

  /* ============================================================
   * HERO-SLOT (TV10 — Q-2 + ADR-0038 D9 binding; TV-A5 / TV-A6 binding)
   * ============================================================ */

  --hero-slot-height: 280px;
  --hero-slot-padding-v: var(--space-12);
  --hero-slot-padding-h: var(--space-8);
  --hero-card-aspect: 16 / 9;
  --hero-cta-padding-v: var(--space-3);
  --hero-cta-padding-h: var(--space-6);
  --hero-title-size: var(--font-size-hero);
  --hero-pitch-size: var(--font-size-base);

  /* ============================================================
   * ICON SYSTEM (TV11 — Lucide-react)
   * ============================================================ */

  --icon-size-sm: 14px;
  --icon-size-base: 16px;
  --icon-size-lg: 20px;
  --icon-size-xl: 24px;
  --icon-stroke-width: 1.5;
}

@layer base {
  :root {
    color-scheme: dark;  /* binding: dark-only in v2.1 */
  }

  /* Reduced-motion override block (TV6) */
  @media (prefers-reduced-motion: reduce) {
    :root {
      --motion-hover-delay: 0ms;
      --motion-zoom-duration: 0ms;
      --motion-crossfade-duration: 0ms;
      --motion-panel-slide-duration: 0ms;
      --motion-frame-cadence: 0ms;
      --motion-modal-open-duration: 100ms;
      --motion-sidebar-reveal-duration: 0ms;
      --motion-toolbar-fade-duration: 0ms;
      --card-hover-zoom-factor: 1.0;
    }
  }

  html,
  body,
  #root {
    height: 100%;
  }

  body {
    font-family: var(--font-stack);
    background-color: var(--color-bg);
    color: var(--color-fg);
    -webkit-font-smoothing: antialiased;
  }

  /* Global focus-visible outline (H-20 binding) */
  :focus-visible {
    outline: var(--focus-outline-width) solid var(--color-outline-focus);
    outline-offset: var(--focus-outline-offset);
    border-radius: var(--card-corner-radius);
  }
}

/* Tabular-numerics utility for duration / retention text */
.font-tabular {
  font-variant-numeric: tabular-nums;
}

/* Keyframe library — no idle pulse / shimmer (anti-pattern §5) */
@keyframes sightline-slide-in {
  from { transform: translateX(24px); opacity: 0; }
  to   { transform: translateX(0);    opacity: 1; }
}

@keyframes sightline-fade-in {
  from { opacity: 0; }
  to   { opacity: 1; }
}

/* sightline-shimmer removed per anti-pattern §5 (no idle pulse) */

.sightline-slide-in {
  animation: sightline-slide-in var(--motion-panel-slide-duration) var(--motion-easing-out) both;
}

.sightline-fade-in {
  animation: sightline-fade-in var(--motion-fade-instant) var(--motion-easing-out) both;
}

@media (prefers-reduced-motion: reduce) {
  .sightline-slide-in,
  .sightline-fade-in {
    animation: none;
  }
}
```

### Tailwind v4 binding notes (Wave 4 reference)

- `@theme {}` discovery generates utility classes for `--color-*`,
  `--space-*`, `--font-size-*`, `--font-weight-*`, `--line-height-*`
  automatically: `bg-accent`, `text-fg`, `text-xs`, `font-body`, etc.
- Custom-prefixed tokens (`--card-*`, `--icon-*`, `--sidebar-*`,
  `--hero-*`, `--motion-*`, `--z-*`) consumed directly via `var(...)`
  in CSS or via arbitrary-value Tailwind utilities like
  `h-[var(--card-lane-height)]`, `text-[--color-fg]`, etc. Tailwind
  v4 may generate convenience utilities (`h-card-lane-height`) — to
  be confirmed in Wave-4 by `pnpm dlx tailwindcss --help` against
  the installed version.

### Token migration summary (Wave-4 work)

| Step                                  | Action                                                |
| ------------------------------------- | ----------------------------------------------------- |
| 1. Replace existing `@theme {}` block | Use the complete CSS block above as canonical content |
| 2. Replace existing `@layer base`     | Use the new dark-only + reduced-motion-override block |
| 3. Remove `sightline-shimmer` keyframe | Delete + audit consuming components for `.shimmer` classes |
| 4. Install `lucide-react`             | `pnpm add lucide-react`                               |
| 5. Audit hard-coded hex in components | `grep -rn "#[0-9a-fA-F]" src/` and migrate to tokens  |
| 6. Replace `bg-blue-500` etc. in `VodCard.tsx` | Use `bg-info`, `bg-warning` utilities          |
| 7. Run token-snapshot test            | Vitest assertion on computed `:root` values           |
| 8. Run APCA contrast test             | Vitest assertion on contrast ratios                   |
| 9. Run visual regression baseline     | Playwright snapshot diff before / after migration     |
| 10. Verify Tailwind utility generation | Check custom-prefix utilities exist or fall back to var() |

---

## CEO Decisions (2026-05-12)

CEO accepted ADR-0040 on 2026-05-12 with seven engineering defaults
adopted as a block, no TV-Decision substance change. Each block
records the original question (for the audit trail), the decision in
one sentence, the rationale (glättet aus CTO-Empfehlung), and the
ADR-Folge that binds the decision into the TV-Sektionen. From this
date the seven decisions below are binding spec for Wave-4
engineering. Full per-decision audit-trail in
`docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §8.

### TV-A1 — `lane_height_px`

- **Original question.** `lane_height_px`: 60 / 72 / 80 / 96 px?
- **Decision.** **`lane_height_px = 72`** is the binding default;
  yields image area 54 × 96 px and N_max-ladder 5 / 3 (no-hero /
  hero-present).
- **Rationale.** GTA-RP action is recognisable at 96 × 54 px image
  area — streamer facecam and overlay UI stay legible. 60 px (image
  80 × 45) drops below the Sightline-use-case recognisability
  threshold; 96 px would beautify thumbnails but reduce N_max-no-hero
  to 3, severely limiting the parallel-streams browse pattern.
  72 px is the balanced trade-off.
- **Folge for ADR.** TV4 Card-Geometry-Tabelle row `--card-lane-height: 72px`
  binding per TV-A1; TV4 derived dimensions (image natural 54 × 96)
  locked; ADR-0038 D4 N_max-Math resolves to 5 / 3 at this value;
  Consequences > Costs accepted #1 references TV-A1 alternatives.

### TV-A2 — Card-width thresholds

- **Original question.** `card-width-readable-px` (Full ↔ Compact)
  and `card-width-image-only-px` (Compact ↔ Sliver)?
- **Decision.** **160 / 96 px** is the binding pair.
- **Rationale.** 96 px = image natural width and the natural kipp-
  point between Compact (image + title strip) and Sliver (image-
  only); below this the card cannot render the image at full size.
  160 px = 1.67× image-width, clean for a two-line strip at the
  Full variant. Alternative pairs (120 / 72 too aggressive Full;
  200 / 120 too conservative) considered and rejected.
- **Folge for ADR.** TV4 Card-Geometry-Tabelle rows
  `--card-width-readable-px: 160px` and `--card-width-image-only-px: 96px`
  binding per TV-A2; TV4 width-variant table at Level-4-zoom-mapping
  locked.

### TV-A3 — `hover-zoom-factor`

- **Original question.** Hover-zoom-factor: 1.08 / 1.10 / 1.12 / 1.20?
- **Decision.** **1.10** is the binding factor within H-17 1.08–1.5×.
- **Rationale.** Geometrically safe distance from the inter-lane
  overlap edge — at lane 72 + inter-lane gap 8 (`--space-2`), 1.10
  produces 7.2 px growth absorbed cleanly; 1.30 would overlap the
  neighbouring lane by ~5.6 px. Netflix-typical 1.20 values are at
  the edge for Sightline's denser lane structure; 1.10 is the safe
  pick without perceptible loss of hover affordance.
- **Folge for ADR.** TV4 Card-Geometry-Tabelle row
  `--card-hover-zoom-factor: 1.10` binding per TV-A3; TV4 hover-zoom
  geometry-implication discussion locked.

### TV-A4 — `sidebar-collapsed-width`

- **Original question.** `sidebar-collapsed-width`: 48 / 56 / 64 px?
- **Decision.** **56 px** is the binding width (= 32-px avatar +
  12 × 2 padding).
- **Rationale.** 32-px avatar is the smallest size at which a
  streamer's face stays recognisable on typical monitors at typical
  reading distance. 48 px (24-px avatar) falls below the
  recognisability threshold; 64 px (40-px avatar) consumes ~5 %
  more viewport-width without a usability payoff.
- **Folge for ADR.** TV9 Sidebar-Tokens-Tabelle row
  `--sidebar-collapsed-width: 56px` binding per TV-A4; TV9 width
  math (`56 = 32 + 12×2`) locked.

### TV-A5 — `hero-slot-height`

- **Original question.** `hero-slot-height`: 240 / 280 / 320 px?
- **Decision.** **280 px** is the binding height when the hero slot
  is active (favourited streamer is live).
- **Rationale.** 31 % of a typical 880-px viewport — billboard-y
  without becoming dominant. Combined with TV-A1 (72) yields
  N_max-hero = 3, an acceptable lane-loss relative to the no-hero
  N_max of 5. 320 px would push N_max-hero to 2 and disproportionately
  constrain the lane-browse use case while the hero is active.
- **Folge for ADR.** TV10 Hero-Slot-Tokens-Tabelle row
  `--hero-slot-height: 280px` binding per TV-A5; new Consequences >
  Risks #6 documents the TV-A1 × TV-A5 N_max compound dependency.

### TV-A6 — Hero-slot tie-break for multiple favourited-live streamers

- **Original question.** Which favourited-live streamer becomes
  "the hero" when more than one is live simultaneously?
- **Decision.** **Most-recently-started** (descending by
  `streamers.last_live_at`) is the binding tie-break.
- **Rationale.** Uses the existing `streamers.last_live_at` field
  from the DB schema — no new IPC endpoint required. Matches the
  "Happening now" mental model: the freshest live stream is the
  most relevant hero candidate. Alternative most-watched-by-user
  would need an IPC aggregation over
  `watch_progress.total_watch_seconds` — an unnecessary Wave-4
  scope expansion for marginal semantic gain. User-pinned and
  random-rotation alternatives also rejected.
- **Folge for ADR.** TV10 Hero-Tie-Break paragraph binding per
  TV-A6; existing `streamers.last_live_at` field reuse documented.

### TV-A7 — Live-state indicator colour

- **Original question.** Accent (`#d4a14a`) or neutral white for
  the live-state dot on streamer avatars?
- **Decision.** **`#d4a14a` accent dot** is the binding colour.
- **Rationale.** Brand touchpoint — Sightline-Honiggold becomes a
  persistent live-state signal in the sidebar. H-07 compliant as an
  identity-bearing discrete marker. Coverage is minimal (64 px² per
  dot × typical 5–10 visible streamers = negligible against the TV1
  5 %-viewport accent budget). Neutral white dot considered and
  rejected (less identity-bearing, weaker brand presence).
- **Folge for ADR.** TV1 `--color-live` binding to `var(--color-accent)`
  per TV-A7; TV9 avatar-composition table row for "favourited live"
  locked; CSS-block comment `/* Live indicator (TV-A7 binding) */`.

---

## Escalation note for CTO (resolved 2026-05-12)

This ADR surfaced **seven** Open Questions (TV-Q1 through TV-Q7).
Per the Welle-3 mission directive, the "≥ 5 CEO-relevante Open
Questions → eskaliere an den CTO"-threshold was reached: 7 > 5.

The Anti-Smell audit (decision-log §6) verified that the seven OQs
were genuine user-feel-prägend choices that the anchor, Q-Decisions,
and upstream ADRs did not directly resolve. Eleven additional
sub-decisions (easing curve, scrim opacity, status-colour set,
focus-outline opacity, icon library choice, etc.) were audited and
documented as engineering-defaults with explicit rationale — no
hidden hedges.

The PROPOSED draft offered the CTO three forward-process options:

1. **Forward as-is.** All seven OQs to CEO as a block.
2. **Pre-process.** CTO picks engineering defaults locally as
   "CTO-approved engineering defaults".
3. **Subdivide.** CTO escalates structural OQs (TV-Q1 / TV-Q5
   with N_max impact) and defaults tuning OQs (TV-Q3 / TV-Q4 /
   TV-Q6 / TV-Q7).

**Resolution (2026-05-12).** The **CTO chose Option 1 (Forward-as-is)**
over Option 3 (Subdivide). Rationale: the tuning-vs-structural
classification proved fuzzy under scrutiny — TV-Q7 is a
brand-touchpoint decision (not tuning), and TV-Q2 is user-feel-prägend
through its compact/sliver-threshold impact; both would have been
mis-classified by the Subdivide pre-split. The block-acceptance
pattern was proven in Welle 1 (4 CEO-Decisions) and Welle 2 (3
CEO-Decisions) and scales linearly with clear per-OQ CTO
recommendations. The Pre-process route would have introduced a
third "CTO-approved engineering defaults" category outside the
existing R-ADR rule binarity (engineering-default-with-rationale
vs. CEO-decision).

The CEO **accepted all seven engineering defaults as a block** —
see CEO Decisions §above (TV-A1 lane=72, TV-A2 thresholds 160/96,
TV-A3 zoom 1.10, TV-A4 sidebar 56, TV-A5 hero 280, TV-A6 tie-break
most-recently-started, TV-A7 live-dot accent). Two **dokumentarische
Pre-Sign-off-CTO-Audit findings** were folded into this acceptance
patch without altering any TV-Decision substance:

- **Counting bug.** The PROPOSED draft counted "six" Open Questions
  while actually listing seven (TV-Q1 through TV-Q7). Patched to
  "seven" throughout.
- **TV-A1 × TV-A5 compound-risk notation.** The two N_max-driving
  knobs are not orthogonal at alternative value pairs; an explicit
  Risk entry was added to Consequences > Risks #6 documenting the
  joint-evaluation requirement for any future re-tune.

**Pattern recorded for future waves.** The Engineering Anti-Smell
audit (pre-delivery) and the CTO Pre-Sign-off audit (pre-sign-off)
are **complementary disciplines, not redundant** — the
Pre-Sign-off-CTO-Audit caught two documentary findings that the
Engineering Anti-Smell audit did not surface due to its
mission-brief-fokus. Both audits will be anchored in a dedicated
R-ADR-rules update as separate rules after this acceptance patch.

---

## References

### Spec sources (binding)

- `docs/reference/visual-language-netflix.md` — H-01..H-25 (all heuristics)
- `docs/reference/visual-language-netflix.md` §2 (Visual System: Color / Typography / Spacing / Elevation)
- `docs/reference/visual-language-netflix.md` §3 (Component patterns: §3.1 Hero / §3.2 Thumbnail Card / §3.3 Hover Preview / §3.4 Detail Overlay / §3.5 Horizontal Rail / §3.6 Single-Time-Axis)
- `docs/reference/visual-language-netflix.md` §4 (Motion System)
- `docs/reference/visual-language-netflix.md` §5 (Anti-Patterns)
- `docs/reference/visual-language-netflix.md` §6 (Sightline Mapping per page)
- `docs/reference/visual-language-netflix.md` §7 Q-1 (sidebar) / Q-2 (hero conditional) / Q-3 (GIF strip) / Q-5 (stack silhouette) / Q-6 (keyboard cascade) / Q-7 (retention) / Q-8 (Multiview equal-quadrants) / Q-9 (storage strip) / Q-10 (3-px progress stripe)
- `docs/reference/visual-language-netflix.md` Locked Input (`#d4a14a` brand accent)
- `docs/decision-log/v2.1-anchor-acceptance.md` (Q-1..Q-10 acceptance audit-trail)
- `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 (CEO-A1..A4 acceptance audit-trail)
- `docs/decision-log/v2.1-adr-0039-vodcard.md` §7 (VOd-A2 / A3 / A5 acceptance audit-trail)

### Related ADRs

- [ADR-0038](0038-timeline-layout-single-time-axis.md) — Timeline-Layout (ACCEPTED 2026-05-12); all D-decisions binding inputs (D2 zoom levels, D3 Now-indicator, D4 lane_height_px central variable, D7 selection-cap-4 + 16-px accent-checkmark, D9 hero favourited-trigger)
- [ADR-0039](0039-vodcard-composition-hover-stack-retention.md) — VodCard (ACCEPTED 2026-05-12); all DV-decisions binding inputs (DV1 three width variants, DV2 idle composition, DV3 800-ms hover delay + cascade, DV4 stack fan-out, DV5 retention escalation, DV6 3-px stripe, DV7 state matrix, DV8 composite hook, DV9 Tab-order, DV10 400-ms frame cadence)
- [ADR-0033](0033-library-ui-redesign.md) — v2.0.1 VodCard (replaced by ADR-0039 successor; token migration eliminates accent-border / accent-focus-outline / accent-fill ✓-badge / 1-px stripe)
- [ADR-0019](0019-asset-protocol-scope.md) + [ADR-0027](0027-asset-protocol-scope-narrowing.md) — Asset protocol (unchanged; tokens do not affect path scope)
- **ADR-Multiview-Pane-Expansion** (parallel v2.1 wave per ADR-0038 CEO-A1) — consumes tokens from this ADR for transport-bar / leader-outline chrome; pane-render geometry pinned separately

### Source files referenced

- `src/styles/globals.css` — token bench to migrate in Wave-4
- `package.json` — Tailwind v4 (`@tailwindcss/vite ^4.0.0`, `tailwindcss ^4.0.0`); `lucide-react` to be added in Wave-4
- `src/features/vods/VodCard.tsx` — v2.0.1 component (replaced by Wave-4 implementation per ADR-0039)
- `src/features/timeline/TimelinePage.tsx` — Phase-4 proof-of-concept (replaced by Wave-4 implementation per ADR-0038)
- `src/features/player/ContinueWatchingRow.tsx` — inner card switches to `<VodCard variant="compact">` in Wave-4
- `src/components/**` — shared primitives consume tokens via Tailwind utilities (utility-class migration is Wave-4 scope)
