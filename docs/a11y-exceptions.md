# Accessibility exceptions

The `pnpm a11y` gate runs axe-core against every route in a
jsdom harness (see `src/a11y/a11y.test.tsx`). Any WCAG 2.1 A/AA rule
that axe flags causes CI's `checks` job to fail.

This file enumerates the rules we intentionally skip and why. The goal
is a list that a human can read in under a minute and reject if any
entry starts rotting.

## Currently-skipped rules

### `color-contrast` and `color-contrast-enhanced`

- **Why.** axe checks contrast by reading computed styles via
  `window.getComputedStyle`. jsdom implements that API but does not
  actually resolve Tailwind's CSS custom properties to RGB values —
  every `var(--color-fg)` comes back as the literal string rather than
  the final colour. Every token ends up being compared against a
  literal "var(..)" and the check fires even on combinations we've
  manually verified meet AA.
- **How we keep contrast clean anyway.** The design tokens live in
  `src/styles/globals.css` with both light and dark values. Every
  foreground-background pairing listed in `docs/design-tokens.md` meets
  WCAG AA on its own; the Phase 4 design-token review exercised every
  pairing with a real contrast checker and recorded the numbers there.
- **Escape-hatch tests.** If we ever move to a rendering harness that
  resolves CSS variables (e.g. Playwright + axe-playwright in a real
  browser), we remove this entry.

## Adding a new exception

1. Try the fix first. axe is conservative and most rules can be
   satisfied with a small markup change.
2. If the rule has to be skipped, add its id to the `ALLOWLIST` in
   `src/a11y/a11y.test.tsx` alongside an inline comment pointing here.
3. Append a new section to this file with:
   - the axe rule id
   - why the rule can't be satisfied
   - the compensating control (manual review cadence, or future work)
4. A follow-up must be tracked; a skipped rule without a removal plan
   accrues silent regression risk.

## Removing an exception

When the compensating control goes away (we move to a real browser
harness, we upgrade jsdom to a version that resolves CSS vars, etc.),
delete the entry from both `ALLOWLIST` and this file in the same
commit. If that makes the axe tests fail, fix the violations first —
do not relist the rule.

## Out-of-scope rules

axe ships hundreds of rules beyond the WCAG 2.1 A/AA set. We opt in
through the `runOnly: { type: "tag", values: ["wcag2a", "wcag2aa",
"wcag21a", "wcag21aa"] }` tag filter in the test. `best-practice` and
other rules are intentionally excluded — the project ships with
opinionated Tailwind + shadcn markup that falls afoul of a handful of
"prefer a `<nav>` over a landmark role" suggestions we've deliberately
made. This is a policy, not a bug; revisit on the Phase 7 localisation
+ a11y polish pass.
