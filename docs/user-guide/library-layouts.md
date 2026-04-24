# Library layouts

Sightline can organize downloaded VODs in one of two layouts. Pick the
one that matches how you already browse your media.

Settings → **Downloads & Storage** → **Library layout** lets you switch
any time; existing files are moved in the background so you never
re-download.

## Plex / Jellyfin / Infuse (default)

```
<library-root>/
  Sampler/
    Season 2026-04/
      Sampler - 2026-04-02 - GTA RP Heist [twitch-v42].mp4
      Sampler - 2026-04-02 - GTA RP Heist [twitch-v42].nfo
      Sampler - 2026-04-02 - GTA RP Heist [twitch-v42]-thumb.jpg
    Season 2026-05/
      ...
```

- One folder per streamer, named after their Twitch **display name**
  (case-preserved, non-ASCII characters sanitized for filesystem
  safety).
- One `Season YYYY-MM` sub-folder per calendar month of streaming,
  matching Plex / Infuse conventions for "show seasons".
- Each VOD is a trio: the `.mp4` file, a Kodi-compatible `.nfo` with
  title + plot + chapter tags + a `<uniqueid type="twitch">`, and a
  `-thumb.jpg` extracted from 10% into the VOD.

### What goes in the NFO

The NFO is a conservative `<movie>` document. Plex, Jellyfin, Infuse,
Kodi all parse this format:

- `<title>` / `<originaltitle>` — Twitch stream title.
- `<plot>` — Twitch VOD description.
- `<runtime>` — duration in minutes, rounded up.
- `<premiered>` / `<year>` — date the stream started (UTC).
- `<dateadded>` — first time Sightline's poller saw the VOD.
- `<director>` — the streamer's display name (stretched semantics —
  gets it onto the Infuse details panel as a credit).
- `<uniqueid type="twitch" default="true">` — the Twitch VOD id, so
  your scanner doesn't try to match this against TMDB or IMDB.
- `<tag>` for each distinct game chapter, for filtering in Jellyfin.
- `<chapters>` block — per-chapter start times so Infuse's "skip to
  next chapter" works.

### Picking this layout

Choose **Plex / Jellyfin** if you have a media server scanning the
library root (or you point Infuse at it directly), or if you like
seeing descriptive filenames.

## Flat

```
<library-root>/
  sampler/
    2026-04-02_v42_gta-rp-heist.mp4
    2026-04-02_v43_late-night-stream.mp4
    .thumbs/
      2026-04-02_v42_gta-rp-heist.jpg
      2026-04-02_v43_late-night-stream.jpg
```

- One folder per streamer, named after their Twitch **login** (always
  lowercase, always URL-safe).
- One file per VOD: `YYYY-MM-DD_<id>_<slug>.mp4` where `<slug>` is a
  URL-ish slug of the stream title.
- No `.nfo`. Thumbnails live in a hidden `.thumbs/` subdirectory so
  Finder / Explorer / file-based scanners don't trip on them.

### Picking this layout

Choose **Flat** if you browse the library with Finder / Explorer, if
your media player doesn't read NFOs, or if you don't want sidecar
files mixed in with the video files.

## Switching layouts

Settings → **Library layout** → click the other option. Sightline
shows a confirmation dialog, then:

1. Any **new** downloads write into the new layout immediately.
2. A background migrator walks every `completed` download and moves
   each file to the new layout's path. Moves on the same filesystem
   are atomic; cross-filesystem moves copy + SHA-256 verify +
   delete the source.
3. A toast shows progress (`moved / total`). Errors during migration
   are counted but do not fail the whole run — you can flip the
   layout toggle back and forward to retry individual files.

Only one migration can run at a time. If you need to cancel a
migration in progress, the safe thing is to wait for it to finish and
then flip back — partial migrations always leave the library in a
consistent state.

## What if I change my library root?

Changing the root (Settings → Library root) is a separate concern —
you need to move the existing files yourself, then point Sightline at
the new location. Sightline does **not** migrate across roots
automatically, because a wrong root entry could otherwise trigger
terabytes of unintentional copy traffic.
