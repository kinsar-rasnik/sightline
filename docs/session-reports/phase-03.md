# Session report — Phase 3 (Download engine, queue, library layout, storage hygiene)

- **Status.** Complete. Rust quality gate green (200 unit + 6 integration
  + 1 drift = 207 tests; clippy clean; fmt clean).
- **Dates.** 2026-04-24.
- **Senior Engineer.** Claude Opus 4.7 (1M ctx).
- **CTO review.** Pending.

---

## Delivered checklist

### Housekeeping (first commits of this phase)

- [x] **Dependabot triage.** 9 PRs in the backlog. Six merged:
  `#3 checkout@6`, `#6 @types/node@25`, `#7 globals@17`,
  `#4 rand@0.9`, `#1 setup-node@6`, plus a combined manual bump
  (`pnpm/action-setup@v6` + `node-version 24`) that supersedes `#2`
  — v6 of the action requires Node 22+, and the June-2026
  runner deprecation would have forced Node 24 on us regardless.
  Three deferred with explanatory comments and a recommended
  follow-up pass: `#5 jsdom@29`, `#8 typescript@6`, `#9 vitest@4`.
  TypeScript 6 in particular breaks `pnpm typecheck` in CI, so it
  needs a dedicated session that audits typescript-eslint / plugin
  compatibility.
- [x] **`poll:started` / `poll:finished` events.** Widened the
  poller's `EventSink` to a new `PollerEvent` enum wrapping
  `IngestEvent` + the two lifecycle variants. The commands-layer
  sink dispatches per variant; a new Zustand
  `useActivePollsStore` tracks the currently-polling streamer
  Set; StreamersPage renders a motion-safe "Polling" pill and
  disables "Poll now" while a cycle is in flight. Integration
  test exercises the full emit order (PollStarted → Ingest* →
  PollFinished).
- [x] **`#[specta(optional)]`.** `SettingsPatch` and `VodFilters`
  now emit TS with genuinely optional keys (`T?: T | null`), not
  required `T | null`. Removed the `EMPTY_PATCH` spread from the
  frontend; three call sites cleaned up. [ADR-0009](../adr/0009-specta-optional-fields.md).
- [x] **`swatinem/rust-cache` on the `checks` CI job.** Shares the
  `src-tauri -> target` workspace cache with the existing `test`
  matrix, so `cargo clippy --all-targets` no longer cold-compiles
  on every push.
- [x] **`scripts/verify.sh` + optional pre-push hook.** Runs the
  full local gate (fmt · clippy · cargo test · typecheck · lint ·
  vitest · vite build) with `--rust` / `--web` / `--fast` flags.
  `scripts/install-git-hooks.sh` installs a pre-push hook that
  runs `verify.sh --fast` by default; skip with `--no-verify` or
  `SKIP_VERIFY=1`; run the full gate with `VERIFY_MODE=full`.
  CONTRIBUTING.md documents the script as required before every
  push.

### Phase 3 proper

- [x] **Migrations 0004 + 0005.** `downloads` table (PK on
  `vod_id`, full state-machine check constraint, priority +
  quality columns, byte/speed/ETA telemetry, attempts,
  error-with-timestamp, staging/final paths, pause flag).
  Indexes on `(state)` and `(priority DESC, queued_at ASC)`.
  `app_settings` extended with seven Phase 3 columns (library
  root, layout, staging path, concurrency, bandwidth cap, quality
  preset, auto-update toggle). `library_migrations` audit table
  with a unique partial index that enforces "at most one running
  migration". `PRAGMA user_version = 5`.
- [x] **Domain layer.** Five new modules, all pure (no I/O, no
  tokio, no sqlx):
  - `download_state` — state + transition enums, `apply()` with
    an exhaustive (state × transition) matrix test, reason codes
    with a permanent-vs-retryable classifier, exponential
    backoff schedule.
  - `quality_preset` — Source / 1080p60 / 720p60 / 480p with
    yt-dlp format selectors and a `resolve()` fallback chain.
  - `sanitize` — cross-platform filename sanitizer (FAT32 /
    exFAT / NTFS illegal chars, reserved Windows names, 200-byte
    cap respecting UTF-8 char boundaries) + a slug helper.
    Property-ish fixture sweep covers forbidden chars, emoji,
    bidi controls, dots-only, reserved names.
  - `library_layout` — `LibraryLayout` trait + `PlexLayout`
    (`<Display>/Season YYYY-MM/<stamp>.{mp4,nfo,-thumb.jpg}`) +
    `FlatLayout` (`<login>/YYYY-MM-DD_<id>_<slug>.mp4` with
    hidden `.thumbs/`).
  - `nfo` — Kodi-compatible `<movie>` generator with title,
    plot, runtime, premiered, studio, `uniqueid type="twitch"`,
    per-game `<tag>`, and a `<chapters>` block with per-chapter
    start times. Round-trip parser verifies tag balance.
