# Phase 6 housekeeping — session report

**Date:** 2026-04-25
**Branch:** `phase-6/housekeeping` (off the Phase-5 final state, equivalent to PR #14 head)
**Commits:** 5 new on top of `e4c6201` (last Phase-5 docs commit).

## Repo state at session start

The session started with a corrupted working tree: HEAD on
`phase-6/housekeeping` carried every Phase-5 deliverable as committed
blobs, but the rsync from Proton-Drive had layered a pre-Phase-5
worktree snapshot on top of the post-Phase-5 `.git`. Result: `git status`
listed 49 entries (deletions of every Phase-5 file the worktree was
missing, plus reverts of CLAUDE.md / README / package.json /
Cargo.toml / docs to their pre-Phase-5 form). The `actions/cache@v5`
edit was the only genuine pre-existing change — staged, intact, at
both call sites in `.github/workflows/ci.yml` (lines 67 and 147).

Recovery was a single `git checkout-index --all --force` (the canonical
v2.53 form of `git restore --worktree --source=:0 .`). Index already
held the right state — Phase-5 blobs from HEAD plus the staged `@v5`
ci.yml — so the worktree repaint produced exactly one diff vs. HEAD:
the staged ci.yml change. No data was lost; nothing in the rsynced
worktree represented genuine Phase-6 work-in-progress.

Local `main` is still at `6d9d2b7` (Phase 4 era). `origin/main` after
the end-of-session fetch is at `a9d781a` (Phase 5 PR #14 squash-merge);
local main was deliberately not rebased — that's a destructive op the
mission spec reserved for follow-up.

## Deliverables

| # | Item | Status | Commit |
|---|------|--------|--------|
| 1 | Dependabot / audit sweep | Done (offline) | n/a — output captured below |
| 2 | `completionThreshold` threaded via `app_settings` (migration 0009) | Done | `1684942` |
| 3 | Player keyboard handler onto shortcuts service (`player_*` IDs) | Done | `f9ddfa7` |
| 4 | `volumeMemory = "vod"` localStorage persistence | Done | `c5ce8b2` |
| 5 | `actions/cache@v4` → `@v5` in CI | Done | `34e43eb` |
| 6 | Player E2E coverage (Vitest); Playwright proper deferred | Partial | `3957367` |

### 1 — Audit sweep

`pnpm audit --prod`: no known vulnerabilities.

`cargo audit`:

- 1 vulnerability (medium): `RUSTSEC-2023-0071` in `rsa 0.9.10`
  (Marvin attack timing sidechannel) via `sqlx-mysql`. We don't enable
  the sqlx-mysql feature in `Cargo.toml`, so the dependency is in the
  lockfile but not in the build graph for any Sightline binary. Per
  ADR-0017, audit is informational through Phase 6 and hardens in
  Phase 7. **No action this phase.**
- 20 unmaintained warnings, all transitive: `paste`, `proc-macro-error`,
  the `gtk-rs` GTK3 crate family (Linux-only, transitively from `wry` /
  `tauri`), `fxhash` / `unic-*` from `tauri-utils`, `rand 0.7.3` from
  `phf_codegen` (build-time only), and `glib 0.18.5` (unsound
  `VariantStrIter` impl, also Linux-only). All are upstream
  Tauri-driven; track via Tauri minor releases.

cargo audit exits 0 (warnings don't fail; the single vulnerability is
not in the build graph), so CI is unaffected.

### 2 — `completionThreshold` threading (`feat(watch): thread completion threshold via app_settings (migration 0009)`)

Closes Phase-5 deferral #1. `cmd_update_watch_progress` was hardcoding
`ProgressSettings::default()` because the user-configured threshold
lived only in `localStorage` on the frontend — meaning the watch state
machine ignored the value users picked in the Settings UI. The fix is
end-to-end:

```sql
-- 0009_completion_threshold.sql
ALTER TABLE app_settings
    ADD COLUMN completion_threshold REAL NOT NULL DEFAULT 0.9
        CHECK (completion_threshold >= 0.7 AND completion_threshold <= 1.0);

PRAGMA user_version = 9;
```

The column-level CHECK mirrors the documented 70–100 % range from
ADR-0018; the service-layer `clamp()` is defence-in-depth for any
patch that slips past the constraint. `cmd_update_watch_progress` now
calls `state.settings.get().await?` and threads the value into
`ProgressSettings`. The frontend Settings UI writes via
`update_settings({ completionThreshold })` instead of localStorage,
and `PlayerPage` reads it from `useSettings()`. Single source of
truth: `app_settings.completion_threshold`.

Schema bumped to `v9`. `AppSettings` dropped its `Eq` derive (`f64`
isn't `Eq`); the test helpers use `PartialEq` and weren't affected.

### 3 — Player keyboard via shortcuts service (`refactor(player): keyboard handler via shortcuts service (player_* IDs)`)

Closes Phase-5 deferral #4. `useShortcuts` (Phase 4) is window-scoped
+ chord-aware; the player previously rolled its own keydown listener
with a hand-written `switch (e.key)`. Now there's a sibling
`usePlayerShortcuts` (`src/features/player/use-player-shortcuts.ts`)
that's container-scoped and shares the named-action contract.

Extends `ActionId` with fifteen `player_*` variants (play/pause, seek
±5/±10, volume ±, mute, fullscreen, pip, chapter next/prev, speed
step ±, close). Defaults match the help-overlay strings in
`player-constants.PLAYER_SHORTCUTS`.

Window-scoped `useShortcuts` filters down to `GLOBAL_ACTION_IDS` so
player keys never fire from the library page; the player hook only
fires while its container has focus. Structural keys (Space, digits
0–9, Shift+Arrow, `,`/`.`) stay outside the customisable table —
they're either an alias or a range that doesn't fit a single-key
binding.

Tests: `use-player-shortcuts.test.ts` covers every named binding plus
the four structural fallbacks and the input-scope guard (12 cases).

### 4 — Per-VOD volume memory (`feat(player): persist per-VOD volume memory (closes Phase 5 #6)`)

`usePlaybackPrefs.volumeMemory` accepted `"session" | "vod" | "global"`
since Phase 5 but the `vod` branch was a no-op — volume reset to 100 %
on every player open. Wires localStorage persistence keyed by vodId
(`sightline:player:volume:vod:<id>`) with a global-key fallback so a
freshly-opened VOD inherits the user's typical listening level rather
than blasting them at default.

`vod`-mode writes also touch the global key so the fallback chain
stays coherent when the user toggles policy back to `global`.

Tests: `volume-memory.test.ts` — 16 round-trip + clamping + scope
cases.

### 5 — `actions/cache@v4` → `@v5` (`ci: bump actions/cache to v5`)

Both sidecar-cache call sites move in lock-step (lines 67 + 147)
so the checks job and the test matrix share a cache hit/miss outcome.

### 6 — Player coverage; Playwright deferred (`test(player): cover keyboard, overlay-auto-hide, resume pre-roll`)

The full Phase-5 deferral asked for Playwright + a fixture MP4 + CI
e2e step. Landing that cleanly needs (a) a fixture-MP4 generation
script (probably driving the bundled ffmpeg sidecar) and (b) a mock
IPC layer for the dev-server webview (the generated bindings call
`@tauri-apps/api/core::invoke` which throws without `__TAURI_INTERNALS__`).
Both are meaningful enough pieces of work to warrant their own commit;
attempting them inline here would have ballooned this housekeeping
pass past its scope.

Instead, this commit covers the three behaviours the deferral cared
about with Vitest:

- Every named player keyboard shortcut + the four structural
  fallbacks (covered in commit `f9ddfa7` — 12 cases).
- Overlay auto-hide (`PlayerChrome.test.tsx` — 5 cases): hide after
  3 s, stay-visible-while-paused, mousemove resets the timer, pause
  flips it back to visible.
- Resume pre-roll math (`player-constants.test.ts` — 8 cases):
  pre-roll subtraction, restart-threshold-of-end clamp, deep-link
  override, missing-stored-position fallback, custom pre-roll values.
  Pre-roll math factored out of the inline `useEffect` in PlayerPage
  into a pure `computeInitialSeekSeconds` so the JS path mirrors the
  already-tested Rust `domain::watch_progress::resume_position_for`.

**Open: Playwright + Tauri-driver wiring** for full real-browser
coverage of the same scenarios. Tracking this as the only Phase-6
deferral.

## Quality gates (local, all green)

```
pnpm typecheck                                     ✅  clean
pnpm lint                                          ✅  clean (zero warnings)
pnpm test                                          ✅  77 / 77 passed (11 files)
cargo fmt --check                                  ✅  no diff
cargo clippy --all-targets --all-features          ✅  zero warnings
cargo test --all-features                          ✅  303 / 303 passed
                                                       (lib 288, integration 15: ipc_bindings 1,
                                                        sidecar_smoke 2, tray 3, plus poll/storage/migrate)
ipc bindings drift                                 ✅  no diff after regeneration
```

## Git fetch result

`git fetch origin` succeeded at end of session (~30 s). Brought
origin/main from `6d9d2b7` (Phase 4) to `a9d781a` (Phase 5 PR #14
squash-merge). **Local `main` was deliberately not pulled / rebased**
per the mission constraint of "no destructive ops without explicit
authorisation"; that's the next step before opening a Phase 6 PR.

## Open issues / what got skipped

1. **Playwright + Tauri-driver E2E** for the player. Unit-test
   coverage substitutes for now (see item 6). Track as a follow-up
   work unit: needs fixture-MP4 generation script + dev-mode mock
   IPC layer + `pnpm e2e` script + CI step.
2. **Local `main` still at Phase 4.** `origin/main` is two squash
   commits ahead. Pulling local main + tagging `phase-5-complete`
   was deferred per session constraints.
3. **`tag phase-6-complete`** — pending until phase-gate passes
   in CI on the Phase 6 PR.
4. **Audit follow-up not pursued.** Per ADR-0017, advisory
   warnings stay informational through Phase 6; Phase 7 hardens.

## Recommendation for next step

When GitHub access is back and the operator can move freely:

1. `git checkout main && git pull --ff-only origin main` to bring
   local main to `a9d781a`.
2. `git tag phase-5-complete a9d781a && git push origin phase-5-complete`.
3. `git checkout phase-6/housekeeping && git rebase main` — should
   be conflict-free (Phase 6 work doesn't touch the Phase 5 surface
   again, only extends it).
4. `git push -u origin phase-6/housekeeping` and open the Phase 6
   PR. Suggested title: `chore: Phase 6 housekeeping — close 5 of 6
   Phase-5 deferrals`.
5. After CI green: tag `phase-6-complete`. Then plan the Playwright
   follow-up (small, focused PR; estimated half-day given the mock-IPC
   piece is the load-bearing decision).
