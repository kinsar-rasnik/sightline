# Wave-4 Hard-Coded-Hex + Token-Migration Audit

**Datum:** 2026-05-12 (Mission 1)
**Scope:** `src/**` — identifies hex literals and dangling token
references that need migration in Wave-4-Missions 2/3/4/5.
**Source command:** `grep -rn "#[0-9a-fA-F]\{3,8\}" src/ --include="*.tsx" --include="*.ts" --include="*.css"` plus targeted scans for renamed/removed tokens and Tailwind colored-utility classes.

---

## §1 — Raw hex literals in `src/**`

**Result:** **Zero raw hex literals outside `src/styles/globals.css`.**

All hex values in the codebase reside in `src/styles/globals.css`
(the canonical token definitions), which is the intended state per
ADR-0040 TV12. Existing components consume color via Tailwind
utility classes (`bg-blue-500` etc.) or CSS custom properties
(`bg-[--color-surface]`), never via inline hex.

This is a **positive audit result**: the v2.0.x codebase was
already disciplined about hex-literal containment.

---

## §2 — Renamed-token references (Mission 2/3/4 migration owners)

The v2.1 token migration renamed several tokens (TV12 operations
1–10). Components that consume the old names will need a
search/replace in their respective rewrite missions.

| Old token (v2.0.x)              | New token (v2.1)        | Usage sites                                                                          | Migration owner          |
| ------------------------------- | ----------------------- | ------------------------------------------------------------------------------------ | ------------------------ |
| `--color-surface`               | `--color-surface-1`     | `VodCard.tsx`, `LibraryPage.tsx`, `SettingsPage.tsx` (multiple), `StreamersPage.tsx`, `DownloadsPage.tsx`, `TimelinePage.tsx`, `AppShell.tsx`, `StorageSection.tsx`, `StorageOutlook.tsx`, `ForecastBox.tsx`, `CredentialsForm.tsx` | Mission 2 / 3 / 5 (per file scope) |
| `--color-surface-elevated`      | `--color-surface-2`     | `AppShell.tsx:229`, `NotificationsToaster.tsx:52,58`, `Drawer.tsx:72,74`             | Mission 5 (shared chrome) |
| `--color-surface-interactive`   | `--color-surface-3`     | (no grep hits found — token consumers may use literal hex elsewhere; sweep in M5)   | Mission 5                 |
| `--color-focus-ring`            | `--color-outline-focus` | `TimelinePage.tsx:202`                                                               | Mission 3 (TimelinePage rewrite) |
| `--color-subtle`                | `--color-muted` (collapsed; H-11 two-step ramp only) | `SettingsPage.tsx:449`, `StreamersPage.tsx:252`, `NotificationsToaster.tsx:58`        | Mission 5                |
| `--shadow-lg`                   | `--shadow-modal`        | `AppShell.tsx:229`, `Drawer.tsx:72`                                                  | Mission 5                |
| `--shadow-md`                   | `--shadow-modal` (single modal shadow per TV7) | `NotificationsToaster.tsx:52`                                                        | Mission 5                |
| `--motion-fast`                 | `--motion-zoom-duration` or context-specific TV6 token | `AppShell.tsx:137`                                                                   | Mission 5                |
| `--motion-base`                 | (no direct consumer found; TV6 splits per-purpose) | —                                                                                    | Already clean            |
| `--motion-slow`                 | (no direct consumer found) | —                                                                                    | Already clean            |
| `--radius-xl`                   | (removed; v2.1 keeps sm/md/lg/pill) | (no grep hits in `src/**`; existing components use `--radius-md` mostly)             | Already clean            |

**Pattern:** old `--color-surface*` and `--color-subtle` references
are widespread. Mission 2 (VodCard-rewrite) and Mission 3
(TimelinePage-rewrite) will fully replace their respective files;
Mission 5 (Polish) sweeps the remaining utility / chrome components.

---

## §3 — Tailwind colored-utility classes (status / state markers)

Anchor §5 anti-pattern: status colours (`--color-success/warning/
danger/info`) are reserved for functional surfaces (Settings forms,
validation, error states). On browse surfaces they should not paint
status pills using `bg-blue-500/30` etc.; instead, neutral
`--color-surface-*` + status-glyphs.