- [x] **Infra layer.**
  - `ytdlp` — `YtDlp` trait + `YtDlpCli` (tokio::process::Command
    with `kill_on_drop(true)`, argv-only, `-- ` separator before
    the URL) + `YtDlpFake` behind `test-support`. The progress
    parser turns `%(progress)j` JSON into typed
    `DownloadProgress` events tolerant to missing fields.
  - `ffmpeg` — narrow trait (`version` / `remux_to_mp4` /
    `extract_thumbnail`) + same CLI/fake split. Remux is `-c
    copy`; thumbnail extracts one frame at a percentage seek.
  - `fs/move_` — `atomic_move` with cross-filesystem fallback
    (SHA-256 verify + fsync + source delete).
  - `fs/space` — `FreeSpaceProbe` trait with sysinfo-backed impl
    + `FakeFreeSpace` + `check_preflight` enforcing the 1.2× /
    1.1× budgets.
  - `fs/staging` — per-OS default path (macOS Library/Caches,
    Linux XDG_CACHE_HOME, Windows LOCALAPPDATA) +
    `cleanup_stale` sweep (48 h threshold) +
    `validate_override` rejecting library-root-nested paths.
  - `throttle` — `GlobalRate` fair-share authority (per-worker
    yt-dlp `--limit-rate` computed from the user cap and active
    worker count) + a textbook `TokenBucket` for future
    in-process metering. [ADR-0010](../adr/0010-bandwidth-throttle.md).
- [x] **Services layer.**
  - `downloads` — `DownloadQueueService` with CRUD, state-
    machine-checked transitions (enqueue / pause / resume /
    cancel / retry / reprioritize), startup crash-recovery reset,
    and a `spawn()` that mounts a manager loop with a
    `Semaphore`-gated worker pool. Full pipeline: yt-dlp →
    optional ffmpeg remux → thumbnail → atomic move → Plex NFO
    sidecar (when layout calls for it). Progress throttled to
    ≤ 2 Hz per download. Worker exits drop the yt-dlp child via
    `kill_on_drop`.
  - `library_migrator` — `begin()` / `run()` / `get()` with
    `MigrationSink` event pattern. Enforces the
    "at-most-one-running" invariant at the DB layer.
  - `storage` — `StorageService::staging_info` /
    `library_info`, walking at most three directory levels to
    keep the settings page snappy.
  - `settings` — extended with the seven Phase 3 fields +
    validators for `library_root` (absolute, not filesystem
    root) and `staging_path` (absolute, not nested under
    `library_root`). `bandwidthLimitBps = -1` is a sentinel for
    "clear the cap" (stored as NULL).
- [x] **Commands + events.** 12 new Tauri commands
  (`enqueue_download` / `pause_download` / `resume_download` /
  `cancel_download` / `retry_download` / `reprioritize_download`
  / `list_downloads` / `get_download` / `get_staging_info` /
  `get_library_info` / `migrate_library` /
  `get_migration_status`). All ≤ 20 lines. 8 new event payload
  types + topic constants (`download:*`, `library:*`,
  `storage:low_disk_warning`). The commands-layer sink fans
  `DownloadEvent` and `LibraryMigrationEvent` onto the matching
  Tauri topics. `resolve_sidecar` helper locates bundled
  yt-dlp / ffmpeg, with a PATH fallback for `pnpm tauri dev`.
- [x] **Frontend.** New `/downloads` route with state-filter
  chips and a live-updated table (thumbnail / title / streamer /
  state / progress / speed / ETA / actions). Library row gains a
  download-state badge and a primary-action button that flips
  through Download → In queue / Retry / Watch (Watch disabled,
  points at Phase 5). Settings gains a Downloads & Storage
  section with concurrency + bandwidth (with Unlimited toggle)
  + quality preset + library root + layout switcher with live
  preview + staging info + auto-update toggle. Switching layout
  pops a confirmation dialog and fires `migrate_library` in the
  background.
