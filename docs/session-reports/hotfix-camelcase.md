# Hotfix: camelCase / snake_case field alignment after tauri-specta migration

**Date:** 2026-04-24
**Scope:** Frontend only (`src/**/*.{ts,tsx}`)
**Branch:** `main`
**Resolves:** Phase 02 Deviation #3 (Proton Drive stall prevented local typecheck)

## Summary

`pnpm typecheck` reported 61 TypeScript errors after the Phase 2 tauri-specta
typegen migration. The generated bindings in `src/ipc/bindings.ts` now emit
field names in camelCase (e.g. `twitchUserId`, `displayName`, `positionMs`),
but the hand-written frontend code was still referencing the pre-migration
snake_case form (`twitch_user_id`, `display_name`, `position_ms`). This hotfix
is a mechanical rename across the frontend to re-align with the generated
contract. No Rust or binding file was touched.

One pre-existing shape mismatch surfaced after the renames: `SettingsPatch` is
a required-fields-with-null type (tauri-specta emits `Option<T>` as
`T | null` on a required key, not an optional key), so every `update_settings`
call must carry all six keys. The frontend was previously passing partial
objects, which was actually wrong against the binding before this hotfix too —
it was just masked by the snake_case errors. A small `EMPTY_PATCH` constant
and a spread at the two call sites resolves this without touching the
backend. See the "Follow-ups" section below.

## Files touched

| File | Change |
|------|--------|
| `src/features/credentials/CredentialsForm.tsx` | `client_id_masked` → `clientIdMasked`, `last_token_acquired_at` → `lastTokenAcquiredAt` |
| `src/features/credentials/CredentialsForm.test.tsx` | Same field renames in mock `CredentialsStatus` objects |
| `src/features/settings/SettingsPage.tsx` | 5 field renames on `AppSettings`/`SettingsPatch` + added `EMPTY_PATCH` baseline so partial updates satisfy the all-keys-required shape of `SettingsPatch` |
| `src/features/streamers/StreamersPage.tsx` | Renamed `Streamer`/`StreamerSummary`/`PollStatusRow` field accesses (`twitch_user_id`, `display_name`, `profile_image_url`, `last_polled_at`, `live_now`, `vod_count`, `eligible_vod_count`, `next_poll_eta_seconds`, `last_poll`, nested `started_at`) |
| `src/features/streamers/StreamersPage.test.tsx` | Same field renames in `stubStreamer` and the `PollStatusRow` fixture (including nested `LastPollSummary` — `started_at`/`finished_at`/`vods_new`/`vods_updated`) |
| `src/features/streamers/use-streamers.ts` | `commands.removeStreamer({ twitch_user_id })` → `{ twitchUserId }` shorthand; same for `commands.triggerPoll` |
| `src/features/vods/LibraryPage.tsx` | Removed unused `Button` import; renamed ~20 `Vod`/`Chapter`/`VodWithChapters` fields (`twitch_video_id`, `stream_started_at`, `duration_seconds`, `view_count`, `is_sub_only`, `ingest_status`, `status_reason`, `streamer_display_name`, `position_ms`, `game_name`, `chapter_type`, etc.); switched `VodFilters` to the new camelCase keys AND changed `undefined`→`null` for unset filters (the binding types them as `T \| null` required, not `T \| undefined`) |
| `src/features/vods/LibraryPage.test.tsx` | Same field renames in `stubVod` (including `Chapter` nested fields) |
| `src/features/vods/use-vods.ts` | `commands.getVod({ twitch_video_id })` → `{ twitchVideoId }` |

All tests were updated in lockstep with the production code (mock fixtures
only — no test semantics changed).

## Quality-gate output

```
$ pnpm typecheck
> sightline@0.1.0 typecheck
> tsc -b --noEmit
(no output — zero errors)

$ pnpm lint --max-warnings=0
> sightline@0.1.0 lint
> eslint src --max-warnings=0 --max-warnings=0
(no output — zero errors, zero warnings)

$ pnpm test
> sightline@0.1.0 test
> vitest run
 ✓ src/components/HealthCheck.test.tsx        (3 tests)
 ✓ src/features/streamers/StreamersPage.test.tsx (6 tests)
 ✓ src/features/credentials/CredentialsForm.test.tsx (4 tests)
 ✓ src/features/vods/LibraryPage.test.tsx     (5 tests)
 Test Files  4 passed (4)
      Tests 18 passed (18)

$ pnpm build
> sightline@0.1.0 build
> tsc -b && vite build
 ✓ 99 modules transformed.
 dist/index.html                   0.45 kB │ gzip:  0.28 kB
 dist/assets/index-Cj2WyLHE.css   16.03 kB │ gzip:  3.98 kB
 dist/assets/index-CxphI2Ds.js   260.66 kB │ gzip: 80.16 kB
 ✓ built in 26.26s
```

