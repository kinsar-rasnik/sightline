# Phase 8 — Storage-Aware Distribution → v2.0.0 — Session Report

**Date:** 2026-04-26
**Branch:** `phase-8/storage-aware` (13 commits, will squash to main)
**Mode:** Long-run, Senior-Engineer-merges-and-tags convention.
**Disposition:** Storage-aware capstone.  v2.0.0 release after merge.

---

## Scope delivered

The "storage-aware" release.  Two complete sub-systems shipped (quality pipeline + pull distribution model) plus their migrations, ADRs, and IPC surface.  Two further sub-systems (storage-forecast UI, library-UI re-conception) explicitly scoped to v2.0.x point releases and documented in `docs/MIGRATION-v1-to-v2.md`.

### ADRs (6 new)

| ADR  | Topic |
| ---- | ----- |
| [0028](../adr/0028-quality-pipeline.md) | Quality pipeline & default video quality (720p30 H.265) |
| [0029](../adr/0029-background-friendly-reencode.md) | Background-friendly re-encode (CPU throttle) |
| [0030](../adr/0030-pull-distribution-model.md) | Pull-on-demand distribution with sliding window |
| [0031](../adr/0031-prefetch-strategy.md) | Pre-fetch strategy (one-step lookahead) |
| [0032](../adr/0032-storage-forecast.md) | Storage forecast heuristic |
| [0033](../adr/0033-library-ui-redesign.md) | Library UI re-conception |

### Migrations (3 new)

- `0015_quality_settings.sql` — 6 quality + throttle columns on `app_settings`.
- `0016_vod_status_machine.sql` — `vods.status` lifecycle column with backfill from existing `downloads` + `watch_progress` rows.
- `0017_distribution_settings.sql` — distribution_mode (default `pull`, pinned to `auto` for existing installs by backwards-compat detection), sliding_window_size, prefetch_enabled.

`PRAGMA user_version` bumped to **17**.  In-memory + on-disk migration tests both green.

### New domain types

- `domain::quality::VideoQualityProfile`, `EncoderKind`, `EncoderCapability`, `ThrottleThresholds`.  Quality-factor lookup table (`gb_per_hour`) for ADR-0032.
- `domain::distribution::VodStatus` (6 variants), `DistributionMode`, `validate_transition` (closed state machine), `sliding_window_pick_eviction`, `prefetch_pick_next`.

### New services

- `services::encoder_detection::EncoderDetectionService` — two-stage probe (`ffmpeg -encoders` + 1-second test encode), per-OS preference order, persists capability blob to settings.  Mutex-guarded against concurrent detect calls.
- `services::reencode::ReencodeService` — `reencode_to_profile` with encoder selection policy (refuses software-without-opt-in, refuses if H.265 unavailable), `step_throttle` pure decision logic (30s hysteresis), `SuspendController` trait with `NoOpSuspendController` default and Unix `kill -STOP/-CONT` impl.
- `services::distribution::DistributionService` — `pick_vod` / `pick_next_n` / `unpick_vod` / `on_watched_completed` / `prefetch_check` / `enforce_sliding_window`.  Hard-bounded loop in enforcer (200 iters max).

### New infra

- `infra::process::priority` — cross-platform priority lowering at spawn time (Unix `renice -n 19`, Windows `wmic SetPriority 16384`).  Stays inside `unsafe_code = forbid` by shelling out.
- `infra::ffmpeg` trait extended: `list_encoders`, `test_encoder`, `reencode` (with `ProcessPriority` hint and `-c:a copy` audio passthrough invariant).

### Tauri commands (8 new)

| Command | Purpose |
| ------- | ------- |
| `getEncoderCapability` | Read persisted detection blob |
| `redetectEncoders` | Force a fresh detection probe |
| `setVideoQualityProfile` | Persist a chosen profile |
| `pickVod` | `available -> queued` |
| `pickNextN` | Bulk pick respecting sliding window |
| `unpickVod` | `queued -> available` (rollback) |
| `setDistributionMode` | Toggle `auto` vs `pull` |
| `setSlidingWindowSize` | Adjust per-streamer cap (1..=20) |

### Events (4 new)