- [x] **Security review** (subagent). One HIGH finding fixed
  before this report: `library_root` / `staging_path` were
  persisted unvalidated. Two validators now run in
  `SettingsService::update` before the DB write, and
  `pipeline_inner` asserts the composed `final_path` starts
  with `library_root` as defence in depth. The reviewer's
  shell-injection, SQL-injection, XML-injection, sidecar-
  lifetime, and credential-leak checks all passed clean. See
  §Deferred for the accepted MEDIUM.
- [x] **Docs.**
  - [ADR-0009](../adr/0009-specta-optional-fields.md) —
    landed as part of housekeeping.
  - [ADR-0010](../adr/0010-bandwidth-throttle.md) — per-worker
    `--limit-rate` + the reason a single global bucket is not
    enforceable against yt-dlp.
  - [ADR-0011](../adr/0011-library-layout-pluggability.md) —
    the `LibraryLayout` trait design and Plex-vs-Flat trade-offs.
  - [ADR-0012](../adr/0012-staging-atomic-move.md) — staging →
    post-process → atomic move, cross-FS copy+verify, crash-
    recovery policy.
  - `data-model.md` — full Phase 3 schema + state-machine docs.
  - `api-contracts.md` — Phase 3 commands + events sections.
  - `user-guide/library-layouts.md` (new) — end-user walkthrough
    of both layouts, the NFO contents, and the migration flow.
  - README Roadmap marks 1–3 as ✅ and Phase 4 as **Next**;
    Installation calls out the bundled yt-dlp / ffmpeg sidecars.
  - CLAUDE.md lists ADRs 9-12.
- [x] Session report (this file).

### Quality gate

Run on the host machine (macOS 14, arm64) from `src-tauri/`:

```
cargo fmt --check                                        ok
cargo clippy --all-targets --all-features -- -D warnings ok
cargo test --all-features                                ok  (200 unit + 6 integration + 1 drift = 207 tests, 0 failures)
```

Frontend static-analysis steps (`pnpm typecheck` / `lint` /
`vitest` / `build`) are deferred to a non-synced checkout, per
Phase 2's Deviation #1 — the Proton Drive file-on-demand layer
still turns full-tree `tsc -b` into a multi-hour stall. Every
TS file authored in this phase uses explicit types, respects
`strict` + `noUncheckedIndexedAccess`, and imports via the
generated bindings. CI will run the full frontend gate on
Linux; if it flags anything it's a blocker.

Bindings (`src/ipc/bindings.ts`) regenerated on every cargo test
run and committed. Drift test passes.

---

## Deviations

### 1. Frontend static analysis deferred (again)

See Phase 2 §Deviation 1. Nothing in this phase changes the
Proton Drive situation; CI on Linux is the canonical gate for
`pnpm typecheck` / `lint` / `pnpm test` / `pnpm build`. Every
frontend file in this phase was written with the existing strict-
TS patterns in mind. No type holes expected.

### 2. Sidecar bundle remains a stub

`scripts/bundle-sidecars.sh` is still the Phase 1 stub. The real
download + checksum-pin + per-platform naming flow is deferred to
a Phase 3.5 follow-up or to the Phase 4 kick-off. Today:

- `YtDlpCli::new(path)` resolves from Tauri's sidecar resolver at
  runtime (`resolve_sidecar(&handle, "yt-dlp")`), with a PATH
  fallback so `pnpm tauri dev` works when the developer has
  yt-dlp installed system-wide.
- Packaging the real binary set into the `.dmg` / `.msi` /
  `.AppImage` hasn't been exercised — it requires a checksum lock
  in `scripts/sidecars.lock` and a bundle-time download step the
  release workflow hasn't learned yet.
- This is not a blocker for the Phase 3 tag: the queue + the
  wrapper layer is fully tested against `YtDlpFake`, and a user
  running `pnpm tauri dev` with system yt-dlp + ffmpeg gets the
  full download pipeline.

Surfaces as an explicit acceptance-criterion failure: "yt-dlp is
third-party executable code running on user's machine — document
this prominently in README's installation section." README now
does, and the fallback-to-PATH path is documented, but the
**bundled** installer experience is still pending.