Rust side was already green from Phase 2; no `cargo` commands were rerun.

## Did CI catch this at the time?

**No — and the reason is worse than the hotfix brief assumed.**

The brief's working hypothesis was that the Linux CI run should have caught
the mismatch and that the broken frontend slipped in because the migration
commit either preceded or was co-committed with the broken code. The
investigation found a more fundamental problem: **CI has never successfully
run `pnpm typecheck` against this project.**

Evidence (from `gh run list --workflow=ci`):

| Commit | CI result | Where it failed |
|--------|-----------|------------------|
| `df0e598` (phase-02 session report) | cancelled | — |
| `ce0f1e0` (`.protonignore`)           | failure | `pnpm/action-setup@v4` |
| `4bb123f` (pnpm modules-dir)          | failure | `pnpm/action-setup@v4` |
| Every Dependabot bump since        | failure | `pnpm/action-setup@v4` |

All 12 CI runs to date fail at the **pnpm setup step**, before the typecheck
step ever runs. The failure message:

```
Error: Multiple versions of pnpm specified:
  - version 10 in the GitHub Action config with the key "version"
  - version pnpm@10.33.0 in the package.json with the key "packageManager"
Remove one of these versions to avoid version mismatch errors like
ERR_PNPM_BAD_PM_VERSION
```

The `packageManager: "pnpm@10.33.0"` field was added in the first frontend
scaffold commit (`79738ec`, 2026-04-24). `pnpm/action-setup@v4` in
`.github/workflows/ci.yml` pins `version: 10` and refuses to coexist with the
`packageManager` declaration.

So the answer is: **CI could not have caught this hotfix's root cause, because
CI has been structurally broken since the first frontend commit landed.** This
is a P0-class CI gap and supersedes any pre-merge local-gate work Phase 3 had
planned to do.

## Follow-ups (flag for Phase 3 scoping)

1. **Unbreak CI.** Remove the `version: 10` arg from the three
   `pnpm/action-setup@v4` invocations in `.github/workflows/ci.yml` so the
   action honours `packageManager` in `package.json`. Without this, no
   regression can be caught automatically. This is urgent — the whole
   `main` branch has been shipping without a working CI gate. Once CI is
   green, backfill a "CI must be green to tag a release" constraint into the
   phase-gate skill.
2. **Local pre-push gate.** Phase 2's Deviation #3 already flagged that a
   local typecheck is not being run before push. The straightforward fix is
   a lefthook / husky / plain `.git/hooks/pre-push` script that runs the
   same quality gate as CI (`cargo fmt --check`, `cargo clippy -D warnings`,
   `cargo test`, `pnpm typecheck`, `pnpm lint`, `pnpm test`) and blocks push
   on failure. Document as a new ADR if more than a shell script is
   warranted.
3. **`SettingsPatch` ergonomics (nice-to-have, not a bug).** tauri-specta
   currently emits Rust `Option<T>` fields as `T | null` **required** keys,
   not `T?: null` **optional** keys. That forced the `EMPTY_PATCH` spread
   pattern added to `SettingsPage.tsx`. Options to consider:
   - Add `#[specta(optional)]` to the `Option<T>` fields in
     `services/settings.rs::SettingsPatch` (and wherever the same pattern is
     used for input types) so the TS emission becomes optional.
   - Or keep the current behaviour and export an `emptyPatch()` helper from
     `@/ipc` so this baseline isn't re-declared in every caller.
4. **`VodFilters` same pattern.** Same fix applies — currently the frontend
   is passing `{ streamerIds: null, statuses: null, gameIds: null, since:
   null, until: null }` baseline for the same reason. If (3) is picked up,
   update this site in the same pass.

## Commit

Single commit, as the hotfix brief suggested:
`fix(frontend): align field names with tauri-specta camelCase bindings`.
See the commit body for the Phase-02 deviation link.
