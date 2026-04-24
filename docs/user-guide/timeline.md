# The Timeline view

The **Timeline** route (click "Timeline" in the top nav, or press `g t`)
shows every stream Sightline has ingested for your followed streamers on a
single horizontal time axis. Each streamer gets their own lane; each stream
is a coloured bar.

## Reading the timeline

- **Bar colour.**
  - Blue — a stream where the streamer was live by themselves.
  - Purple — a stream that overlaps at least one other streamer's stream.
    You can jump between overlapping streams from the library detail
    drawer.
- **Width.** Proportional to the stream duration. A 30-minute stream is a
  narrow tick; an all-day marathon is a wide rectangle.
- **Hover.** The native browser tooltip shows start time + duration.

## Filtering

Use the filter bar above the lanes to narrow the view:

- **Streamer dropdown** — show only one streamer's lane.
- **"Only streams with overlaps"** — hide solo streams so it's easier to
  spot the co-stream density.

Your filter selection is kept in the URL query string, so bookmarking or
sharing a filtered view inside the app works naturally.

## Behind the scenes

The timeline is rendered from a dedicated `stream_intervals` table that
Sightline maintains automatically. Every time a new VOD is ingested from
the poller, an interval is upserted. On first boot after upgrading to the
Phase 4 release, Sightline backfills the table from your existing library
— you'll see a progress banner if there's any work to do.

If the timeline ever looks wrong (a gap you don't expect, a duplicate
row), open Settings → Advanced → **Rebuild timeline index**. This drops
and re-derives the table from your VOD list; it only takes a few seconds
for a typical library and doesn't touch any other data.

## Coming in Phase 6

The timeline is the foundation for **Multi-View Sync**: shift-click two
bars that visually overlap and Sightline opens them side-by-side, with
playback locked to a shared wall-clock. That's a Phase 6 feature — the
timeline UI today shows the overlaps but the sync engine itself is not
wired yet.