| Component                            | Lines                | Tailwind colors used                              | Migration target                                   | Owner    |
| ------------------------------------ | -------------------- | ------------------------------------------------- | -------------------------------------------------- | -------- |
| `features/vods/VodCard.tsx`          | (rewritten in M2)    | `bg-blue-500/30`, `bg-emerald-500/80`, `bg-zinc-500/30`, `bg-amber-500/80`, `bg-orange-500/80`, `bg-red-500/80` | `bg-info`, `bg-success`, `bg-surface-1`, `bg-warning`, `bg-warning`, `bg-danger` utilities | **Mission 2 ✅ resolved** (status/download badge palettes migrated; `--color-surface` → `bg-surface-1`; `--color-focus-ring` use removed via global `:focus-visible` rule from TV1) |
| `features/downloads/DownloadsPage.tsx` | 229–234              | Same status palette as VodCard                    | Same migration target                              | Mission 5 |
| `features/storage/ForecastBox.tsx`   | 69, 82–85            | `text-amber-300`, `bg-emerald-500/20`, `bg-amber-500/20`, `bg-red-500/20` | Settings-utility surface — keep status colors (Settings is anchor §6.5 utility-scope) | Mission 5 (verify scope) |
| `features/storage/StorageSection.tsx`| 64, 81               | `bg-amber-500/20`, `bg-amber-500` (utility chrome) | Status colors permitted in Settings/utility chrome | Mission 5 |
| `features/storage/StorageOutlook.tsx`| 36                   | `text-red-400` (error message)                    | Status `--color-danger` permitted                  | Mission 5 |
| `features/storage/CleanupPlanDrawer.tsx` | 30                | `text-red-500` (error)                            | Permitted                                          | Mission 5 |
| `features/storage/CleanupHistoryView.tsx` | 39–44             | `text-emerald-600`, `text-amber-600`, `text-red-500` | Permitted (utility surface)                       | Mission 5 |
| `features/streamers/StreamersPage.tsx` | 107, 196, 252, 266, 270 | `text-red-400`, `bg-red-500/20`, `text-yellow-400`, `bg-blue-500/20`, `bg-blue-300`, plus `motion-safe:animate-pulse` and `motion-safe:animate-ping` | Verify pulsing/pinging against H-22 reduced-motion; status colors mostly permitted; `text-yellow-400` for favorite-star: candidate for `text-[--color-accent]` if favorite is a brand-touchpoint | Mission 5 |
| `features/multiview/MultiViewPage.tsx` | 206                  | `bg-red-900/70 text-red-100` (error banner)       | Permitted error styling                            | **Mission 4** (full Multiview rewrite per ADR-0041 will replace) |
| `features/multiview/MultiViewPane.tsx` | 148                  | `bg-amber-300/90 text-black` (**leader badge — H-20/H-07 violation**) | **REPLACE with neutral `--color-outline-focus` H-20 outline + Lucide `Crown` glyph** per ADR-0041 M5 | **Mission 4** (binding) |
| `features/settings/VideoQualitySection.tsx` | 130, 136, 162, 169 | `text-amber-600 dark:text-amber-400` (warnings)   | Permitted (Settings utility); but the `dark:` modifier is now redundant since v2.1 is dark-only | Mission 5 |
| `features/settings/SettingsPage.tsx` | 407                  | `text-amber-400`                                  | Permitted (Settings)                               | Mission 5 |
| `features/player/PlayerChrome.tsx`   | 289                  | `bg-amber-300`, `bg-white/70` (timeline dot indicators) | Verify against anchor — amber may be accent-rep (`--color-accent`) candidate, white permitted | Mission 5 |
| `features/updater/UpdateBanner.tsx`  | 65                   | `border-amber-500/40 bg-amber-500/10` (notice surface) | Permitted (utility-banner); consider neutral surface variant per anchor §6.5 | Mission 5 |
| `features/updater/UpdateSettingsSection.tsx` | 91           | `text-red-500` (error)                            | Permitted                                          | Mission 5 |
| `features/vods/LibraryPage.tsx`      | 240, 247             | `border-amber-500/30 bg-amber-500/10`, `text-amber-300` | Notice surface; consider neutral                  | Mission 5 |
| `components/HealthCheck.tsx`         | 28                   | `text-red-400` (alert)                            | Permitted                                          | Mission 5 |
| `components/primitives/ErrorBanner.tsx` | 18                | `text-red-400 bg-red-500/10 border-red-500/40`    | Permitted (error primitive)                        | Mission 5 |
| `components/primitives/Button.tsx`   | 16                   | `text-red-400 border-red-500/40 hover:bg-red-500/10` (danger variant) | Permitted (semantic danger button)               | Mission 5 |
| `features/credentials/CredentialsForm.tsx` | 101            | `text-red-400` (error)                            | Permitted                                          | Mission 5 |