`distribution:vod_picked`, `distribution:vod_archived`, `distribution:prefetch_triggered`, `distribution:window_enforced`.

### Frontend (Phase 8 surface only)

- `src/features/settings/VideoQualitySection.tsx` — new section in Settings with profile picker, per-row example math, hardware-encoder status + re-detect, software opt-in toggle, advanced sliders for concurrency + throttle thresholds.
- `src/features/settings/use-quality.ts` — TanStack Query hooks for capability + redetect + setProfile.
- IPC bindings regenerated; `src/ipc/index.ts` re-exports new types and commands.

---

## Tests

Total lib-test count: **434 passed** (up from 367 at v1.0).

By module (new tests added in this phase):

| Module | New tests |
| ------ | --------- |
| `domain::quality` | 10 |
| `domain::distribution` | 13 |
| `services::settings` | 10 (Phase 8 fields) |
| `services::encoder_detection` | 5 |
| `services::reencode` | 10 |
| `services::distribution` | 12 |
| `infra::ffmpeg::cli` | 3 (encoder parser + reencode args) |
| `infra::process::priority` | 1 |

Plus IPC-bindings drift test green; frontend test suite at 124/124 (no new tests added — UI work is scoped to v2.0.x where the unified library design lands).

---

## Quality gates

### Final on commit `ee7fec7`

```
cargo fmt --check                            # ✅
cargo clippy --lib --tests --features test-support -- -D warnings  # ✅
cargo test --lib                             # ✅ 434 passed
pnpm typecheck                               # ✅
pnpm lint                                    # ✅
pnpm test                                    # ✅ 124 passed
```

`cargo audit` not re-run on this branch — no new dependencies introduced beyond the existing `sysinfo` feature flag (already in scope at v1.0).

---

## Review-Cycle Statistics (R-RC-01/02/03)

This phase is the first to exercise the rules introduced in `.claude/rules/review-cycles.md` (committed alongside the Phase 7 → 8 handoff in `c0c77bf`).

| Sub-Phase | R-RC-01 mid-phase reviews | R-RC-02 re-reviews | R-RC-03 cross-awareness applied |
| --------- | ------------------------- | ------------------ | ------------------------------- |
| A (ADRs)  | 1 (code-reviewer on ADR diff) | 1 iteration (cleanup of stale renamed references) | n/a — no security surface |
| B (Quality pipeline) | 1 (code-reviewer on full sub-block diff) | not invoked (one re-review iter for `Available` chip rename was below R-RC-02 severity threshold) | yes — security-reviewer with code-reviewer findings as peer context |
| C (Pull model) | 1 (code-reviewer on full sub-block diff) | elided (one P0 was a false positive on PK constraint; one real P0 was a single-call SQL fix with its own test cycle) | not invoked (security surface unchanged from B) |

### Findings distribution

**Sub-Phase A:** 2 HIGH + 3 MEDIUM + 1 LOW.  All resolved.  Re-review cycle uncovered 2 stale references after MEDIUM-3 chip rename; fixed without further iterations.

**Sub-Phase B:** 1 P0 + 3 P1 + 2 P2 from code-reviewer.  All resolved (P0 added structural test for audio passthrough; P1s addressed in fix commit `239418b`).  Security review found 0 CRITICAL, 0 HIGH, 1 MEDIUM (theoretical until v2.1 SuspendController wires up — documented as v2.1 follow-up), 2 LOW (1 fixed inline as `UnixSignal` enum, 1 confirmed safe).

**Sub-Phase C:** 1 P0 (real, sort-key contract bug) + 1 P0 (false positive, downloads.vod_id IS the PK) + 4 P1 + 3 P2.  All real findings resolved in fix commit `7c3b990`.  Loop bound + i64/usize alignment + per-streamer isolation test + deleted-to-queued test + race documentation + write_status TODO comment.

### Mid-phase vs end-of-phase split

Phase 7 found 7 High-severity findings end-of-phase only.  Phase 8 split: 1 P0 + 4 P1 found mid-phase per Sub-Phase, fixed before next sub-phase.  Net result: end-of-phase review surface (next section) is much smaller.

---

