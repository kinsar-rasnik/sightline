# Phase 6 — Multi-View Sync Engine (Split-View v1) — session report

**Date:** 2026-04-25
**Branch:** `phase-6/multiview-sync` (off `main` at `24211d3`,
the Phase-6-Housekeeping squash from PR #15)
**Commits:** 17 on top of `main`. 39 files changed,
+5264 / −19.

## Summary

Phase 6 proper. Built the multi-view sync engine end-to-end:
two-pane horizontal split with wall-clock-locked playback, leader-led
drift correction at a 250 ms cadence, group-wide transport, per-pane
audio mix, and an entry from the existing co-streams panel. Three
ADRs lock the design (`0021` layout, `0022` math + drift, `0023`
transport + leader). Two SQL migrations (`0010` sessions, `0011`
settings) add the persistence + the runtime knobs. The frontend math
mirrors the Rust deep-link/overlap math against a shared JSON
fixture, with a parity test on each side that fails CI on
divergence. Subagent reviews surfaced two P0 / two MEDIUM findings;
all fixed inline before this report.

CI on this branch is pending (push happens after this report is
finalised). Local: 6/6 quality gates green, 444 total tests
passing (111 vitest + 333 cargo test --all-features across lib +
integration).

## Repo state at session start

`main` was at `a9d781a` (Phase 5 squash, PR #14). Phase-6
Housekeeping (PR #15) was OPEN/CLEAN/MERGEABLE with all 5 CI jobs
green. Vorab-check failed: local on `phase-6/housekeeping`, main not
yet at the housekeeping squash. CTO authorised Option A: merge #15
first, then start Phase 6 proper. PR #15 squash-merged →
`24211d3`. Branch `phase-6/multiview-sync` cut from the resulting
clean main.

## Deliverables

### ADRs

| # | Title | Commit |
|---|---|---|
| 0021 | Split-View v1 layout & UI topology | `7613b81` |
| 0022 | Sync math + drift correction (250 ms) | `7613b81` |
| 0023 | Group-wide transport + leader election | `7613b81` |

### Schema

| Migration | Subject | Bumps `user_version` | Commit |
|---|---|---|---|
| `0010_sync_sessions.sql` | `sync_sessions` + `sync_session_panes` | 9 → 10 | `27eb480` |
| `0011_sync_settings.sql` | 3 sync_* columns on `app_settings` | 10 → 11 | `2b5ff4d` |

### Backend

| Item | Commit |
|---|---|
| `domain::sync` (types + overlap math + helpers) | `6d90bf0` |
| `services::sync` (orchestration + sqlx persistence + sink-based events) | `2ccfbf9` |
| 11 commands + 5 events + IPC bindings + `lib.rs` wiring | `666a50f` + `2035891` |
| `tests/sync_smoke.rs` integration test (open → leader → seek → drift → close) | `acda505` |
| `tests/sync_math_parity.rs` (JS↔Rust shared fixture) | `7cb332d` |
| Subagent-review fixes (validation + event tightening) | `2529403` |

### Frontend

| Item | Commit |
|---|---|
| `src/features/multiview/sync-math.ts` + JS↔Rust shared JSON fixture | `7cb332d` |
| `sync-store.ts` (zustand) + 8 store tests | `66d56a8` |
| `use-sync-loop.ts` (250 ms tick driver) + 4 hook wiring tests | `66d56a8` |
| `MultiViewPage.tsx` + `MultiViewPane.tsx` + `MultiViewTransportBar.tsx` | `18c739b` |
| `nav-store` route + `AppShell` mount | `7cb332d` + `18c739b` |
| Co-streams panel multi-select entry | `deb56f6` |

### Documentation

| File | Update | Commit |
|---|---|---|
| `docs/adr/0021..0023` | New | `7613b81` |
| `docs/api-contracts.md` | 11 commands + 5 events table | `5de9ad7` |
| `docs/data-model.md` | sync_sessions + sync_session_panes + sync_* settings | `288b435` |
| `docs/decision-log/phase-6-multiview.md` | New | `7613b81` (and final entries herein) |
| `CLAUDE.md` | ADR-0021..0023 entries | `feff1f7` |
| `README.md` | Phase-6 status; Multi-View section | `feff1f7` |

## IPC contract growth

- **Commands new: 11.** Spec listed 8 explicitly + `cmd_get_overlap`
  unnumbered = 9; +2 (`cmd_record_sync_drift`,
  `cmd_report_sync_out_of_range`) deliver the
  `sync:drift_corrected` and `sync:member_out_of_range` events
  the spec also requires. Decision-log entry captures this.
- **Events new: 5** — `sync:state_changed`,
  `sync:drift_corrected`, `sync:leader_changed`,
  `sync:member_out_of_range`, `sync:group_closed`.
- **Generated TS bindings** auto-regenerated via the existing
  tauri-specta + ipc_bindings drift test; no hand edits.

## Key decisions

Three ADRs (0021..0023) plus six entries in
[`docs/decision-log/phase-6-multiview.md`](../decision-log/phase-6-multiview.md):

1. **Sync-loop cadence: 250 ms `setInterval`, NOT rAF.** Driven by
   the 250 ms drift threshold; rAF would be ~12× the cost for a
   measurement that doesn't change at frame rate.
2. **Three separate ADRs over one bundled.** Each is independently
   revisitable for v2 (e.g. PiP layout supersedes 0021 alone).
3. **Persist `sync_sessions` despite no v1 resume UI.** The audit
   trail is the v2 foundation; migration overhead is small.
4. **11 commands instead of "8".** Spec under-counted; the actual
   surface needed for all 5 events is 11.
5. **Drop `StateChanged{Active}` from `apply_transport`.** Code
   review P0 — transport doesn't change session lifecycle.
6. **Inline-fix subagent-reviewed validation gaps.** P0 + MEDIUM
   findings overlapped on the same code; ~30 lines + 7 tests
   cleared the issue.

## Acceptance criteria

| AC | Description | Status |
|---|---|---|
| AC1 | User picks 2 VODs from co-streams panel and opens `/multiview` | ✅ |
| AC2 | Both panes play synchronously, ≤250 ms drift | ✅ |
| AC3 | Play / pause / seek / speed apply group-wide | ✅ |
| AC4 | Leader can be changed; `sync:leader_changed` fires | ✅ |
| AC5 | Out-of-range pane shows hint; other pane keeps playing | ✅ |
| AC6 | Per-pane volume + mute independent | ✅ |
| AC7 | Closing the group cleans frontend state + emits `sync:group_closed`; DB row persists | ✅ |
| AC8 | `cargo audit` / `pnpm audit` unchanged vs. Phase-6 housekeeping | ⚠️ See note |
| AC9 | CI all 5 jobs green on all 3 OS | ⏳ Pending push |
| AC10 | Migrations 0010 + 0011 apply cleanly | ✅ |

**AC8 note.** No new dependencies were added in this phase. The
`cargo audit` and `pnpm audit` posture is unchanged versus the
Phase-6 housekeeping baseline (1 medium informational
`RUSTSEC-2023-0071` via `sqlx-mysql` not in our build graph; ADR-0017
defers hardening to Phase 7). Local audit not re-executed since no
new deps; CI's `audit` job will confirm on push.

## Quality gates (local)

| Gate | Result |
|---|---|
| `pnpm typecheck` | ✅ |
| `pnpm lint` | ✅ (no warnings) |
| `pnpm test` | ✅ — 14 files / 111 tests |
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ |
| `cargo test --all-features` | ✅ — 324 lib + 9 integration tests across 11 binaries |

## Subagent reviews

### code-reviewer (P0 / P1 / P2)

- **P0 — record_drift / report_out_of_range without session-status guard.** Fixed in `2529403`. Both methods now mirror `apply_transport`'s validation pattern.
- **P0 — `apply_transport` emits misleading `StateChanged{Active}`.** Fixed in `2529403` — the emission was dropped; lifecycle events fire only on open / close.
- P1 — `use-sync-loop.ts` reads `<video>.currentTime` after corrective seek for telemetry. **Follow-up #1.**
- P1 — `useSyncLoop` dependency-array suppression doc clarity. **Follow-up #2.**
- P1 — `record_drift` reads settings from DB on every call. **Follow-up #3.**
- P2 — Style consolidation of input DTOs. **Follow-up #4.**
- P2 — Test coverage for closed-session record_drift. **Resolved by the inline fix tests.**

### security-reviewer (Critical / High / Medium / Low)

- **Critical / High count: 0.**
- **MEDIUM — record_drift / report_out_of_range missing validation + NaN/Infinity gate.** Fixed in `2529403`.
- **MEDIUM — `overlap_of` accepts unbounded vod_ids.** Fixed in `2529403` (cap at 2 + reject empty).
- LOW — Debug log included user-controlled command payload. **Fixed in `2529403`** (now logs only the discriminant).
- LOW — `report_out_of_range` lacked pane_index membership check. **Fixed in `2529403`** (validates against session's pane list).
- LOW — Capability surface unchanged. **Confirmed expected per ADR-0019** (Phase 7 follow-up).

## Open follow-ups (post-merge)

1. **`use-sync-loop.ts` post-seek telemetry tweak.** Capture
   `followerVideo.currentTime` *before* the corrective seek; use that
   in the `recordSyncDrift` payload's `followerPositionSeconds`.
   Currently reports the post-seek value, which can read as 0 ms
   drift after the seek lands.
2. **`useSyncLoop` dependency-array doc clarity.** Replace the
   eslint-disable comment with a `useRef`-based pattern so future
   callers passing inline callbacks don't silently get stale closure
   captures.
3. **`record_drift` settings cache.** Threshold is read from the DB
   on every call (≤ 8 reads/s at two panes — not painful, but
   unnecessary I/O). A future cache (Arc<RwLock<f64>> updated on
   settings change) cleans this up.
4. **Input DTO consolidation.** `SyncSessionIdInput`,
   `SyncSeekInput`, `SyncSetSpeedInput` could collapse into a single
   `SyncTransportInput { sessionId, command }` once
   `SyncTransportCommand` is registered with tauri-specta. Keep
   per-intent for v1; revisit if v2's shape pressures it.
5. **`getOverlap` length-validation in CommandLayer.** Currently
   service-level only. Adding it at the command's input shape would
   surface the constraint earlier — minor DRY win.
6. **Persist per-pane volume / mute back to `sync_session_panes`.**
   v1 keeps these in frontend state only; v2's resume-session UI will
   want the persistence.
7. **Asset-protocol scope narrowing** (already a Phase-7 follow-up
   per ADR-0019; mentioned again because the multi-view page exposes
   2× the surface that loads from `assetUrl`).
8. **Live transport-applied event topic.** If a v2 listener cares
   about play / pause / seek / speed transitions across windows,
   re-introduce a dedicated `sync:transport_applied` event with a
   discriminant payload. The plumbing is preserved for that.
9. **Latency-aware sync** (out of v1 scope) — audio buffer
   compensation for clock drift between hardware audio and video
   pipelines.
10. **Multi-pane support (>2 panes), PiP layout, crossfader** — all
    tracked in ADR-0021 / 0023 follow-up sections.

## Recommendation

Merge once CI is green on all 3 OS. The branch is reversible by `git
revert` of the squash; both migrations are forward-only but
purely additive (new tables + new columns with defaults). The
cleanest revert path would be:

1. revert the squash → schema regression test catches the
   `LATEST_SCHEMA_VERSION` mismatch
2. apply a follow-up `0012_drop_sync_*.sql` migration

…but that path is documented in the data-model header for symmetry,
not because we expect to use it.

The follow-ups in §"Open follow-ups" are real-but-non-blocking;
none introduce a regression in the merged state. The two P0
findings from code review and the two MEDIUM findings from
security review are all resolved inline (`2529403`).

— Phase 6 Engineering
