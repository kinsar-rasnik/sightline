---
name: phase-gate
description: Run the full phase-gate quality check and report what passed, what failed, and whether the phase is shippable. Use at the end of a phase or before tagging a release.
---

# Phase gate skill

## Trigger
- The Senior Engineer declares a phase candidate-complete.
- Before creating a git tag or preparing a release.
- Ad-hoc, when the human CTO asks "are we good?".

## Inputs
- Phase number (for the session report path).
- Optional: a baseline commit to compare against.

## Process

1. **Working tree clean.**
   - `git status --porcelain` must be empty. If not, stop and surface the unsaved work.

2. **Branch posture.**
   - On `main` or a phase branch. Fetch and compare to `origin/main`.

3. **Rust quality.**
   - `cd src-tauri && cargo fmt --check`
   - `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings`
   - `cd src-tauri && cargo test`

4. **Frontend quality.**
   - `pnpm install --frozen-lockfile`
   - `pnpm typecheck`
   - `pnpm lint`
   - `pnpm test`

5. **IPC drift.**
   - `pnpm run check:ipc` — runs `cargo test --test ipc_bindings` (regenerates `src/ipc/bindings.ts` via tauri-specta) and then `git diff --exit-code` on that file. See ADR-0007.

6. **Build.**
   - `pnpm tauri build` (dev build on CI; release build when producing installers).

7. **Docs freshness.**
   - Every ADR that was referenced in the phase exists.
   - `docs/session-reports/phase-NN.md` exists and is non-empty.
   - `README.md` roadmap table matches reality.

8. **Security.**
   - `cargo audit` — no unhandled high-severity advisories.
   - `pnpm audit --prod` — same.

## Validation (what "done" looks like)

A green phase-gate produces a block like:

```
PHASE N — GREEN
  rust: fmt ok / clippy ok / tests 14 passed
  frontend: typecheck ok / lint ok / tests 22 passed
  ipc: in sync
  build: ok (size 4.2 MB)
  docs: adrs present / session report present / readme aligned
  audit: cargo ok / pnpm ok
```

A red phase-gate prints the failing step(s) with the first 20 lines of output, and a prescription: which doc / test / fix to write.

## Out of scope
- Tagging the release. That is a human action (CEO).
- Publishing artifacts.
- Writing the session report — use the `docs-writer` subagent, or author inline.

## Examples

Good: after finishing a phase, the Senior Engineer runs `/phase-gate 1`. Output is green; a session report is drafted; CTO reviews and signs off.

Bad: running the skill mid-phase "to check progress". The outputs are noisy and not actionable. Use `pnpm typecheck` / `cargo check` for quick feedback instead.