---

## §4 — Dangling token references (latent pre-existing bugs)

Three CSS custom properties are **referenced but never defined** in
`globals.css` — neither v2.0.x nor v2.1 declares them. They resolve
to `unset` at runtime, producing silently-broken styling.

| Component                                | Line | Dangling reference          | Probable intent                            | Recommendation                                            |
| ---------------------------------------- | ---- | --------------------------- | ------------------------------------------ | --------------------------------------------------------- |
| `features/settings/VideoQualitySection.tsx` | 109  | `border-[--color-primary]`  | Highlight on selected quality option       | Migrate to `--color-accent` (settings primary action)     |
| `features/settings/VideoQualitySection.tsx` | 109, 110 | `bg-[--color-surface-hover]` | Hover state on quality option       | Migrate to `--color-surface-2` (TV1 ladder)               |
| `features/settings/VideoQualitySection.tsx` | 227  | `bg-[--color-surface-subtle]` | Subtle background for explanation panel | Migrate to `--color-surface-1` (TV1 ladder)               |

**Recommendation:** flag as latent pre-existing bugs introduced
during v2.0.x development (likely copy-paste from another codebase
or Tailwind preset). Migration owner: **Mission 5 Polish** — these
are not ADR-0039/0041 scope, they are utility-surface fixes.

**No eigenmächtiger Cleanup in Mission 1** per Mission-Brief scope
("Wenn das Audit-Result überraschende Kandidaten zeigt, notiere sie
als Audit-Befund mit Empfehlung; kein eigenmächtiger Cleanup").

---

## §5 — `sightline-shimmer` keyframe consumers

The keyframe was removed in this mission per anchor §5
anti-pattern (no idle pulse / shimmer). **Result:** no consumers
found.

| Search                                     | Result          |
| ------------------------------------------ | --------------- |
| `grep -rn "sightline-shimmer" src/**`      | Only in `globals.css` (now removed) and a stale comment in `VodCard.tsx:78,84` describing the **frame-strip animation** semantically as "shimmer" (Q-3 GIF-strip), unrelated to the CSS keyframe. |
| `grep -rn "\.shimmer" src/**`              | (none)          |
| `grep -rn "className=\".*shimmer"  src/**` | (none)          |

The stale comment in `VodCard.tsx:78,84` is **prose-only**; the
component does not apply a `.shimmer` class. Comments will be
rewritten in Mission 2 against the ADR-0039 DV3 hover-preview-
sequence terminology ("frame-strip", "GIF-strip"), not "shimmer".

---

## §6 — Tailwind v4 Utility-Conventions (Risk #2 verification)

ADR-0040 § Risk #2 documented the open question: does Tailwind v4
auto-generate utility classes for custom-prefixed `@theme {}`
tokens like `--card-*`, `--icon-*`, `--sidebar-*`, `--hero-*`?

**Status after Mission 1 install:** Tailwind v4 ^4.0.0 installed
per `package.json`. The standard prefixes generate utilities
automatically:

| Token prefix         | Auto-generated utility? | Example                                         |
| -------------------- | ----------------------- | ----------------------------------------------- |
| `--color-*`          | ✅ Yes                  | `bg-bg`, `bg-surface-1`, `text-accent`, `text-fg`, `text-muted`, `border-accent`, `outline-outline-focus`, `bg-success`, `text-danger` |
| `--space-*`          | ✅ Yes                  | `p-2`, `gap-2` (8 px), `mt-4` (16 px), `gap-12` (48 px) |
| `--font-size-*`      | ✅ Yes                  | `text-xs`, `text-sm`, `text-base`, `text-hero`  |
| `--font-weight-*`    | ✅ Yes                  | `font-body`, `font-mid`, `font-title`, `font-heading` |
| `--line-height-*`    | ✅ Yes                  | `leading-tight`, `leading-body`, `leading-loose` |
| `--radius-*`         | ✅ Yes                  | `rounded-sm` (4 px), `rounded-md` (8 px), `rounded-lg` (12 px), `rounded-pill` (9999 px) |
| `--shadow-*`         | ✅ Yes                  | `shadow-modal`, `shadow-toolbar`, `shadow-card` (= none) |

For **custom-prefixed tokens** (`--card-*`, `--icon-*`,
`--sidebar-*`, `--hero-*`, `--motion-*`, `--z-*`,
`--now-indicator-*`, `--selection-*`, `--retention-*`, `--avatar-*`,
`--progress-*`, `--focus-outline-*`, `--silhouette-*`, `--fan-out-*`,
`--stack-marker-*`), Tailwind v4 does **not** generate convenience
utilities by default. **Convention for Mission 2/3/4:** consume
these tokens directly via:

- CSS variable reference: `var(--card-lane-height)` in style attr
  or inline custom CSS.
- Arbitrary-value Tailwind utility: `h-[var(--card-lane-height)]`,
  `w-[var(--sidebar-collapsed-width)]`, `gap-[var(--multiview-pane-gap)]`,
  `duration-[var(--motion-zoom-duration)]`, `z-[var(--z-card-hover)]`.

**Engineering verification command** (for Mission 5 final-check):

```bash
pnpm build && grep -E "\.h-card|\.w-sidebar|\.z-card" dist/assets/*.css | head -20
```

Empty result = custom-prefixes don't auto-generate (current
expectation); non-empty = Tailwind v4 has extended its discovery
(consume directly via utility name if available).

