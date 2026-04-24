# Sightline — Design tokens

The tokens live in [`src/styles/globals.css`](../src/styles/globals.css) under
`@theme`. Tailwind 4 reads them at build time; every component references
tokens via utility classes (`bg-[--color-surface]`,
`shadow-[var(--shadow-md)]`) — the tokens are the single source of truth.

Dark mode is the first-class theme. Light mode is fully implemented via
`prefers-color-scheme: light`. The two themes share every non-colour token.

## Palette

| Token | Dark | Light | Purpose |
|-------|------|-------|---------|
| `--color-bg` | `#0a0a0d` | `#fafafa` | Page background. |
| `--color-surface` | `#14141a` | `#ffffff` | Cards, tray popover, navigation chrome. |
| `--color-surface-elevated` | `#1c1c24` | `#f4f4f5` | Modals, drawers, dropdowns — one level up. |
| `--color-surface-interactive` | `#242430` | `#e4e4e7` | Hover/active state for interactive surfaces. |
| `--color-fg` | `#f5f5f7` | `#0f0f12` | Primary text. |
| `--color-muted` | `#a1a1aa` | `#52525b` | Secondary text, captions. |
| `--color-subtle` | `#71717a` | `#71717a` | Tertiary text — metadata, helper copy. |
| `--color-accent` | `#7c3aed` | `#7c3aed` | Brand accent; selected states, primary CTA. |
| `--color-accent-fg` | `#ffffff` | `#ffffff` | Foreground on `--color-accent`. |
| `--color-success` | `#10b981` | `#10b981` | Completed downloads, live indicators. |
| `--color-warning` | `#f59e0b` | `#f59e0b` | Retryable failures, paused states. |
| `--color-danger` | `#ef4444` | `#ef4444` | Permanent failures, destructive actions. |
| `--color-info` | `#3b82f6` | `#3b82f6` | In-flight work, timeline bars. |
| `--color-border` | `#2a2a34` | `#e4e4e7` | Default border. |
| `--color-border-strong` | `#3a3a48` | `#d4d4d8` | Emphasised border. |
| `--color-focus-ring` | `#8b5cf6` | `#7c3aed` | 2-px outline for `:focus-visible`. |

## Spacing (4 px base)

`--space-0` (0), `--space-1` (4 px), `--space-2` (8 px), `--space-3` (12 px),
`--space-4` (16 px), `--space-5` (20 px), `--space-6` (24 px), `--space-8`
(32 px), `--space-12` (48 px), `--space-16` (64 px). Tailwind's `p-4` / `gap-6`
etc. map directly; the tokens are there for component props that need CSS-var
substitution.

## Typography

`--font-size-xs` (12 px), `--font-size-sm` (14 px), `--font-size-base` (16 px),
`--font-size-lg` (18 px), `--font-size-xl` (20 px), `--font-size-2xl` (24 px).
The body font is the system UI stack — zero web fonts loaded, keeps startup
under the 2 s budget on cold cache.

## Motion

| Token | Duration | Use |
|-------|----------|-----|
| `--motion-fast` | 120 ms ease-out | Hover highlights, icon swaps. |
| `--motion-base` | 200 ms ease-out | Dropdowns, drawer slide-in, tab swaps. |
| `--motion-slow` | 320 ms ease-out | Full-page route transitions. |

`prefers-reduced-motion: reduce` zeros all animation + transition durations
globally. Keep animations that carry meaning (download progress) driven by
state, not CSS keyframes — the reduced-motion rule would flatten them otherwise.

## Radius

`--radius-sm` (4 px) · `--radius-md` (8 px) · `--radius-lg` (12 px) ·
`--radius-xl` (16 px). Drawer headers / modal dialogs use `--radius-lg`;
cards + chips use `--radius-md`; inline chips + pills use `--radius-sm`.

## Shadow

`--shadow-sm` (1 px) / `--shadow-md` (4 px) / `--shadow-lg` (12 px). The dark
theme uses higher opacity (30–50 %) so shadows read against the near-black
background; the light theme uses the usual 6–12 % opacity. Both are defined
under the `@media` light switch in `globals.css`.

## Focus rings

Every interactive element gets a focus ring automatically via the global
`:focus-visible { outline: 2px solid var(--color-focus-ring); }` rule in
`globals.css`. Components should not override it unless they're replacing
the ring with something equivalent. The ring is offset by 2 px so it never
overlaps border edges.

## Keyframe animations

Registered in `globals.css`:

- `sightline-slide-in` — 200 ms translateX fade; used by the library
  detail drawer and notification toasts.
- `sightline-fade-in` — 120 ms fade; used by modal backdrops.
- `sightline-shimmer` — pulsing opacity; reserved for skeleton loaders.

All keyframes are suppressed under `prefers-reduced-motion: reduce`.

## Refreshing the tokens

1. Edit `src/styles/globals.css` (under `@theme { ... }` for shared values,
   inside the `@media (prefers-color-scheme: light)` block for light
   overrides).
2. Run `pnpm dev` — Tailwind reloads the token map on file save.
3. Update this doc with new tokens.
4. If a component was hard-coding a colour (search for `#rrggbb` literals),
   replace with the matching token.
