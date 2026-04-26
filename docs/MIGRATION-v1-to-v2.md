# Migration: Sightline v1.0 → v2.0

This document is for **existing v1.0 users**.  New users can skip
straight to [`README.md`](../README.md) — the v2.0 defaults are
right for fresh installs.

## TL;DR

- The v1.0 → v2.0 upgrade is **automatic and non-destructive**.
- Your existing VODs, watch progress, settings, and library
  layout are preserved.
- Your install **stays in legacy "Auto-download" mode** by default
  — Sightline detects your existing downloads and pins
  `distribution_mode = 'auto'` to keep the v1.0 behaviour you're
  used to.
- New v2.0 features (pull-on-demand, 720p30 H.265 default,
  hardware-encode-first re-encoding) are **available but opt-in**
  for upgraders.  Settings → Distribution and Settings → Video
  Quality are the only two places you have to look.

## What changed

### Pull-on-demand distribution (ADR-0030)

v1.0 auto-enqueued every newly-discovered VOD.  v2.0 introduces a
**pull model**: polling produces `available` rows; the user picks
what to download.

**Existing v1.0 installs are pinned to `auto` mode by the migration**
(`0017_distribution_settings.sql`).  This is a one-line UPDATE that
runs only if your `downloads` table has any `completed`, `queued`,
or `downloading` rows.  Detection is conservative — even one
historical download flips you to `auto`.

To opt in to pull-on-demand:

1. Settings → Distribution → **Distribution Mode** → "Pull-on-demand
   (recommended)".
2. Settings → Distribution → **Sliding window size** controls how
   many `(queued + downloading + ready)` VODs Sightline keeps
   per-streamer.  Default 2 for new installs; pick 5–10 if you binge.

You can flip back to `auto` at any time without data loss.

### Quality pipeline (ADR-0028)

v1.0 default was `Source` (whatever the upstream gives, typically
1080p60 H.264 ≈ 4 GB/h).  v2.0 default for new installs is
**720p30 H.265 ≈ 0.7 GB/h**.

**Existing installs preserve your `quality_preset` value.**  Phase 8
adds a separate `video_quality_profile` column; the new column gets
the default `720p30` for all rows including upgrades.  In v2.0 the
re-encode service ships and is fully tested, but the **automatic
trigger from a completed download is a v2.0.x integration** (see
Known Limitations under "v2.0.x download-worker integration"
below).  Until that integration lands, picking a VOD downloads it
at the source quality the existing Phase 3 download path produces;
the `video_quality_profile` setting is honoured for any path that
explicitly invokes `ReencodeService::reencode_to_profile` and is
the source of truth the v2.0.x integration will read.  Existing
files on disk are never re-encoded — their bytes stay exactly as
v1.0 left them.

To opt in to the new quality pipeline:

1. Settings → Video Quality → **Quality profile** → "720p30 H.265
   (recommended)" or your preferred tier.
2. Sightline's encoder detection runs once on first start.  If
   you're on a machine without hardware encoders, Settings → Video
   Quality → **Allow software encoding (libx265 fallback)** is the
   opt-in for the libx265 path (off by default — saturates CPU
   during gaming).

### Audio is **never** re-encoded

ADR-0028 §Audio policy makes this a load-bearing invariant.  The
ffmpeg pass uses `-c:a copy` unconditionally; a regression test
asserts the args contain exactly one `-c:a copy` and no other
audio-codec directive.  GTA-RP's voice acting is too important to
lose.

### Sliding-window auto-cleanup

ADR-0030 + ADR-0024 together: when you mark a VOD as watched, the
**oldest already-archived VOD on the same streamer** is freed if
the per-streamer window is at or above the cap.  Watch progress is
preserved on the freed row, so a re-pick gets you back to the
position you stopped at.

This replaces the v1.0 "watermark-only" cleanup as the primary
disk-pressure relief.  The watermark stays as an emergency belt-
and-braces — if your disk fills up despite the window, the Phase
7 watermark cleanup still fires.

### What v2.0 ships vs. v2.0.x

The v2.0 release ships:

- The full pull-mode foundation (state machine, sliding window,
  pre-fetch hook, IPC surface, settings, migrations).
