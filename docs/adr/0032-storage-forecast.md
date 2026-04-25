# ADR-0032 — Storage forecast heuristic

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0028](0028-quality-pipeline.md) (quality profiles whose
  storage cost we estimate) ·
  [ADR-0030](0030-pull-distribution-model.md) (sliding-window size
  parameter that bounds the forecast).

## Context

The CEO storage-aware mandate explicitly calls out: "Before adding a
streamer, the user should see what it'll cost them."  This is the
critical UX moment to set expectations — once a streamer is added and
6 hours of VOD has been pre-fetched, the user is already 1.4 GB in.

We need a heuristic that:

1. Produces a reasonable estimate from data Sightline can collect
   without much friction (Twitch metadata, the user's settings).
2. Is honest about its accuracy — the actual disk usage will differ
   by 20-40 % depending on stream content variability.
3. Maps to two end-user-meaningful numbers: "GB / week of new
   downloads" and "peak GB on disk".

## Decision

A pure-function `services::forecast::estimate_streamer_footprint(...)`
that combines four inputs into two outputs.

### Inputs

1. **Average VOD length** for the streamer over the last 30 days.
   Sourced from a Twitch Helix `/videos?user_id=X` query, filtered by
   `created_at` window.  Falls back to the global default (180 min)
   when the streamer has fewer than 3 VODs in the window — typical
   for a fresh discovery.
2. **Stream frequency** — count of VODs in that 30-day window divided
   by 30 days.  Smoothed: a streamer with 28 VODs in 30 days is
   approximated to 1 stream/day; a streamer with 4 VODs is 0.13/day.
3. **Quality factor** — looked up by `VideoQualityProfile`:

   | Profile     | GB / hour |
   |-------------|-----------|
   | 480p30 H.265 | 0.30     |
   | 480p60 H.265 | 0.45     |
   | 720p30 H.265 (default) | 0.70      |
   | 720p60 H.265 | 1.10      |
   | 1080p30 H.265 | 1.50      |
   | 1080p60 H.265 | 2.20      |
   | source (1080p60 H.264) | 4.00      |

   Every non-source row assumes the H.265 re-encode path defined in
   ADR-0028; only `source` keeps the upstream H.264 bitrate.  Quality
   factors do not differentiate between hardware and software
   encoders — the difference at the same profile is on the order of
   5-10 % and folds into the forecast's documented fuzziness.

   These constants come from re-encoding 6 representative GTA-RP VODs
   at each preset and averaging the resulting file sizes per hour.
   Documented in `docs/reference/quality-factor-measurement.md`
   (referenced from this ADR but written separately so it can be
   updated without re-issuing an ADR).

4. **Sliding window size** — the per-streamer cap on `ready+queued`.

### Outputs

```rust
pub struct ForecastResult {
    pub weekly_download_gb: f64,
    pub peak_disk_gb: f64,
    pub watermark_risk: WatermarkRisk,
}

pub enum WatermarkRisk {
    Green,   // peak < 50 % of free disk
    Amber,   // peak in 50-80 % of free disk
    Red,     // peak > 80 % of free disk → would trip cleanup
}
```

Math:

- `avg_VOD_GB = avg_VOD_hours × quality_factor`
- `weekly_download_gb = streams_per_day × 7 × avg_VOD_GB`
- `peak_disk_gb = sliding_window_size × avg_VOD_GB`
- `watermark_risk` compares `peak_disk_gb` against the user's free
  disk on the library partition.

### UI integration

The Settings → Streamers → Add dialog renders:

> Adding **{display_name}**:
>
> - Streams roughly **{N}** VODs / week, **{H}** hours each on
>   average.
> - At your current settings (**{quality_label}**, sliding window
>   **{N}**), this streamer will:
>   - Download about **{weekly_download_gb} GB / week**.
>   - Peak at about **{peak_disk_gb} GB on disk** at any time.
> - Free space available: **{free_gb} GB**.
> - {watermark_risk_indicator}

The Settings → Storage section shows a global "all streamers
combined" forecast.

### Known inaccuracies

Documented in the UI under a "Why is this estimate fuzzy?" expand:

- New streamers with no 30-day history get a guess based on global
  averages.
- The user's actual quality-factor depends on the encoder (hardware
  encode at the same profile produces 5-10 % larger files than
  software encode).  We don't differentiate.
- A streamer who alternates 8-hour weekday streams with 30-min
  weekend pop-ins has high variance the average doesn't capture.

## Alternatives considered

### A. Server-side forecast service

Rejected.  Sightline has no server.  All math has to be local.

### B. Sample-based forecast (download 5 minutes, extrapolate)

Considered.  More accurate but requires the user to actually start a
download to see the forecast — defeats the "before adding a streamer"
goal.

### C. Per-quality-profile measurement table from the user's own
       library

Interesting but requires the user to have a populated library at
multiple quality profiles, which they wouldn't.  The static table is
fine for the forecast UI's "fuzzy estimate" purpose; the user's
actual disk usage is observable post-add via the Storage section.

### D. Live-update forecast as the user changes Settings

Done in the Streamer-Add dialog (frontend recomputes when settings
change in the same session).  Not done in the global forecast (it's
already a view-time computation).

### E. Confidence interval (e.g., "70-100 GB peak") instead of a
       single number

Rejected for v2.0 — adds visual clutter for marginal accuracy gain.
The watermark-risk indicator (Green/Amber/Red) communicates the
confidence implicitly.

## Consequences

**Positive.**
- The user sees the cost before committing.  Sets expectations,
  reduces the "why is my disk full" support burden.
- The math is pure and testable; the lookup table is data, not code.

**Costs accepted.**
- Calling Helix `/videos` for a new-streamer forecast adds one HTTP
  round-trip to the Add flow.  Rate-limit-friendly (single shot).
- The quality-factor table is opinionated; some users with 4K
  monitors will notice the source-quality estimate is generous and
  some with low-bitrate streams will find ours pessimistic.

**Risks.**
- The 30-day window is too short for streamers who took a 2-week
  vacation; the average gets skewed.  Documented in the fuzziness
  expand.
- Quality factors will drift as ffmpeg / hardware-encoder defaults
  change between sidecar versions.  We re-measure when we bump the
  ffmpeg pin (ADR-0013).

## Follow-ups

- Live-update forecast that watches `app_settings` changes for the
  Settings → Storage section.  Tracked for v2.1.
- Quality factor by encoder (VideoToolbox vs Software produces
  measurably different file sizes).  Tracked for v2.1 if support
  data shows the discrepancy is bothering users.
- Per-streamer historical accuracy report ("forecast said 5 GB,
  actual 4.2 GB" after the user has lived with the streamer for a
  month).  Considered for v2.2 once we have telemetry-free ways to
  collect that data locally.

## References

- `src-tauri/src/services/forecast.rs`
- `docs/reference/quality-factor-measurement.md` (data behind the
  lookup table, separately versioned)
- `src/features/streamers/AddStreamerDialog.tsx` (UI integration)
- `src/features/settings/StorageForecast.tsx`
- ADR-0028 (the quality profiles)
- ADR-0030 (the window size we multiply against)
