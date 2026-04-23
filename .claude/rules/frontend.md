---
description: Frontend conventions; loaded when touching React/TS source
glob: "src/**/*.{ts,tsx,css}"
---

# Frontend rules

## File organization

- Feature folders under `src/features/<feature-name>/`. A feature owns its components, hooks, stores, and tests.
- Shared primitives live in `src/components/ui/` (shadcn generated) and `src/components/primitives/` (hand-rolled).
- Generated IPC bindings live in `src/ipc/` and are read-only — the file starts with a header warning.

## State

- Server state via TanStack Query keyed by the command name and input.
- Cross-feature UI state via Zustand stores in `src/stores/`. One store per concern; avoid a megastore.
- Local UI state via `useState` / `useReducer`. If a `useState` grows past three fields, consider a reducer.

## Styling

- Tailwind CSS 4. Prefer utility classes in JSX.
- Design tokens live in `src/styles/globals.css` as CSS custom properties. Components reference tokens via utilities (`bg-[--color-surface]`).
- No CSS-in-JS runtime libraries.
- Dark mode via `prefers-color-scheme`; no runtime toggle in Phase 1.

## Accessibility

- Every interactive element has an accessible name (text, `aria-label`, or `aria-labelledby`).
- Keyboard paths exist for every mouse path; test them with Tab, Enter, Space, Escape, arrow keys as appropriate.
- `prefers-reduced-motion` honored for animations over 150ms.
- Color contrast meets WCAG 2.1 AA on both light and dark tokens.

## IPC

- Never call `invoke()` directly. Use typed wrappers from `src/ipc/`.
- Errors from IPC commands are typed unions (`AppError`). Handle each `kind` explicitly.
- Long-running calls bind a `useMutation` with optimistic updates only when the failure mode is easy to reverse.

## Testing

- Vitest + Testing Library for component tests. One `.test.tsx` per component with non-trivial logic.
- Prefer user-facing queries (`getByRole`, `getByLabelText`) over `getByTestId`.
- Mock IPC at the `src/ipc/` boundary, not at `invoke()`.

## TypeScript

- `strict` and `noUncheckedIndexedAccess` are on.
- No `any`. No `as unknown as T` except in a test's type-erasure for a generic helper.
- Discriminated unions for state machines; avoid boolean-flag soup.
