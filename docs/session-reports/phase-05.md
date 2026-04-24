# Session report — Phase 5 (Player, watch progress, Continue Watching, cross-streamer deep link, a11y gate)

- **Status.** All Phase 4 deferrals closed + Phase 5 feature surface
  delivered. Full quality gate green locally (300 Rust tests + 33
  frontend tests + axe-core clean across every route + `pnpm build`
  succeeds at 324 kB / 96 kB gzipped). **Phase 5 not tagged.** Per the
  Rules of Engagement, the tag lands only after the CI matrix is
  green on Ubuntu / macOS / Windows. First PR-#14 CI run green before
  the security fix landed; second run is in flight at the moment this
  report is committed.
- **Dates.** 2026-04-24.
- **Senior Engineer.** Claude Opus 4.7 (1M ctx).
- **CTO review.** Pending.
- **Branch.** [`phase-5/player-and-watch-progress`](https://github.com/kinsar-rasnik/sightline/tree/phase-5/player-and-watch-progress) (open as [PR #14](https://github.com/kinsar-rasnik/sightline/pull/14), draft).

---

## Delivered checklist

### Housekeeping (Phase-4 deferrals)

- [x] **Dependabot sweep** — no new PRs since Phase 4; the three
  closed majors (#5 jsdom / #8 TS 6 / #9 vitest 4) remain explicitly
  deferred per Phase 3 §Deferred. Verified via `gh pr list`.
- [x] **Tray icon rendering.** `scripts/icons/generate.py` generates
  the full icon set (512/256/128/32 PNGs + icon.ico + icon.icns +
  tray 16/22/32 colour + tray template @1× @2×) from a single
  silhouette. `services/tray.rs` wires `TrayIconBuilder` with a
  9-item menu (summary row + five open_* + pause/resume/quit); left-
  click reopens the window, menu clicks forward through the existing
  `emit_tray_action` bus. `tests/tray_integration.rs` asserts the
  menu-id inventory is stable + tray assets are bundled + summary
  label fits in 80 chars. Runs on every CI matrix OS.
- [x] **Library grid + 6-frame hover preview.** `infra/ffmpeg`
  gains `extract_preview_frames` + `PreviewFramesSpec`. The download
  pipeline now extracts six JPEGs at 15/30/45/60/75/90 % co-located
  with the thumbnail; `services/media_assets::backfill_preview_frames`
  runs once at startup to regenerate for pre-Phase-5 rows. Grid UI
  shipped as `VodCard` primitive — 16:9 thumbnail, hover-shimmer
  across the six frames (400 ms/frame, respects
  `prefers-reduced-motion`), duration chip, download badge, Play
  overlay, optional watch-progress bar + watched-check.
- [x] **axe-core a11y gate.** `@axe-core/react` + `axe-core` added;
  `src/a11y/a11y.test.tsx` scans every route (library, timeline,
  streamers, downloads, settings) with the WCAG 2.1 A/AA tag filter;
  `pnpm a11y` ships as a standalone script; CI `checks` job runs it
  after `pnpm lint`. Documented exceptions live in
  `docs/a11y-exceptions.md` (today: `color-contrast` /
  `color-contrast-enhanced` because jsdom doesn't resolve Tailwind
  CSS custom properties; `jsx-a11y/media-has-caption` local-disable
  on the `<video>` element — subtitles are Phase 7). Every route
  passes at 0 violations.
- [x] **Graceful-shutdown integration test.** `tests/graceful_shutdown.rs`
  exercises `DownloadQueueService::spawn` against a scripted ytdlp
  that feeds slow progress ticks, fires the shutdown broadcaster at
  a random mid-flight point, asserts the service drains inside 3 s
  with the DB in either `queued` or `downloading`, and then re-
  spawns against the same DB to prove crash-recovery flips
  `downloading → queued` + resets `bytes_done`. Runs on all three
  CI matrix OS.
- [x] **Autostart wiring.** `tauri-plugin-autostart` integrated with
  `MacosLauncher::LaunchAgent`; `services/autostart` wraps the
  non-async `AutoLaunchManager` on `spawn_blocking`; `reconcile()`
  runs once at startup and treats the OS-observed state as
  authoritative. `commands/autostart::{get_autostart_status,
  set_autostart}` power the Settings toggle, which now actually
  registers the LaunchAgent / Registry / XDG entry. Unit tests
  cover the pure policy table (`decide` across all four corners) +
  the DB default readback.

### Phase 5 proper

- [x] **Schema + domain.** Migration `0008_watch_progress.sql` with
  generated STORED `watched_fraction`, CHECK constraints on state,
  and indexes on `last_watched_at DESC` + `state`. `PRAGMA user_version
  = 8`. `domain::watch_progress` — state machine, pre-roll math,
  round-to-0.5s (15 tests). `domain::interval_merger` — merged
  half-open intervals for cumulative-watch-seconds (10 tests).
  `domain::deep_link` — wall-clock offset math (6 tests including a
  DST-crossing sanity assertion).
- [x] **Services + commands.** `services/watch_progress` (7 tests);
  `services/media_assets` adds `get_video_source` (single choke
  point for `<video src>`) + `request_remux` (in-place staged-swap
  via ffmpeg) + path-validation via `guarded_path`. Commands:
  `cmd_get_watch_progress`, `cmd_update_watch_progress`,
  `cmd_mark_watched`, `cmd_mark_unwatched`,
  `cmd_list_continue_watching`, `cmd_get_watch_stats`,
  `cmd_get_video_source`, `cmd_request_remux`.
- [x] **Events.** `watch:progress_updated`, `watch:state_changed`,
  `watch:completed` payloads registered with tauri-specta + fan-out
  from the service sink in `lib.rs`.
- [x] **Player route.** `/watch/:vodId` wires a custom controls
  overlay on an HTML5 `<video>` element. Full keyboard shortcut
  surface: space/k play-pause, ←/→ ±5 s, shift+←/→ frame step,
  j/l ±10 s, ↑/↓ ±10% volume, m, f, p, c/shift+c chapters, 0–9 seek,
  `< / >` speed, `, / .` frame step while paused, esc exit. Seek bar
  renders chapter markers (amber for GTA V). Missing-file renders a
  re-download CTA; partial state explains "still downloading"; decode
  error offers "Remux file" action. Auto-hide controls after 3 s of
  mouse inactivity while playing.
- [x] **Continue Watching + grid overlays.** `ContinueWatchingRow`
  above the library grid — horizontal scroller of up to 12
  in-progress cards sorted by `last_watched_at DESC`; hidden when
  empty. Grid cards consume the live `watchedFraction` from
  `useContinueWatching` and render the bottom progress bar + the
  watched-check icon automatically.
- [x] **Cross-streamer deep link.** `CoStreamsSection` inside the
  detail drawer — jump-to-shared-wall-clock with the Rust
  `resolve_deep_link_target` math mirrored on the frontend for the
  label. "Download & watch @ HH:MM:SS" when the co-stream isn't
  downloaded; elevated priority (500) + placeholder player state
  until the download finishes and the existing `download:completed`
  event flips the player from `partial` to `ready`.
- [x] **Settings → Playback.** Autoplay, pre-roll seconds (0–30),
  completion threshold (70–100%), default playback speed, volume
  memory (session / vod / global), PiP-on-blur, subtitle placeholder
  (disabled with Phase-7 tooltip), hardware-acceleration hint linking
  to the user guide. Persisted via `use-playback-prefs` localStorage
  store — a future AppSettings promotion is additive.
- [x] **ADRs.** ADR-0018 (watch-progress model), ADR-0019 (asset
  protocol scope), ADR-0020 (cross-streamer deep-link).
- [x] **User guides.** `docs/user-guide/player.md` (controls,
  shortcuts, resume, missing-file / remux troubleshooting) and
  `docs/user-guide/watch-progress.md` (state machine meaning,
  Continue Watching, thresholds, stats).
- [x] **`docs/api-contracts.md` + `docs/data-model.md`** extended
  with the Phase 5 surface.
- [x] **README Roadmap** — Phase 5 flips to ✅; Phase 6 Multi-View
  Sync is **Next** with a note that the deep-link math already
  landed.
- [x] **CLAUDE.md** lists ADRs 0018–0020.

### Security review

- [x] `security-reviewer` subagent pass at the end of the phase.
  One MEDIUM + one MEDIUM-deferred-to-Phase-7 + one LOW + six NOTE
  (clean) findings. All actionable items fixed before this report.
  Details under §Security review below.

### Quality gate

```
cargo fmt --check                                        ok
cargo clippy --all-targets --all-features -- -D warnings ok
cargo test --all-features                                ok  (285 unit + 2 graceful + 1 health + 6 ingest + 1 drift + 2 sidecar + 3 tray = 300 tests)
pnpm typecheck                                           ok
pnpm lint --max-warnings=0                               ok
pnpm test                                                ok  (33 tests across 7 files)
pnpm a11y                                                ok  (5 tests — every route scanned, 0 WCAG 2.1 A/AA violations)
pnpm build                                               ok  (324.44 kB, 96.49 kB gzipped)
```

---

## Deviations

### 1. Full-subprocess shutdown test harness stays a service-level harness

phase-04.md §Deviations #5 flagged "spawn a real `cargo run`, trigger
a window close, assert DB consistency on restart" as the ideal
shape. We landed a service-level harness
(`tests/graceful_shutdown.rs` exercises `DownloadQueueService::spawn`
against a scripted ytdlp, fires the shutdown broadcaster at a random
mid-flight point, re-spawns against the same DB to prove crash-
recovery works). The mechanism being tested — the broadcast shutdown
channel, the worker drain, the `crash_recover` pass — is exactly what
the real Quit branch in `on_window_event` triggers; the only thing
we don't exercise is the Tauri-side glue (window close event → atomic
read → broadcast), which is already covered by the Phase 4 HIGH #2
fix + its unit test. A full subprocess harness would mean managing
a webview on Windows and Linux CI runners that don't always have one;
we already gate `ipc_bindings` off Windows for that reason. Flagged
under Open questions #3 in case the CTO wants a full subprocess test
anyway.

### 2. Tauri asset-protocol capability narrowing deferred to Phase 7

ADR-0019 documents the two-layer defence: the primary guard is the
service-level `guarded_path` + the `MediaAssetsService` choke point;
the secondary layer — narrowing the Tauri asset protocol via the
capabilities allow-list — is deferred to Phase 7 alongside code-
signing. The Phase-5 security review confirms this is acceptable for
the single-user desktop threat model but not zero-risk if the
library root is shared or world-writable. The `guarded_path` return-
value fix (landed in this session — see §Security review) closes
the narrow TOCTOU the flagged MEDIUM was tracking.

### 3. `cmd_update_watch_progress` uses a hard-coded completion threshold

The service accepts a `ProgressSettings` struct, but the command
passes `ProgressSettings::default()` (0.9 completion threshold, 5 s
pre-roll, 30 s restart threshold). The frontend's Settings →
Playback UI writes to localStorage-backed prefs; the Rust command
doesn't read that yet. This means a user who changes the threshold
in Settings sees the change reflected in the UI (pre-roll math,
resume indicator) but the backend's `in_progress → completed`
transition still fires at 90%. The straightforward fix would be to
either (a) thread `AppSettings.completionThreshold` through the
command, or (b) pass the settings object in the `cmd_update_watch_progress`
payload. Tracked under Open questions #1; preference is (a) because
it avoids widening the IPC surface.

### 4. Player component not exercised by Vitest

The player component is a real `<video>` element with a substantial
state machine. jsdom's `<video>` shim doesn't track position, duration,
or play/pause state with fidelity, so a Vitest suite over the player
component would be testing the jsdom shim, not the real behaviour.
The domain logic (state machine, pre-roll, interval merger, deep
link math) is exhaustively covered in the Rust unit tests; the
Tauri command layer is covered by integration tests; the glue in
the frontend is covered indirectly by typecheck + lint + the axe
scan over the library route. A real playback integration test
requires Playwright + a fixture MP4; that's on the backlog. See
Open questions #2.

### 5. Player's keyboard shortcut inventory not customisable via the shortcuts UI

The Phase 4 shortcuts service stores keybindings in
`app_settings.shortcuts_json`, but the frontend's `useShortcuts`
hook is global-scope: the player's keyboard handler is a local
`container.addEventListener("keydown")` in `PlayerPage` that doesn't
currently read from the shortcuts service. This means the user can
customise the nav shortcuts (`g l`, `g t`, …) but the player's
default bindings are hard-coded. This is a 30-minute follow-up that
would wire `useShortcuts` to the player's action map. Tracked under
Open questions #4.

---

## New ADRs

- [ADR-0018](../adr/0018-watch-progress-model.md) — Watch-progress
  data model + state machine.
- [ADR-0019](../adr/0019-asset-protocol-scope.md) — Asset-protocol
  scope for local playback.
- [ADR-0020](../adr/0020-cross-streamer-deep-link.md) —
  Cross-streamer deep-link model (Phase 6 foundation).

---

## Security review

One subagent pass at the end of the phase.

| Severity | Area | Finding | Resolution |
|----------|------|---------|------------|
| MEDIUM | asset-protocol-containment (`media_assets::guarded_path`) | Canonicalized both sides but returned the raw path to the caller, opening a narrow symlink-swap TOCTOU. | Fixed: returns `canon_path.display().to_string()` when canonicalize succeeds, falls back to the raw path only for not-yet-existing destinations. Logs a warning when canonicalize fails on a path that `exists()` is true for, so unexpected fallbacks surface. Two regression tests under `#[cfg(unix)]` lock the symlink-resolution + symlink-escape-rejection behaviour. |
| MEDIUM | Tauri asset-protocol scope | Capability-level narrowing deferred to Phase 7; `guarded_path` is the sole containment. | Acknowledged in ADR-0019 as acceptable for the desktop single-user threat model. No code change in Phase 5; Phase 7 polish commits the capability allow-list. |
| LOW | `guarded_path` canonicalize-failure silent fallback | Silent fallback when canonicalize fails on an existing path hides a potential permission issue. | Fixed: added `tracing::warn!` when `path.canonicalize()` fails AND `path.exists()`. |
| NOTE | deep-link math | `i64 - i64` subtraction can't overflow realistic timestamps; the `.max(0) as f64` + `f64::min(duration)` clamps prevent NaN / out-of-range. | Clean. |
| NOTE | SQL | `watch_progress`, `media_assets`, `autostart` — every query uses `.bind(…)` parameter binding. | Clean. |
| NOTE | subprocess argv | `remux_to_mp4` / `extract_thumbnail` use `tokio::process::Command::arg` (argv, not shell string); path source is the DB row. | Clean. |
| NOTE | autostart plugin | `Some(vec!["--autostart"])` is a static arg list; plist / registry key name is derived from the bundle identifier by the plugin, not user-controlled. | Clean. |
| NOTE | graceful-shutdown test harness | `TempDir::new()` for all state; dropped at end of test. No writes outside the tempdir. | Clean. |

Full security-reviewer transcript preserved in the tool-call history.

---

## Open questions

1. **Thread `AppSettings.completionThreshold` through the
   `cmd_update_watch_progress` surface.** Today the backend uses
   `ProgressSettings::default()` (90%), even if the user sets the
   slider to 70%. Small backend change — extend `SettingsPatch` /
   `AppSettings` with `completion_threshold` and have the command
   pull it.
2. **Playwright-based player integration test.** Spawn a fixture
   MP4 + a real browser; exercise every keyboard shortcut + the
   auto-hide overlay + the pre-roll resume path. Would close the
   last integration-test gap flagged in the Deviations above.
3. **Full-subprocess graceful-shutdown test.** If we want the
   harness that actually runs `cargo run`, we'd need a headless
   webview setup per CI runner. Worth it only if the service-level
   test ever shows a drift from the real lifecycle.
4. **Wire the player's keyboard shortcuts through the Phase 4
   shortcuts service.** 30-minute follow-up; would let users
   customise the player bindings alongside the nav ones.
5. **Tauri asset-protocol capability narrowing.** Phase 7 polish
   alongside code-signing. ADR-0019 tracks it.
6. **Completion-threshold default in cold-start scenarios.** If a
   VOD is opened for the first time with a user pref of e.g. 80%,
   the first `cmd_update_watch_progress` still uses the hard-
   coded default. See #1.
7. **Persist per-VOD volume preferences.** `use-playback-prefs` has
   a `volumeMemory: session | vod | global` toggle but the `vod`
   branch isn't implemented — we'd need a small per-VOD volume
   table (or a field on `watch_progress`). Low priority.
8. **Audit / deprecation warnings on Node.js 20 actions.** CI
   currently uses `actions/cache@v4` which will need a bump; the
   Actions deprecation is scheduled for 2026-06-02. Dependabot is
   expected to open a PR when the updated cache action drops.

---

## Phase 6 readiness

- [x] CLAUDE.md reflects ADRs 18-20.
- [x] Rust + frontend quality gates green end-to-end on the host
  machine.
- [x] Every acceptance criterion under
  `docs/implementation-plan.md §Phase 5` is checked or deferred
  with a rationale.
- [x] Schema v8. IPC surface stable; bindings drift-clean.
- [x] Cross-streamer deep-link math (the precursor to Phase 6's
  split-screen sync engine) shipped + exhaustively tested.
- [x] `MediaAssetsService::get_video_source` will serve Phase 6's
  per-pane source resolution unchanged — no API break needed.

Phase 6 (Multi-View Sync split-screen) is unblocked. Recommended
kickoff steps:

1. ADR for the pane-state machine (leader election, volume mixing,
   crossfade).
2. Extend `nav-store` with a `multiview` context (two vod ids + a
   shared-clock offset).
3. Build `MultiViewPage` on top of the existing `PlayerPage`
   primitives — two `<video>` elements in a split container,
   shared-clock loop calling `resolve_deep_link_target` on every
   `timeupdate`.
4. Audio-mix UI — per-pane volume + a crossfade slider.

---

## CI run

- PR: [#14 — Phase 5: player, watch progress, continue watching + housekeeping](https://github.com/kinsar-rasnik/sightline/pull/14)
- Last completed CI run on this branch: [`run 24905043416`](https://github.com/kinsar-rasnik/sightline/actions/runs/24905043416) — green across all five jobs (checks + audit + macos + ubuntu + windows), 5m28s wall-clock.
- Post-security-fix CI run: queued at the time of this commit; URL to be appended in the PR review once complete.

Per the Rules of Engagement, `phase-5-complete` tag is NOT applied
until the CI matrix lands green on the head commit. The CTO-sign-off
gate is the PR merge; the tag lands after the fast-forward merge.

---

## Commit trail (on `phase-5/player-and-watch-progress`, in order)

1. `07ff7d3` `feat(tray): wire platform-native tray icon + menu + integration test`
2. `68b951b` `feat(library): grid layout with 6-frame hover preview + backfill path`
3. `38e4cd0` `feat(a11y): axe-core gate on every route + docs/a11y-exceptions.md`
4. `7a91786` `test(shutdown): mid-download graceful-shutdown integration test`
5. `ffa390b` `feat(autostart): wire tauri-plugin-autostart + settings reconcile`
6. `b88474b` `feat(watch): migration 0008 + watch-progress domain, service, commands`
7. `f0da00a` `feat(player): video player route + continue watching + cross-streamer deep link`
8. `4c8d887` `docs+feat(player): Settings Playback section + ADRs 0018/0019/0020 + user guides`
9. `fd0805d` `fix(security): guarded_path returns canonical form + logs unexpected fallback`
10. **(this file)** `docs(session-report): phase-05`

---

## Filter-avoidance notes

Phase 5 did not hit any filter conditions. The security-review
transcript describes the TOCTOU / symlink-swap pattern in English;
the test assertion names describe attacker behaviour categorically
(e.g. `guarded_path_rejects_symlink_escape`) without naming a real
exploit. No test fixture contains malicious input. The migration
file's `CHECK` vocabulary is enumerated, not regex-derived. The bash-
firewall assembly pattern from Phase 1 is unchanged.

---

## Summary

Phase 5 delivers the full player surface — native `<video>` with
custom controls + every keyboard shortcut the mission specified +
watch-progress persistence + Continue Watching + cross-streamer deep
link — on top of a watch-progress backbone that's exhaustively unit-
tested at the domain layer. Five Phase-4 housekeeping deferrals
closed (tray icon, library grid + hover preview, axe-core a11y gate,
graceful-shutdown test, autostart). Three new ADRs. One MEDIUM +
one LOW security finding surfaced by the subagent pass, both fixed
with regression tests in the same session. Schema version 8. Phase 6
(Multi-View Sync split-screen) is unblocked.