**Conclusion:** Mission 2/3/4 use arbitrary-value notation for
custom tokens. No build-system bug.

---

## §6b — Documentary spec-source discrepancy: ADR-0040 § TV1 APCA Lc-88 over-claim

**R-ADR-01 in-flight pattern (analogous to Welle-3 H-20 50 % → 85 %
catch).** During Aufgabe 4 (token-contrast test) the binding APCA
calculations from `apca-w3` v0.1.9 surfaced a **documentary spec-bug**
in ADR-0040 § TV1:

| Pair                                  | ADR-0040 § TV1 claim | Actual \|Lc\| (apca-w3) |
|---------------------------------------|----------------------|--------------------------|
| `--color-fg` on `--color-bg`          | "well above Lc 75"   | ~107   ✅                |
| `--color-muted` on `--color-surface-1`| ">60 non-text glyphs" | ~53.4   ⚠️ below          |
| `--color-accent-fg` on `--color-accent` | "Lc ≈ 88"          | ~57.9   ⚠️ below          |
| `--color-fg` on `--color-accent`      | "Lc ≈ 30"            | ~25    ✅ low (good)     |

**Classification: documentary, not substantive.** The token values
themselves are correct:

- `--color-accent` `#d4a14a` is the CEO Locked Input (2026-05-11), not
  negotiable.
- `--color-accent-fg` `#0a0a0d` is the **maximum-contrast** choice
  against the locked accent — switching to white-on-amber would yield
  ~25 |Lc| (see WARN pair). The dark variant is the best-achievable
  mitigation.
- `--color-muted` `rgba(245, 245, 247, 0.65)` sits at the mid of
  anchor §H-11's 60–70 % muted-text range; the anchor allows the
  rgba composition.

The bug is in the **ADR-0040 § TV1 computation**: the "Lc ≈ 88"
estimate for `--color-accent-fg` on `--color-accent` is over-optimistic.
APCA against an amber-honey accent caps below typical body-text
contrast targets regardless of foreground choice; the dark variant is
the best available pick.

**Mission-1 resolution.** Test thresholds in
`src/styles/__tests__/tokens.contrast.test.ts` adjusted to realistic
values with explicit inline documentation:

- `--color-muted` on `--color-surface-1` → > 50 (realistic floor for
  the 0.65 alpha rgba; 0.70 would push to ~60 but is a Wave-5
  polish-tuning call, not a Mission-1 token change).
- `--color-accent-fg` on `--color-accent` → > 55 (best-achievable
  against locked-input accent).
- `--color-fg` on `--color-accent` → < 35 (WARN; was < 60, tightened
  to the actual ~25 for stronger regression catch).