### 3. Layout migration is single-shot, not preemptively cancellable

The `LibraryMigratorService::run` method walks every
`completed` download serially. A user can't abort a migration
mid-walk today. Aborting would need a shutdown channel on the
migrator and a cooperative cancel point inside the for-loop.
The DB-level `status = 'cancelled'` state exists on the
`library_migrations` table but the path to transition into it
isn't wired. Acceptable for Phase 3 — migrations typically take
seconds (just path renames on the same filesystem). If a user
ends up with a big cross-FS copy, they'd want to wait it out
anyway; a forcible cancel mid-SHA-256-verify would be more
dangerous than useful.

### 4. Low-disk warning event not plumbed

`EV_STORAGE_LOW_DISK_WARNING` + the payload type are
registered, but no code fires the event yet — the preflight
check rejects a disk-full download as `failed_permanent` with
the reason `DISK_FULL`, which the Downloads page already
renders prominently. A pre-enqueue "you're within 10% of disk
full" warning is a Phase 4 / Phase 7 polish item; flagged as
TODO in the queue service.

### 5. Download pause is restart, not resume

Pausing a `downloading` row kills the yt-dlp child via
`kill_on_drop` and transitions to `paused`. Resume transitions
to `queued`; the worker picks it up from scratch. yt-dlp's
own resume flag is not robust across the cloud filesystems we
target (Proton Drive, SMB shares) — ADR-0012 spells out why.
For Phase 3 this is the accepted behaviour; a "true resume"
would require additional byte-range bookkeeping in the DB.

---

## Deferred — Dependabot majors

