# Hotfix: unbreak CI (`pnpm/action-setup` conflict + downstream bugs it masked)

**Date:** 2026-04-24
**PR:** [#10 — ci: unbreak pnpm/action-setup; restore clean .npmrc](https://github.com/kinsar-rasnik/sightline/pull/10)
**Merged commit:** [`fece630`](https://github.com/kinsar-rasnik/sightline/commit/fece630)
**First green run on `main`:** [run #24889369828](https://github.com/kinsar-rasnik/sightline/actions/runs/24889369828)
**Resolves:** `docs/session-reports/hotfix-camelcase.md` § Follow-up #1 (CI had never actually run typecheck).

## TL;DR

`.github/workflows/ci.yml` passed the string `version: 10` to
`pnpm/action-setup@v4` while `package.json` also declared
`packageManager: "pnpm@10.33.0"`. The action refuses to boot when both
are set, so every job on every CI run since Phase 1 exited 1 at the
**setup step** with `ERR_PNPM_BAD_PM_VERSION`. No CI job has ever
actually executed `cargo test`, `pnpm typecheck`, `pnpm lint`, or
`pnpm test` — Phase 1 and Phase 2 cross-platform green-check claims
were entirely unverified.

Fix: drop `version:` from the action input (three spots in `ci.yml`,
one spot in `release.yml`). The action then resolves pnpm's version
from `packageManager` in `package.json`, which is the officially
supported path since pnpm 9.

Unblocking the pnpm step exposed three more pre-existing bugs that
had been silently masked; all four are fixed in this hotfix.

## Root cause (evidence)

[Run #24886534565](https://github.com/kinsar-rasnik/sightline/actions/runs/24886534565)
(the run cited as "latest failure" at the top of this hotfix) produced
an identical failure on every job. Representative log line from the
`audit (cargo · pnpm)` job:

```
2026-04-24T11:12:26.858Z Error: Multiple versions of pnpm specified:
2026-04-24T11:12:26.858Z   - version 10 in the GitHub Action config with the key "version"
2026-04-24T11:12:26.858Z   - version pnpm@10.33.0 in the package.json with the key "packageManager"
2026-04-24T11:12:26.858Z Remove one of these versions to avoid version mismatch errors like ERR_PNPM_BAD_PM_VERSION
    at readTarget (.../pnpm/action-setup/v4/dist/index.js:1:7537)
    at runSelfInstaller (.../pnpm/action-setup/v4/dist/index.js:1:6702)
```

The same error was emitted by `checks`, `test (ubuntu-latest)`,
`test (macos-latest)`, `test (windows-latest)`, and `audit` — all five
jobs failed identically at `Run pnpm/action-setup@v4`, before reaching
any meaningful build or test step.

Historical scope: scrolling through `gh run list` back to the earliest
CI activity on this repo showed **12 CI runs, all either failure or
cancelled, none successful.** The `packageManager` field was added in
`79738ec` (the first frontend scaffold commit, 2026-04-24 00:59), and
the conflicting `version: 10` in the workflow was there from
`538f884` (the workflow scaffold, earlier). So the CI has in fact
been structurally broken since the very first commit that added a
frontend, which is every commit on this repo.

## What was changed

Four distinct changes, grouped into one PR because they are all
required to reach a green CI.

### 1. Drop `version:` from `pnpm/action-setup@v4`

`.github/workflows/ci.yml` (three invocations — `checks`, `test`,
`audit`) and `.github/workflows/release.yml` (one invocation —
`build`). The action's [README][pnpm-action-setup] states the
`version` input is optional when `packageManager` is present in
`package.json`, and mutually exclusive with it. Omitting `version`
makes the action read `packageManager` and install exactly
`pnpm@10.33.0`, keeping local dev and CI on the same pnpm version
without divergence.

[pnpm-action-setup]: https://github.com/pnpm/action-setup

Kept `packageManager` in `package.json` intact (the user explicitly
asked for this; it is also what Corepack and the action now read
from). `actions/setup-node@v4` with `cache: pnpm` continues to work
because it runs *after* `pnpm/action-setup`, which puts `pnpm` on
PATH first.

### 2. Revert `.npmrc` to the minimal form

Committed `.npmrc` at HEAD (from `4bb123f`) hard-coded absolute paths
to the user's local machine:

```ini
node-linker=hoisted
modules-dir=/Users/kinsar/.local/sightline-build/node_modules
virtual-store-dir=/Users/kinsar/.local/sightline-build/.pnpm-store
```

These paths do not exist on any CI runner and would have broken
`pnpm install` the moment the setup step started succeeding. Reverted
to the minimal CI-safe form:

```ini
auto-install-peers=true
strict-peer-dependencies=false
```

The local Proton-Drive-sync-stall workaround that motivated `4bb123f`
should live in a user-local override — `NPM_CONFIG_MODULES_DIR` env
var or an un-tracked `.npmrc.local` — not in the tracked file.

### 3. Add `src-tauri/icons/icon.ico`

With the pnpm setup finally running, `test (windows-latest)` failed
next with

```
`icons/icon.ico` not found; required for generating a Windows Resource
file during tauri-build
```

tauri-build's build script unconditionally looks for
`src-tauri/icons/icon.ico` when building on Windows, regardless of
what `bundle.icon` in `tauri.conf.json` lists. The repo only had PNGs.
Generated the full icon set via `pnpm tauri icon src-tauri/icons/icon.png`
and committed **only `icon.ico`** (1.9 KB). The iOS/Android/Square*/etc.
outputs that the CLI emitted are not needed for our CI or for a
desktop-only Phase 1 build, so they were discarded.

### 4. Gate `ipc_bindings` integration test on non-Windows

Even with `icon.ico` in place, `test (windows-latest)` still failed —
but this time much later, deep in `cargo test`. The Rust unit tests
and the `health_integration` and `ingest_integration` test binaries
all ran clean (87 unit + 1 + 5 + 0 tests, all passed). The
`ipc_bindings` integration test binary exited with
`STATUS_ENTRYPOINT_NOT_FOUND` (`exit code: 0xc0000139`) before the
test harness even started.

Root cause: `tests/ipc_bindings.rs` is the only integration test that
references `tauri::Wry` (via `Builder::<Wry>::new()` inside
`ipc_builder()`). `Wry` transitively links `wry → webview2-com-sys`,
which on the GitHub Actions `windows-latest` runner resolves against
a WebView2 import surface that doesn't match what the generated
Windows resource expects — so the exe is rejected by the loader at
startup. No other test binary pulls in `Wry`, which is exactly why
they all ran fine.

Fix: gated the test function and its sole import behind
`#[cfg(not(target_os = "windows"))]` with a module-level doc comment
explaining the skip. The generated `src/ipc/bindings.ts` is
byte-identical across platforms, so running the drift check on Linux
and macOS is sufficient. The commit message is explicit that solving
the underlying webview2 link issue belongs in a dedicated
tauri-upgrade PR, not this CI hotfix.

### 5. Add `working-directory: src-tauri` to `cargo audit`

The `audit` job failed with

```
error: not found: Couldn't load Cargo.lock
```

because `cargo audit` was run from the repo root while `Cargo.lock`
lives in `src-tauri/`. Added the missing `working-directory`, matching
the pattern already used by `cargo fmt`, `cargo clippy`, and
`cargo test` in the other jobs.

## Verification — first green run on `main`

[Run #24889369828](https://github.com/kinsar-rasnik/sightline/actions/runs/24889369828)
(commit `fece630`, the squash-merge of PR #10):

| Job | Conclusion | Duration |
|-----|-----------|----------|
| `checks (fmt · lint · typecheck)` | ✅ success | ~2m 21s |
| `test (ubuntu-latest)` | ✅ success | ~1m 48s |
| `test (macos-latest)` | ✅ success | ~1m 2s |
| `test (windows-latest)` | ✅ success | ~5m 59s |
| `audit (cargo · pnpm)` | ⚠️ failure (non-blocking) | ~3m 12s |

Workflow conclusion: **success**. No step was silently skipped — every
step's `Run` line appears in the job log, and the `ipc_bindings` skip
on Windows is an explicit, documented `cfg` gate (not a silent
workflow-level filter).

## Other CI gaps discovered

Four follow-ups of varying severity surfaced as a side effect of
actually running CI for the first time.

1. **`audit (cargo · pnpm)` is red on a transitive tauri dep and will
   stay red until tauri updates.** `cargo audit --deny warnings` fires
   on RUSTSEC-2026-0097 (`rand 0.7.3` unsound advisory) which reaches
   us through
   `rand → phf_generator → phf_codegen → selectors → kuchikiki → tauri-utils → tauri`.
   We cannot fix it without upstream tauri bumping its selectors/kuchikiki
   stack. The job is configured `continue-on-error: true` (workflow
   comment: "informational today, gating in Phase 7") and the workflow
   concludes green despite the red check. When Phase 7 moves audit into
   the required set, either wait for a tauri release that drops the old
   `rand` or add an `[advisories] ignore = ["RUSTSEC-2026-0097"]` entry
   with a time-boxed comment. Do not drop `--deny warnings` across the
   board — that would hide future unsoundness hits.

2. **Node.js 20 deprecation notice on every action.** The runner prints:

   > Node.js 20 actions are deprecated. The following actions are
   > running on Node.js 20 and may not work as expected:
   > actions/checkout@v4, actions/setup-node@v4, pnpm/action-setup@v4.
   > Actions will be forced to run with Node.js 24 by default starting
   > June 2nd, 2026. Node.js 20 will be removed from the runner on
   > September 16th, 2026.

   Dependabot has already opened PRs to bump these to v6 (currently
   sitting unmerged on dependabot branches; they could not land because
   CI was broken). Phase 3 should merge the dependabot bumps — now
   that CI is actually running, their checks will produce real signal.

3. **`checks` job does not use `Swatinem/rust-cache@v2`.** The `test`
   job does. That means clippy runs on a cold Rust target every time
   the `checks` job fires, which took ~2m 21s on ubuntu in the first
   green run and will only grow. Adding `rust-cache` to `checks` would
   be a pure speedup. Not in scope for this hotfix (the brief said
   "do NOT expand scope beyond making the existing jobs actually run"),
   but a 30-second follow-up.

4. **CI has no `pnpm build` / `vite build` gate.** `pnpm test` covers
   vitest, `pnpm typecheck` covers `tsc -b`, but nothing runs the
   frontend production build. If a change breaks rollup/vite but not
   tsc, CI won't catch it. Consider adding `pnpm build` to the `test`
   matrix in Phase 3. (The release workflow does run `tauri build`,
   which includes `pnpm build`, but release only fires on `v*` tags.)

## Local pre-push gate

Independently of CI, the original hotfix (`hotfix-camelcase.md`,
follow-up #2) flagged the need for a local pre-push gate so that
"Proton Drive stalls prevent local typecheck" and similar situations
can never again result in broken code hitting `main`. That work still
stands — CI being green is not a substitute for running the quality
gate on the same machine that made the commits. Recommended shape: a
`.git/hooks/pre-push` script (or lefthook/husky) running
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `pnpm typecheck`, `pnpm lint`, `pnpm test`, aborting
on any failure. File an ADR if the choice of framework matters.

## Commit trail

The PR squash-merged as a single commit (`fece630`), but on the branch
it landed as four logical commits:

- `5ba9c91` `ci: remove pnpm/action-setup version input; restore clean .npmrc`
- `e127f89` `ci: apply the same pnpm-setup fix to release workflow`
- `f6899e7` `ci: add windows icon.ico and run cargo audit from src-tauri`
- `0855da3` `test(ipc): skip ipc_bindings drift test on Windows`