- The full quality pipeline (encoder detection, reencode service,
  CPU throttle decision logic, settings UI).

The v2.0.x point releases will add:

- **v2.0.x download-worker integration** — the existing download
  service will start observing `vods.status = 'queued'` to drive
  `queued -> downloading -> ready` transitions.  In v2.0, picking a
  VOD transitions it to `queued` but the download itself is still
  driven by the Phase 3 `downloads.state` machine; the two run in
  parallel without conflict.  Both end-states are correct;
  the integration just makes the column the single source of truth.
- **v2.0.x storage forecast UI** — the math is in
  `domain/quality.rs::quality_factor_gb_per_hour`; the UI hook-up
  ships in v2.0.1.
- **v2.0.x library UI re-conception** — ADR-0033 describes the
  full unified view (available + downloaded + filter chips); the
  v2.0 ship has the existing library UI plus the new Distribution
  Settings tab.  v2.0.1 adds the unified card design.
- **v2.1 Windows ffmpeg suspend** — the throttle decision logic
  ships in v2.0; the actual `SuspendThread` integration on Windows
  is deferred (the Unix `kill -STOP/-CONT` path works in v2.0).

### Schema migrations

| Migration | Schema version | Adds |
|-----------|----------------|------|
| 0015 | 14 → 15 | Quality settings (6 columns) |
| 0016 | 15 → 16 | `vods.status` lifecycle column + backfill |
| 0017 | 16 → 17 | Distribution settings (3 columns) + backwards-compat detection |

All three are forward-only, idempotent, and apply automatically on
first launch of v2.0.  Reverting to v1.0 is **not supported** — the
schema_meta version is monotonic and v1.0 won't recognise schema
17.  If you need to roll back, restore the SQLite file from a
backup of your `<library_root>/sightline.sqlite`.

### Settings the migration touches

- `app_settings.distribution_mode` — new column, populated to
  `'auto'` or `'pull'` by the migration's detection.
- `app_settings.video_quality_profile` — new column, defaults
  `'720p30'` for all rows (including upgrades).  Re-encodes only
  fire when you actually download something new.
- `app_settings.encoder_capability` — populated lazily on first
  app start by `EncoderDetectionService`; null until detection
  completes (~2 s after launch).
- `app_settings.cleanup_*` — unchanged from v1.0.
- All other Phase 1–7 columns — unchanged.

### What stays exactly the same

- Library file layout (Plex / Flat).  ADR-0011 unchanged.
- Watch progress + completion threshold.  ADR-0018 unchanged.
- Multi-view sync engine.  ADR-0021/22/23 unchanged.
- Auto-cleanup watermarks.  ADR-0024 unchanged.
- Tauri capabilities + asset protocol scope.  ADR-0019/27 unchanged.
- Update checker.  ADR-0026 unchanged.

## Verification after upgrade

1. Open Sightline.  The migration runs on first launch.
2. Settings → Distribution → confirm `Distribution Mode` shows
   "Auto-download (legacy)" if you have existing downloads, or
   "Pull-on-demand (recommended)" if you're starting fresh.
3. Settings → Video Quality → confirm the encoder status shows
   your hardware encoder ("VideoToolbox", "NVENC", etc.) within
   ~5 seconds of first start.
4. Library → confirm all your existing VODs are still there.
   `vods.status` is set per the backfill rules in
   `0016_vod_status_machine.sql`.
5. (Optional) Settings → Storage → run cleanup once to verify the
   Phase 7 path still works.

## Reporting issues

Per the [README](../README.md), issues go to GitHub.  Tag with
`v2.0-migration` if your problem is upgrade-specific so we can
triage faster.

## References

- [ADR-0028](adr/0028-quality-pipeline.md) — Quality pipeline
- [ADR-0029](adr/0029-background-friendly-reencode.md) — Background re-encode
- [ADR-0030](adr/0030-pull-distribution-model.md) — Pull distribution
- [ADR-0031](adr/0031-prefetch-strategy.md) — Pre-fetch
- [ADR-0032](adr/0032-storage-forecast.md) — Storage forecast
- [ADR-0033](adr/0033-library-ui-redesign.md) — Library UI redesign
- [`CHANGELOG.md`](../CHANGELOG.md) — release notes
