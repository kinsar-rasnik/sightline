# ADR-0035 — Download-engine settings wiring

- **Status.** Accepted
- **Date.** 2026-04-26 (v2.0.3 hotfix)
- **Related.**
  [ADR-0028](0028-quality-pipeline.md) (defines the
  `video_quality_profile` setting that this ADR finally wires to the
  worker) ·
  [ADR-0010](0010-bandwidth-throttle.md) (the per-worker bandwidth
  cap that depends on the live worker count this ADR makes correct) ·
  [ADR-0036](0036-per-download-process-lock.md) (the in-flight lock
  that AC3 introduces alongside this work).

## Context

The CEO installed v2.0.2 on a Mac mini with a Proton-Drive library on
an external HDD and reported, within hours, that the app had pinned
the disk and was stuck in an endless RETRYING loop on every queued
VOD.  Terminal log excerpt:

```
[info] v2756822427: Downloading 1 format(s): 1080p60
ERROR: Unable to rename file:
   '...2756822427.mp4.part-Frag160.part'
   -> '...2756822427.mp4.part-Frag160'
ERROR: Unable to download video: [Errno 2] No such file or directory:
   '...2756822427.mp4.part-Frag160'
```

Two diagnoses:

1. **Quality wiring.** Settings → Video Quality showed `720p30 H.265
   (recommended)` — the Phase 8 default from ADR-0028.  Yet yt-dlp
   spawned with no `-f` filter at all and resolved to
   `bestvideo+bestaudio = 1080p60 H.264` from Twitch.  At ~13 GB per
   six-hour VOD that is the entire reason a "20-GB-disk hobbyist"
   cohort exists in the first place — and it was being silently
   ignored.
2. **Concurrency wiring.** Settings → Downloads & Storage showed the
   slider at 2.  Yet the log reproduced six parallel
   `Downloading m3u8 information` lines for the same VOD-IDs in
   different orders, i.e. the worker was spawning yt-dlp processes
   without bound and they were colliding on the same staging
   fragment files.

Both problems trace to the same architectural gap: the Phase 8
Settings UI was wired into the database, but no end-to-end-tested
chain proves that a value typed into the UI changes the bytes that
flow off the network.  Phase 8's own Final-Report didn't catch this
because the test coverage stopped at the IPC boundary.

## Decision

Three pieces of plumbing, end-to-end-tested.

### Quality profile → yt-dlp `--format`

`services::downloads::pipeline_inner` now reads
`settings.video_quality_profile` (the Phase 8 field) and passes its
`format_selector()` into `DownloadSpec.format_selector`.  The legacy
`settings.quality_preset` (Phase 3 enum) is no longer read on the
spawn path.

The `VideoQualityProfile::format_selector()` method is co-located
with the enum in `domain/quality.rs`.  Selectors are static strings
with two structural invariants:

- **Audio passthrough** (ADR-0028 §"Audio policy").  Every selector
  pairs `bestvideo` / `best` with `bestaudio` so the muxed audio is
  byte-exact source.  Trip-wire test:
  `every_selector_contains_bestaudio`.
- **No `vcodec` filter.** Twitch overwhelmingly delivers H.264; the
  Phase 8 H.265 target is the re-encoder's job, not yt-dlp's source
  pick.  Trip-wire test: `no_selector_filters_on_vcodec`.

For 30-fps profiles the chain probes the **exact target height
first** (with and without the fps filter) before walking down to
lower heights.  This is what makes
"720p as the GTA-RP text-legibility floor" (ADR-0028) actually
hold:

```
bestvideo[height=720][fps<=30]+bestaudio
/bestvideo[height=720]+bestaudio
/bestvideo[height<720][fps<=30]+bestaudio
/bestvideo[height<720]+bestaudio
/best[height<=720]/best
```

Naive `bestvideo[height<=720][fps<=30]+bestaudio` would silently pick
`480p30` over `720p60` on a Twitch ladder of `{1080p60, 720p60,
480p30}` because yt-dlp's `/` is left-to-right "first match wins".
Two ordering trip-wires
(`thirty_fps_profiles_prefer_fps_match_over_any_fps_at_target_height`,
`thirty_fps_profiles_prefer_target_height_over_lower_height`) pin
the correct ordering.

60-fps profiles keep the simpler single-arm chain because
`[fps<=60]` accepts every reasonable Twitch variant.  The final
`/best` in every chain is the safety net for sources missing every
height tier.

### Concurrency cap → live worker count

Up to v2.0.2 the worker read `app_settings.max_concurrent_downloads`
**once** at queue startup, sized a `Semaphore`, and never resynced.
A user dragging the slider after launch had no way to make their
change take effect short of a full app restart.

The `Semaphore` is gone.
`services::downloads::DownloadQueueService::drain_once` now reads the
cap fresh from settings on every tick (every 5 s on the timer; on
demand via WAKEUP).  Spawning is gated by
`in_flight.len() < cap` against an `Arc<Mutex<HashSet<String>>>` of
live `vod_id`s ([ADR-0036](0036-per-download-process-lock.md)
documents the lock).

Trip-wire test: `drain_once_respects_concurrency_cap`.

### Cap range → 1..=3 + frontend

The service-layer clamp was `1..=5`; the frontend slider went up to
5.  Both are now `1..=3`.  The service-layer test
`max_concurrent_downloads_clamped_to_1_3` pins the new ceiling and
`max_concurrent_downloads_inrange_round_trips` pins that 1, 2, 3
are all preserved verbatim — i.e. an existing user with 2 (CEO
context) doesn't get silently flattened.

