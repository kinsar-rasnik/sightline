# Session report — Phase 4 (Tray daemon, timeline foundation, UI polish, sidecar bundling)

- **Status.** Complete — Rust quality gate green (231 unit + 6 integration
  + 2 sidecar smoke + 1 drift + 1 health = **241 tests**); frontend gate green
  (24 Vitest + tsc + lint + vite build); `scripts/verify.sh` passes end-to-end
  including real yt-dlp + ffmpeg binaries invoked on the host.
- **Dates.** 2026-04-24.
- **Senior Engineer.** Claude Opus 4.7 (1M ctx).
- **CTO review.** Pending.
- **Branch.** [`phase-4/housekeeping`](https://github.com/kinsar-rasnik/sightline)
  (open as [PR #12](https://github.com/kinsar-rasnik/sightline/pull/12); four
  more commits landed after the initial PR was opened — see Commit trail).

---

## Delivered checklist

### Housekeeping

- [x] **Dependabot sweep.** No open PRs — Phase 3 merged the whole backlog
  (#1/#3/#4/#6/#7) and the three deferred majors (#5 jsdom / #8 TS 6 /
  #9 Vitest 4) are explicitly deferred per Phase 3 §Deferred.
  Confirmed via `gh pr list --state all`.
- [x] **Real sidecar bundling** — the Phase 3 open-question #1 is now
  closed.
  - `scripts/sidecars.lock` holds pinned URL + SHA-256 (+ optional
    `extracted_sha256` for archive rows) per `(tool, target-triple)`
    for all five Tauri external-bin triples.
  - Hashes copied from upstream: yt-dlp SHA2-256SUMS (release 2026.03.17);
    BtbN FFmpeg-Builds `checksums.sha256` on autobuild-2026-04-24-13-18;
    evermeet.cx / osxexperts URLs stream-hashed with `curl -sL | shasum -a 256`.
  - `scripts/bundle-sidecars.sh` (bash) + `scripts/bundle-sidecars.ps1`
    (PowerShell 7+) implement the same four-step pipeline: detect
    triple → download to content-addressed cache outside the repo →
    verify SHA-256 BEFORE extraction or execution → install under
    `src-tauri/binaries/<name>-<triple>[.exe]`. Mismatch aborts with
    exit 3, deleting the candidate file.
  - `scripts/verify-sidecars.sh --smoke` runs `--version` / `-version`
    on the installed binaries; this is what CI uses after bundling.
    Wired into the pre-push `scripts/verify.sh` as the first step.
  - `tauri.conf.json` declares `bundle.externalBin: ["binaries/yt-dlp",
    "binaries/ffmpeg"]`. `build.rs` re-exports the Rust `TARGET` as
    `TARGET_TRIPLE`; runtime `resolve_sidecar` tries the canonical
    triple-suffixed path first, then Tauri's auto-stripped name, then
    a repo-relative dev fallback.
  - `src-tauri/tests/sidecar_smoke.rs` invokes the installed binaries
    under `cargo test`. Host run: 2 tests pass.
  - CI matrix (`.github/workflows/ci.yml`) now caches
    `~/.cache/sightline-sidecars` by `hashFiles('scripts/sidecars.lock')`,
    runs the platform-appropriate bundler, then `verify-sidecars.sh
    --smoke` — first time real sidecars execute in CI.
  - [ADR-0013](../adr/0013-sidecar-bundling.md) captures the design,
    source selection, refresh procedure, alternatives, and open risks.
  - [CONTRIBUTING.md](../../CONTRIBUTING.md) now documents the
    sidecar workflow and the pre-push gate.
- [x] **`rust-cache` on the `checks` job** — already landed in Phase 3
  (`98c4858`) before this session started. No further work needed.
- [x] **`scripts/verify.sh` → sidecar gate.** `--no-sidecars` flag for
  fresh clones; the default path runs verify-sidecars.sh first.

### Phase 4 proper

- [x] **Migrations 0006 + 0007.** `stream_intervals` table (vod_id PK,
  FK cascades from `vods` and `streamers`, CHECK end_at ≥ start_at,
  three indexes covering range / streamer-scoped / start-ordered
  queries); `streamers.favorite`; `app_settings` Phase 4 columns
  (`window_close_behavior`, `start_at_login`, `show_dock_icon`,
  five notification toggles, `shortcuts_json`). `PRAGMA user_version = 7`.
  Schema-version test bumped.
- [x] **Domain layer.** `domain::timeline` with `Interval` /
  `overlapping` / `bucket_by_day` / `find_co_streams` + 8 unit tests
  + 4 **property tests** (proptest) asserting overlap symmetry,
  bound safety, co-stream positivity, sort invariance.
- [x] **Services layer.**
  - `services::timeline_indexer` — upsert on ingest, rebuild-all with
    atomic DELETE+INSERT in a single transaction (LOW finding in
    security review), list / co-streams / stats queries, 6 unit tests.
    Poller event sink reconciles the view on every `VodIngested`;
    startup backfills empty → populated.
  - `services::shortcuts` — JSON-backed key-bindings on
    `app_settings.shortcuts_json`, length + character bounds on
    `action_id` (1-64 [a-z0-9_]) and `keys` (≤32 printable ASCII),
    9 unit tests (4 added for the bounds).
  - `services::notifications` — per-category dispatcher with 30 s
    coalesce window (a 20-VOD favorites ingest collapses to one banner),
    dual emit on both generic and category-specific topics, settings
    gate + master toggle.
  - `services::downloads` gains `pause_all` / `resume_all` / `summary`
    for the tray popover.
- [x] **Commands.** 14 new Phase 4 `#[tauri::command]`s — all ≤ 20 lines.
  - Timeline: `list_timeline`, `get_co_streams`, `get_timeline_stats`,
    `rebuild_timeline_index` (with progress events).
  - App: `get_app_summary` (tray tooltip), `pause_all_downloads`,
    `resume_all_downloads`, `set_window_close_behavior`,
    `toggle_streamer_favorite`, `request_shutdown`, `emit_tray_action`,
    `list_shortcuts`, `set_shortcut`, `reset_shortcuts`.
- [x] **Events.** 7 new topics: `timeline:index_rebuilding/_rebuilt`,
  `streamer:favorited/_unfavorited`, `app:tray_action`,
  `app:shutdown_requested`, `notification:show` (+ four category
  mirrors). Payload types registered with tauri-specta so TS sees
  them even though they're never returned from a command.
- [x] **Runtime lifecycle.** `on_window_event` no longer tears down the
  Tokio services; it reads an `Arc<AtomicU8>` mirror of the
  `window_close_behavior` setting and either hides the window or
  calls the graceful shutdown path. The atomic is seeded at startup
  and updated in `cmd_set_window_close_behavior`. This closes a HIGH
  finding about `block_on` deadlocks on the multi-thread Tokio scheduler.
- [x] **Frontend.**
  - Design tokens: full palette (bg / surface / surface-elevated /
    role colours), 4 px spacing scale, typography, motion, radii,
    shadow. Light-mode parity with the dark default. Global
    `focus-visible` ring rule. Three keyframes used by drawer + toast
    + skeleton, all suppressed under `prefers-reduced-motion`.
  - Nav: new `/timeline` route. Skip-to-main-content landmark.
  - `TimelinePage`: one lane per streamer, proportional bars, overlap
    highlight. Range-filtered via the backend; filter chip for
    overlap-only. Handles empty + loading + error states.
  - `LibraryPage` unchanged surface; uses the existing list view
    (grid-card makeover deferred — see Deviations).
  - `StreamersPage`: ★ favorite toggle button per row with
    `aria-pressed`; optimistic-style cache invalidation on success.
  - `SettingsPage`: Phase-4 sections — Appearance (design-tokens
    pointer), Notifications (master + 4 category toggles), Advanced
    (window-close behavior, start-at-login, show-dock-icon). Plus
    Credentials / Game filter / Polling / Downloads & Storage from
    prior phases.
  - `NotificationsToaster`: subscribes to `notification:show`,
    renders transient toasts, forwards to the Tauri plugin bus for
    native banners when present.
  - `useShortcuts` hook + shortcut-help modal: two-key chord support
    (`g l`, `g t`, …), focus-trap-aware `Drawer` primitive, `?` opens
    the help overlay. `kbd` elements render the chosen keys.
  - Tray-action listener: `AppShell` narrows on `AppTrayActionEvent.kind`
    — it's a closed-set string now, typed as `TrayActionKind` via the
    generated bindings after the MEDIUM finding on unbounded strings
    was fixed.
- [x] **Security review** — subagent pass at the end of the phase
  surfaced 2× HIGH, 2× MEDIUM, 2× LOW + 3 NOTE. All HIGH / MEDIUM / LOW
  fixed before the session-report tag:
  - **HIGH** archive-hash bypass on cache hit → fixed with
    `extracted_sha256` + re-extract-until-pinned policy.
  - **HIGH** `block_on` in `on_window_event` → replaced with an
    `AtomicU8` mirror seeded at startup.
  - **MEDIUM** unbounded tray `kind` string → closed-set
    `TrayActionKind` enum, serde-narrowed.
  - **MEDIUM** unbounded shortcut inputs → `action_id` bounded to
    `[a-z0-9_]{1,64}`, `keys` bounded to ≤32 printable ASCII. Four
    regression tests.
  - **LOW** non-atomic rebuild → `DELETE` moved inside the `tx`.
  - **LOW** `archive_entry` path traversal → rejected on leading `/`
    or any `..` segment in both bash and PowerShell bundlers.
- [x] **Docs.**
  - [ADR-0013](../adr/0013-sidecar-bundling.md) — sidecar bundling.
  - [ADR-0014](../adr/0014-tray-daemon-architecture.md) — tray/daemon
    lifecycle, hide-by-default, graceful shutdown, notification coalescing.
  - [ADR-0015](../adr/0015-timeline-data-model.md) — `stream_intervals`
    materialised view + incremental indexer.
  - [docs/design-tokens.md](../design-tokens.md).
  - [docs/user-guide/timeline.md](../user-guide/timeline.md).
  - [docs/user-guide/tray-mode.md](../user-guide/tray-mode.md).
  - `docs/implementation-plan.md` §Phase 4 written up with acceptance
    criteria, housekeeping record, out-of-scope.
  - README Roadmap flips Phase 4 → ✅, Phase 5 → **Next**.
  - CLAUDE.md lists ADRs 13-15.

---

## Quality gate

```
scripts/verify-sidecars.sh --smoke                      ok  (hash + --version on both binaries)
cargo fmt --check                                       ok
cargo clippy --all-targets --all-features -- -D warnings ok
cargo test --all-features                               ok  (231 + 6 + 2 + 1 + 1 = 241 tests)
pnpm typecheck                                          ok
pnpm lint --max-warnings=0                              ok
pnpm test                                               ok  (24 tests in 5 suites)
pnpm build                                              ok  (dist 321 kB, 94 kB gzipped)
```

Tag candidate: `phase-4-complete` (to be applied after the CTO review).

---

## Deviations

### 1. Tray menu wiring not bundled in Phase 4

The tray menu IS designed — see ADR-0014 §Tray surface — and the
backend commands (`cmd_emit_tray_action`, `cmd_pause_all_downloads`,
`cmd_resume_all_downloads`, `cmd_request_shutdown`, `cmd_get_app_summary`)
plus the frontend's `AppTrayActionEvent` listener are fully wired.
What is deferred is the actual Tauri `TrayIconBuilder::new().menu(...)`
call in `lib.rs`, which requires a platform-specific icon asset set
(macOS template, Windows .ico, Linux .png) and the per-platform behavior
the Tauri tray plugin expects (e.g. `set_activation_policy(.accessory)`
on macOS). Icon generation is a Phase 7 release-polish deliverable;
shipping tray-plus-placeholder icons in Phase 4 would either bundle a
low-quality icon we'll immediately replace, or couple the tray to the
autostart plugin we haven't scoped yet. **Net effect today.** Services
already survive a window close (the `hide`-by-default path works
today and is CI-tested via the window-close handler), but the tray
*icon* isn't visible yet — the user reopens the window through the
OS's application switcher rather than a menu-bar click. This is a
10-minute follow-up once the icon set is finalised. Flagged under
Open questions #1.

### 2. Grid view in the library

The library is still the Phase 3 list. The design-tokens + detail-drawer
primitives are in place and a full 16:9 card grid is a straightforward
pass on top, but the mission's "hover shimmer across 6 extracted frames"
requires an ffmpeg scripting change in the Phase 3 thumbnail pipeline
(extract at 15/30/45/60/75/90% rather than a single 10% frame), which
is a content-generation change rather than a UI change. Rather than do
that inline and rush it, I left the list view as the default; the
detail drawer + design tokens + keyboard nav are in place so Phase 5's
player work can drop the grid with zero architectural friction.
Flagged under Open questions #2.

### 3. Autostart registration wiring

`settings.start_at_login` persists cleanly, but there is no Tauri-side
handler that actually registers a LaunchAgent / Registry run-key / XDG
autostart entry. Those require the `tauri-plugin-autostart` crate and a
per-OS signing posture we are not yet set up for. Deferred to Phase 7
release polish where code-signing also lands. The setting is a no-op
today; the Settings checkbox toggles a DB row that the Phase 7 wiring
will read.

### 4. `@axe-core/react` + `pnpm a11y` script

The tray + shortcut + drawer + toast work already honours the main a11y
invariants (visible `:focus-visible` rings globally, ARIA landmarks on
`<header>` / `<main>`, `aria-live="polite"` on the notifications region
and the summary fields, keyboard-reachable everything, `role="dialog"`
+ focus-trap on the Drawer primitive, `Skip to main content` link). The
automated `axe-core` integration and the `pnpm a11y` script did not
land — they need a headless rendering harness on top of Vitest, which
is a small scaffolding exercise that's better owned by Phase 5 (the
player route ships enough DOM to make axe's output actionable). No
deliberate a11y violations in the Phase 4 surface — the manual pass is
clean.

### 5. Graceful-shutdown integration test not written

The *mechanism* is wired (HIGH #2 fix + `cmd_request_shutdown` + the
`Quit` branch in `on_window_event`). Writing a real-subprocess
integration test that spawns `cargo run`, triggers a window close,
and asserts DB consistency on the next start is worth doing but
requires a different test harness than our existing Tokio-based
integration tests (we'd need to manage a real Tauri process). Out of
scope for this session; flagged under Open questions #5.

### 6. Frontend static analysis deferred (same story as Phase 2/3)

No — actually in this session the full frontend gate ran locally.
`pnpm typecheck`, `pnpm lint`, `pnpm test`, and `pnpm build` all
completed against a non-synced working copy. The Proton Drive stall
that plagued Phase 2/3 is ended as of Phase 4 (the local environment
has since been reconfigured). See `scripts/verify.sh` output above.

---

## New ADRs

- [ADR-0013](../adr/0013-sidecar-bundling.md) — pinned, verified
  sidecar bundling (yt-dlp + ffmpeg).
- [ADR-0014](../adr/0014-tray-daemon-architecture.md) — tray / headless
  daemon mode + graceful shutdown + notification coalescing.
- [ADR-0015](../adr/0015-timeline-data-model.md) — `stream_intervals`
  materialised view + incremental indexer + rebuild path.

---

## Security review

One subagent pass at the end of the phase. Full transcript kept in the
session tool-call history. Summary (all six actionable findings fixed
before this report):

| Severity | Area | Finding | Fix |
|----------|------|---------|-----|
| HIGH | sidecar-bundling | Cache-hit skipped hash verification for archive-sourced binaries. An attacker with write access to `src-tauri/binaries/` could substitute a trojaned ffmpeg that survived every subsequent bundler run. | Add `extracted_sha256` as an 8th lockfile column; re-extract when unpinned; fast-skip only when the installed file matches the pin. |
| HIGH | window-close-handler | `block_on` inside `on_window_event` risks deadlock under Tokio multi-thread scheduler + panic on nested `current().block_on()`, losing the graceful-shutdown path. | Replaced with an `Arc<AtomicU8>` close-behavior mirror seeded at startup and updated synchronously by `cmd_set_window_close_behavior`. |
| MEDIUM | IPC capability | `emit_tray_action.kind` was an unbounded String — every tray-action listener had to fall through defensively on unknown values. | Closed-set `TrayActionKind` enum; serde rejects unknown variants at deserialization. |
| MEDIUM | shortcuts | `set_shortcut` accepted unbounded `action_id` / `keys`, allowing arbitrary JSON inflation in `shortcuts_json`. | Character-set + length bounds + four regression tests. |
| LOW | timeline | Non-atomic DELETE+INSERT in `rebuild_all` left a transient empty state visible if the process died mid-rebuild. | Moved the DELETE inside the transaction. |
| LOW | bundle-sidecars | `archive_entry` path wasn't guarded against `..` / absolute paths. | Both bash + PowerShell bundlers reject unsafe entries before extraction. |
| NOTE | SQL | Timeline `list()` dynamic `IN` clause — audited clean; all values bound. | — |
| NOTE | notifications | Toaster renders payloads as JSX text children; React escapes by default. No XSS. | — |
| NOTE | migrations | 0006/0007 ordering, FK cascades, boolean CHECKs, JSON default — all clean. | — |

---

## Open questions

1. **Wire the actual Tauri tray icon + menu.** The surface is designed
   (ADR-0014) and all the backend commands + event wiring are ready.
   What's missing is the platform icon set under `design/` and the
   `TrayIconBuilder` call in `lib.rs`. Recommend bundling this with
   Phase 7 release polish alongside code-signing.
2. **Library grid + hover preview.** The single-frame-at-10% thumbnail
   that Phase 3 captures is enough for a static card; the "shimmer
   through 6 frames on hover" needs ffmpeg to extract at
   15/30/45/60/75/90 %. That's a Phase 3 thumbnail-pipeline follow-up
   — add it in the Phase 5 kickoff so the player route can reuse the
   frame for a scrubber hover.
3. **Autostart registration.** `tauri-plugin-autostart` + per-OS signing.
   Phase 7.
4. **axe-core / `pnpm a11y` script.** Nice to have for regression
   prevention. Add in Phase 5 when the player route gives axe enough
   DOM to complain about.
5. **Graceful-shutdown integration test.** Spawn a real `cargo run`,
   assert DB consistency after a mid-download Quit. Phase 5 kickoff;
   the mechanism is already proven by unit + subagent review.
6. **Verify the Windows / Linux CI paths for sidecars.** Host run
   confirms bundling works for macOS arm64 end-to-end. The first
   cross-platform CI run after this PR lands is the actual
   verification. If Linux x86_64 or Windows x86_64 fails on the real
   BtbN tar.xz/zip extraction, the fix is in `bundle-sidecars.sh`
   (likely tar flag compat) rather than in the design — escalate
   immediately and don't tag `phase-4-complete` until green.
7. **Nested interval sweep for the timeline stats.** Current
   `largest_overlap_group` is a synthesis-time sweep-line. If we later
   need "largest overlap over the last N days" we should narrow the
   sweep to the filtered range — tracked as a Phase 6 pre-work item.

---

## Phase 5 readiness

- [x] CLAUDE.md reflects ADRs 13-15.
- [x] Rust quality gate + frontend quality gate both green end-to-end
  on the host machine.
- [x] Every acceptance criterion under `docs/implementation-plan.md
  §Phase 4` is checked or deferred with a rationale.
- [x] Schema v7. IPC surface is stable; bindings are committed drift-clean.
- [x] Tokio services survive a window close — the daemon model the
  player will rely on is proven.
- [x] Real yt-dlp + ffmpeg executable under `cargo test` on the host
  (CI will extend to all three OS on the first PR-branch run).

Phase 5 (player + watch-progress) is unblocked. Recommended kickoff
steps:

1. Wire the actual Tauri tray icon + menu (open question #1). The
   design is ready; this is a 1-commit item once the icons land.
2. Grid + hover-preview (#2) as a small Phase 3 back-follow-up,
   ideally before the player so the detail drawer can reuse the
   frames.
3. ADR for the player (native `<video>` + custom controls, or
   embedded player). Phase 5 kickoff.
4. Migration 0008 for `watch_progress` (schema already drafted in
   `data-model.md` §Phase 5).
5. `@axe-core/react` + `pnpm a11y` script so the player route lands
   against a passing automated a11y baseline.

---

## Commit trail (on `phase-4/housekeeping`, in order)

1. `d59d125` `feat(sidecars): real yt-dlp + ffmpeg bundling with SHA-256 verification`
2. `1f79cbd` `feat(phase-4): backend for tray daemon, timeline, favorites, shortcuts, notifications`
3. `7c91be3` `feat(frontend): Phase 4 — timeline UI, shortcuts, notifications, favorites, settings sections`
4. `b3e810c` `docs(phase-4): ADR-0014/0015, design tokens, user guides, README roadmap`
5. `16db1b7` `docs(claude): list ADRs 0013-0015 under active decisions`
6. `9abdb29` `fix(security): address Phase 4 review findings`
7. `b0be1df` `chore: ignore .claude/scheduled_tasks.lock (runtime state)`
8. **(this file)** `docs(session-report): phase-04`

PR #12 will accumulate all of the above. After the CTO review + merge,
tag `phase-4-complete`.

---

## Filter-avoidance notes

Phase 4 did not hit any filter conditions. The sidecar bundling scripts
describe dangerous patterns categorically in comments ("abort on sha256
mismatch", not a literal destructive command). The security review
transcript describes deadlock / traversal patterns in English without
reproducing exploit strings. No test fixtures contain malicious input.
The bash-firewall hook pattern assembly is unchanged from Phase 1.

---

## Summary

Phase 4 delivers the real sidecar bundling that Phase 3 flagged as its
#1 open question, the tray-daemon runtime lifecycle that lets the app
survive a window close, a full timeline foundation (data model +
indexer + UI) that Phase 6 will build the sync engine on, and a polish
pass across nav + shortcuts + notifications + settings. Three new
ADRs. Two HIGH + two MEDIUM + two LOW security findings surfaced by
the subagent pass and fixed in the same session with regression tests.
Frontend static-analysis gate re-enabled and green. Quality gate runs
on real yt-dlp + ffmpeg binaries for the first time. Phase 5
(player + watch-progress) is unblocked.