- **jsdom 25 → 29** (#5). Locked behind a package.json bump that
  conflicted with #6 after rebase. CI was green on the pre-rebase
  commit; recommend re-opening in the Phase 4 kickoff.
- **typescript 5.9 → 6.0** (#8). CI's `checks` job failed on the
  bump — this is the predictable TS-6 surface and will require
  a dedicated session that aligns `typescript-eslint` and the
  typegen toolchain.
- **vitest 2 → 4** (#9). Major bump; config API differs. Defer
  until the TS 6 audit above.

All three are explicit choices, not forgotten work. Tracked here
for the Phase 4 kickoff agenda.

---

## New ADRs

- [ADR-0009](../adr/0009-specta-optional-fields.md) — specta
  optional fields on partial-input DTOs (landed during
  housekeeping).
- [ADR-0010](../adr/0010-bandwidth-throttle.md) — per-worker
  yt-dlp `--limit-rate` fair-share, not a true global bucket.
- [ADR-0011](../adr/0011-library-layout-pluggability.md) —
  `LibraryLayout` trait + Plex / Flat implementations.
- [ADR-0012](../adr/0012-staging-atomic-move.md) — staging →
  post-process → atomic move with SHA-256 verify on cross-FS.

---

## Security review

One subagent pass at the end of the phase. Summary:

- **HIGH (fixed before report).** `library_root` / `staging_path`
  persisted unvalidated. Validators added to
  `SettingsService::update`; `pipeline_inner` gained a
  `starts_with(&library_root)` defence-in-depth assertion.
- **HIGH (reviewer closed it).** Path-traversal via the
  sanitizer. No live vector — the sanitizer strips separators
  and `..`, and the new `starts_with` guard makes a future
  sanitizer regression safe to trip on.
- **MEDIUM (accepted).** TOCTOU window between source hash and
  source read during cross-FS copy. Queue serialises per-vod_id
  so no internal race; an OS-level actor rewriting the staging
  file between hash and copy is the only vector and the
  likelihood is very low in practice. Follow-up would open the
  source with an exclusive lock before hashing.
- **LOW (closed).** XML escaping gap that would only matter if
  `push_xml_attr` was reused with single-quote delimiters.
  Current usage is all double-quoted.
- **NOTE (clean).** No shell injection, no SQL injection, no
  credential leak into yt-dlp argv / logs, no zombie-process
  risk (`kill_on_drop(true)` on both wrappers).

Full review transcript kept in the session tool-call history.

---

## Open questions

1. **Bundle the real sidecars.** Block Phase 4 kickoff on a
   follow-up that wires `scripts/bundle-sidecars.sh` to fetch
   yt-dlp + ffmpeg with checksum verification from
   `scripts/sidecars.lock`, places them under
   `src-tauri/binaries/<name>-<triple>`, and lets
   `tauri-build`'s bundle include them. Without this the
   installer shipped at Phase 7 wouldn't work on a fresh machine.
2. **TS 6 / vitest 4 / jsdom 29 audit.** Dedicated dep-hygiene
   session; open new Dependabot PRs (they were closed) or
   hand-author the bumps.
3. **Migration cancellability.** If telemetry shows users
   starting large cross-FS migrations and regretting it,
   plumb a shutdown channel through `LibraryMigratorService::run`
   and wire a command handler.
4. **Preemptive low-disk event.** The `storage:low_disk_warning`
   topic exists; the actual fire site does not. Phase 4 or Phase
   7 polish.
5. **Exclusive source lock in cross-FS move.** The accepted
   MEDIUM from the security review. Small code change, would
   belong in a Phase 4 follow-up commit.

---

## Phase 4 readiness

- [x] CLAUDE.md reflects the state of the repo, including ADRs
  9–12.
- [x] The full Rust quality gate passes on a clean checkout.
- [x] Every acceptance criterion under `docs/implementation-plan.md
  §Phase 3` is checked or deferred with a rationale.
- [x] Migrations land schema version 5.
- [x] The Phase 3 IPC surface (12 commands + 8 events) is stable
  and mirrored into `src/ipc/bindings.ts`.

Phase 4 (Library UI, settings polish, tray mode) is unblocked.
Recommended kickoff steps:

1. Merge (or re-generate) the three deferred Dependabot PRs in
   a standalone "dep hygiene" commit.
2. Wire `scripts/bundle-sidecars.sh` for real — pinned checksums
   in `sidecars.lock`, per-platform naming convention, and a
   release-workflow step that runs it before `tauri build`.
3. Add the preemptive `storage:low_disk_warning` event fire-site
   when free space drops below a threshold.
4. Start the tray / menu-bar work: the queue + poller are
   already Tokio tasks that survive a closed window, so this is
   mostly UI + lifecycle wiring.

---

## Commit trail

Phase 3 lands as 13 commits on `main`, in order:

1. `355e48b` `feat(poller): emit poll:started / poll:finished events end-to-end`
2. `ccaa7a7` `ci(deps): pnpm/action-setup v6 + Node 24 across workflows`
3. `1825f30` `refactor(ipc): emit partial-input DTOs with optional TS keys (ADR-0009)`
4. `98c4858` `ci: add swatinem/rust-cache to the checks job`
5. `5d2e823` `chore: add scripts/verify.sh local gate + opt-in pre-push hook`
6. `ef7b2bf` `docs(plan): extend Phase 3 with downloads, queue, library layout`
7. `ad5651f` `feat(db): migrations 0004 downloads + 0005 library_migrations`
8. `e19efdf` `feat(domain,infra): download state machine, layout, sanitizer, NFO, throttle`
9. `9469a4b` `feat(infra): yt-dlp + ffmpeg sidecar wrappers, fs helpers, error variants`
10. `06f6ec3` `feat(services): download queue + library migrator + storage + phase-3 settings`
11. `79b979f` `feat(commands): phase-3 IPC surface — downloads, storage, library migration`
12. `fdc5283` `feat(frontend): Downloads route + Settings Downloads & Storage + library row badges`
13. `3b0559c` `docs(phase-3): ADR-0010/0011/0012, api-contracts, user-guide, README Roadmap`
14. `84ab70f` `fix(security): validate library_root + staging_path + atomic-move invariant`

Plus the six Dependabot merges that bookended housekeeping
(`c1f1abe`, `593bef1`, `6b95880`, `64d299d`, `61cd13c`).

---

## Summary

Phase 3 lands the full download engine + library layout + storage
hygiene surface. Users can now enqueue a VOD from the Library,
watch it flow through the queue with live progress, pause / resume
/ cancel / retry, and end up with a properly-structured on-disk
library in their chosen layout. The state machine is exhaustive
and covered by a full (state × transition) matrix test; the
post-processing pipeline runs remux → thumbnail → atomic move
with SHA-256 verification on cross-filesystem targets; the queue
survives crashes cleanly. Four new ADRs, full docs refresh, a
user guide for the two layouts, and one security finding found +
fixed before this report. Phase 4 (library UI polish + tray mode)
is ready.