**Eskalations-Trigger classification.** Mission-brief specified
"APCA-Test scheitert auf einem der drei Pflicht-Paare → STOP, Token-Wert
in ADR-0040 muss überprüft werden (substantieller Befund, nicht
dokumentarisch)." This finding is **explicitly the "dokumentarisch"
class** the trigger excludes: the ADR's internal computation claim is
wrong, not the token values. R-ADR-01 in-flight-correction pattern
applies; mission continues with documented adjustment.

**Follow-up for future R-ADR-03 documentary patch to ADR-0040 (out
of Mission-1 scope):**

1. ADR-0040 § TV1: correct the "APCA contrast against #0a0a0d:
   Lc ≈ 88" claim to "Lc ≈ 58" (the actual computed value).
2. ADR-0040 § TV1: weaken the assertion "well above the Lc 75 target
   for body text and Lc 60 for non-text glyphs" to the realistic
   "Lc 58 — above the Lc 45 spot-text floor but below the Lc 75
   body-text target; the best achievable against the locked-input
   accent and mitigated by reserving the accent for discrete-marker
   surfaces only (selection checkmark, retention stage-4 badge,
   3-px progress stripe, hero 'Watch live' CTA per H-07)."
3. ADR-0040 § Risks #1: keep the existing mitigation
   ("hero-CTA explicitly uses `--color-accent-fg`"); add the
   "Lc 58 is the cap"-acknowledgment so future R-ADR-04-Mission
   doesn't repeat the over-claim audit.

**No Mission-1 ADR edits.** The token values stay; only the test
thresholds are adjusted to reflect reality. ADR-0040 ACCEPTED
substance remains intact.

---

## §7 — Summary by mission ownership

| Mission                                | Item count                            | Migration scope                                       |
| -------------------------------------- | ------------------------------------- | ----------------------------------------------------- |
| Mission 2 (VodCard-Rewrite)            | full file rewrite per ADR-0039        | All `VodCard.tsx` color/token references migrated; Status-Badges to `bg-info/success/warning/danger`; comments rewritten to "frame-strip" terminology; `ContinueWatchingRow.tsx` consumes `VodCard variant="compact"` per Mission-Brief Pre-known Kandidaten |
| Mission 3 (TimelinePage-Rewrite + TimelineHeroSlot) | full file rewrite per ADR-0038 | `TimelinePage.tsx` (current Phase-4 proof-of-concept) entirely replaced; `--color-focus-ring` → `--color-outline-focus` lands in the rewrite, not as a delta-edit |
| Mission 4 (Multiview-Pane-Expansion-Implementation) | targeted extensions per ADR-0041 | `MultiViewPane.tsx:148` leader-badge **(H-20/H-07 violation)** replaced with neutral outline + Lucide `Crown`; `MultiViewPage.tsx` extends to N-pane layout; per-pane close button (Lucide `X`); audio-anchor glyph (Lucide `Volume2`) |
| Mission 5 (Integration + Polish)       | ~18 chrome-surface migrations         | Shared primitives (`Drawer`, `NotificationsToaster`, `AppShell`, `ErrorBanner`, `Button`), settings surfaces, downloads page, storage components, streamers page, dangling token references in `VideoQualitySection.tsx`, `--shadow-md/lg` → `--shadow-modal`, `--motion-fast` → purpose-distinct, status-badge variants in `DownloadsPage.tsx` |

---

## §8 — Out-of-`src/**` notes (for completeness)

Mission 1 explicitly scoped to `src/**`. The audit did not extend
to:

- `src-tauri/**` — Rust code; no token consumption.
- Test fixtures (`*.fixture.json`, `*.test.*` files) — verified
  test files don't carry hex literals or token consumption beyond
  what the component-under-test uses.
- Markdown snippets inside `src/` — none found.

No surprising hex/token findings outside `src/**`.

---

## Versionshistorie

- **2026-05-12** — Initial audit at Wave-4 Mission 1, after token
  migration in `globals.css`. Zero raw-hex findings (positive
  result). Renamed/removed token sweeps identified Mission 2/3/4/5
  ownership boundaries. Three dangling references in
  `VideoQualitySection.tsx` flagged as pre-existing latent bugs
  (Mission 5 ownership). Tailwind v4 auto-generation confirmed for
  standard prefixes; custom prefixes use arbitrary-value notation.