Migration `0018_concurrency_default.sql` flattens any pre-existing
NULL or `>3` value to 1.  The 1..=3 range is preserved as-is.  The
column DEFAULT (set in migration 0004 to 2) is **not** changed; a
SQLite table-rewrite to do so is out of scope for v2.0.3.

### What we didn't do (deferred to v2.1)

- **Drop `app_settings.quality_preset` column.**  Cleanup-Task A in
  the v2.0.3 mission removed the legacy Quality preset pills from
  the Settings UI but kept the column.  Dropping it requires
  updating `enqueue()`, the `DownloadRow` DTO, the IPC bindings,
  the `DownloadsPage` display fallback, and several test mocks —
  exceeds the per-task 30-minute budget set by R-SC.  Tracked as
  v2.1 backlog item.
- **Drop `downloads.quality_preset` per-row column.**  Same reason.
- **Change `app_settings.max_concurrent_downloads` column DEFAULT
  to 1.**  Requires a SQLite table rewrite (30+ columns).  Fresh
  installs land on 2 from migration 0004 — still inside the new
  safe range, and the worker enforces the cap regardless.  Tracked
  as v2.1 backlog item.
- **Record actual downloaded tier in `downloads.quality_resolved`.**
  v2.0.3 records the requested `video_quality_profile` string;
  parsing yt-dlp's `format_id` to capture what was actually
  downloaded is tracked as v2.1.

## Alternatives considered

### A. Move quality fallback into a Rust-side resolver (drop yt-dlp's `/` chain)

The Phase 3 `resolve_preset` function did exactly this: probe
`yt-dlp --print "%(height)s %(fps)s"` first, decide which preset the
source can satisfy, then hand a single-arm selector to the download
pass.  Rejected for v2.0.3 because (a) the extra round-trip to
yt-dlp adds latency on every download, (b) the Phase 8 selector
chain expresses the same fallback cleanly inside yt-dlp, and (c)
the resolver path was the source of the v2.0.2 bug — it read
`settings.quality_preset` (legacy) instead of `video_quality_profile`
(Phase 8) and was the immediate trigger for this hotfix.

### B. Use yt-dlp `--format-sort` instead of a `/` chain

yt-dlp's `--format-sort "res:720,fps:30"` is a more expressive way
to express "prefer 720p over 480p, then prefer 30 fps".  Rejected
for v2.0.3 because adding a new CLI flag means extending
`DownloadSpec` and the `YtDlpCli` flag set; the `/` chain achieves
the same picking behaviour for the discrete Twitch height ladder
(1080/720/480/360/160) without that surface change.  Tracked as
v2.1 backlog if Twitch starts offering finer-grained height
variants.

### C. Wake-up the worker on every settings UPDATE

Today the cap-change takes effect on the next tick (≤ 5 s).  We
could fire a `WakeUp` command from the settings IPC so the change
propagates immediately.  Rejected as overkill — the 5 s drift is
unobservable to a human dragging a slider, and the wake-up plumbing
would couple two services that don't otherwise know about each
other.

## Consequences

**Positive.**
- A user setting the Phase 8 quality profile actually gets that
  quality.  CEO's `720p30 H.265` install no longer downloads at
  `1080p60 H.264`.
- The slider value is now load-bearing.  Slider changes propagate
  on the next tick without an app restart.
- The 1..=3 hard cap removes the `max=10` foot-gun that produced
  six parallel yt-dlp processes for a single VOD.

**Costs accepted.**
- Three columns survive as zombies (`app_settings.quality_preset`,
  `downloads.quality_preset`, the unchanged DEFAULT on
  `max_concurrent_downloads`).  Documented in ADR + the v2.1
  backlog.
- `quality_resolved` now records the requested profile, not the
  post-hoc tier yt-dlp actually downloaded.  The UI is consistent
  with the Settings page; the "what did we land on" signal is lost
  until v2.1 parses `format_id`.

**Risks.**
- The `pick_next_queued_excluding` SELECT carries an inline `NOT IN
  (?, ?, ...)` placeholder list whose length tracks the in-flight
  HashSet.  Defence-in-depth: every value comes from a `String` we
  ourselves persisted in `downloads.vod_id`, and `.bind()` separates
  data from SQL even if that ever changes.

## Follow-ups

- **v2.1 backlog item: drop `app_settings.quality_preset`** — and
  the matching IPC field, `DownloadRow.quality_preset`, the
  DownloadsPage display fallback, and the test mocks.
- **v2.1 backlog item: rewrite `app_settings` to bump the
  `max_concurrent_downloads` DEFAULT to 1** — table-rewrite
  migration; bundle with any other column-default change that
  accumulates.
- **v2.1 backlog item: record the actual downloaded tier in
  `quality_resolved`** — parse yt-dlp's `format_id` from the
  download stdout/result.
- **Phase-8 retro: how did this ship?**  The Settings UI was tested
  for IPC round-trip but not for end-to-end behavioural impact.
  Future settings-touching phases need at least one trip-wire that
  drives the worker through process_one with a non-default setting
  and asserts on the captured spawn args.

## References

- `src-tauri/src/services/downloads.rs::pipeline_inner`
- `src-tauri/src/domain/quality.rs::VideoQualityProfile::format_selector`
- `src-tauri/src/services/downloads.rs::drain_once`
- `src-tauri/src/services/downloads.rs::current_concurrency`
- `src-tauri/migrations/0018_concurrency_default.sql`
- `src/features/settings/SettingsPage.tsx::DownloadsAndStorageSection`
- ADR-0028 (Phase 8 quality pipeline — defines the field this ADR
  wires)
- ADR-0036 (per-download process lock — the AC3 companion)