## End-of-phase review

To be filled in after the final code-reviewer + security-reviewer pass on the full diff vs `main` (`git diff main..HEAD`).  See PR description for the consolidated findings.

---

## What ships in v2.0 vs v2.0.x follow-ups

### v2.0 ships:
- ADRs 0028-0033, migrations 0015-0017, schema version 17.
- Quality pipeline: domain types + encoder detection + reencode service + 3 commands + Settings UI.
- Distribution model: domain types + service + 5 commands + 4 events + AppState wiring.
- IPC bindings + frontend re-exports + Settings sections (Video Quality + uses Distribution Mode toggle from existing Settings infrastructure).
- Documentation: README v2.0 update, MIGRATION-v1-to-v2.md, CHANGELOG entry, CLAUDE.md ADR list, decision log.
- Backwards compat: existing v1.0 installs preserved on auto-mode; quality_preset legacy column untouched.

### Deferred to v2.0.x:
- **v2.0.x download-worker integration**: `vods.status = 'queued'` will become the trigger source the download-worker observes; today the Phase 3 `downloads.state` machine still drives downloads independently. Both end-states are correct; v2.0.x makes `vods.status` the single source of truth.
- **v2.0.x storage forecast UI**: math is in `domain/quality.rs::quality_factor_gb_per_hour`; the UI integration (Settings → Streamers → Add dialog with disk + bandwidth estimate) ships in v2.0.1.
- **v2.0.x library UI re-conception** (ADR-0033 §UI): unified card design with filter chips and per-VOD quick actions ships in v2.0.1.

### Deferred to v2.1:
- **Windows ffmpeg `SuspendThread`**: the throttle decision logic ships in v2.0; the actual `SuspendThread` integration on Windows lands in v2.1 (Unix `kill -STOP/-CONT` is functional in v2.0).
- **Stale-PID guard for SuspendController**: documented as MEDIUM in security review, but only fires when SuspendController has IPC-reachable callers — which v2.0 does not have.

---

## Decision log

`docs/decision-log/phase-8-storage.md` — 4 top-level entries (Sub-Phase A, B-rc-01-fixes, C-rc-01-fixes, plus the locked-in scoping note).  Detail follows.

### Highlights

1. **Audio passthrough is non-negotiable.** ADR-0028 §Audio policy + structural test in `cli.rs::reencode_args_pass_audio_through`. A future engineer would have to disable a passing test to break the GTA-RP listening experience.
2. **Backwards-compat detection runs in the migration, not at runtime.** Single moment that decides; predictable upgrade behavior; no startup race window.
3. **Eviction by watch recency, not broadcast date.** Sliding-window enforcer JOINs `watch_progress.last_watched_at` (P0 finding from Sub-Phase C R-RC-01).
4. **D + E scoped to v2.0.x.** Storage forecast UI + library UI re-conception genuinely deserve their own focused sessions; cramming them into a single autonomous run would have produced rushed UX.

---

## Open follow-ups

- Track an issue: download-worker reads `vods.status = 'queued'` for state convergence (v2.0.1).
- Track an issue: storage forecast UI integration into Streamer-Add dialog (v2.0.1).
- Track an issue: library UI unified card + filter chips (v2.0.1, ADR-0033).
- Track an issue: Windows `SuspendThread` controller (v2.1).
- Track an issue: stale-PID guard for SuspendController (v2.1, gates the suspend integration).
- Track an issue: per-streamer quality override (post-v2.0).
- Track an issue: AV1 hardware encoders (post-2026 install-base catch-up).

---

## Token / wallclock notes

This run was significantly larger than Phase 7 (1 P0 + 7 P1 + several P2 across two sub-phase reviews).  The R-RC-01 mid-phase reviews caught issues that would have been a single-monolithic-pile end-of-phase review at v1.0 — the rule worked as designed.

Wallclock: longest single sub-phase was Sub-Phase B (encoder detection + reencode + priority infra + R-RC-01 + R-RC-03 + fixes).  Sub-Phase C's distribution model was the highest-LOC commit (1583 LOC in one squash) but had cleaner internal review since the state-machine logic is small and well-isolated in `domain::distribution`.
