# ADR-0011: Pluggable library layout — Plex/Jellyfin and Flat

- **Status.** Accepted (2026-04-24).
- **Phase.** 3.
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

Sightline downloads VODs into a user-selected library root. Two user
personas split cleanly along layout lines:

1. **Media-server users** who already run Plex, Jellyfin, Infuse. They
   want each VOD as a trio — `.mp4` + `.nfo` + `-thumb.jpg` — in a
   `<Streamer>/Season YYYY-MM/` folder so their existing library
   scanner picks it up without configuration.
2. **Local-only users** who treat the library as a plain file tree and
   browse it in Finder / Explorer. They find the sidecar files
   cluttering; they prefer a single-file-per-VOD layout with a
   predictable flat per-streamer folder.

A hardcoded choice will annoy half of the user base. A single "mixed"
layout satisfies neither.

## Decision

A `LibraryLayout` trait in `domain::library_layout` with two
implementations — `PlexLayout` and `FlatLayout` — gated on a settings
value (`app_settings.library_layout`, defaulting to `"plex"`).

The trait is narrow by design:

```rust
pub trait LibraryLayout: Send + Sync {
    fn path_for(&self, v: &VodWithStreamer) -> PathBuf;
    fn sidecars_for(&self, v: &VodWithStreamer) -> Vec<SidecarFile>;
    fn thumbnail_path(&self, v: &VodWithStreamer) -> PathBuf;
    fn describe(&self) -> &'static str;
}
```

- `path_for` returns the relative path for the video file.
- `sidecars_for` enumerates extra files the layout wants written
  alongside the video (NFO + thumbnail for Plex; empty for Flat —
  the flat layout puts the thumb under a hidden `.thumbs/` dir).
- `thumbnail_path` is separate because Flat puts it in a different
  parent directory.

Paths are computed from the pure `VodWithStreamer` DTO; no I/O
happens inside the trait. The service layer joins the returned
relative path onto the user-chosen library root. ADR-0012 owns the
atomic-move story.

Filename sanitization (`domain::sanitize`) is shared by both
backends.

Switching layouts is a user action — there's a migrator
(`services::library_migrator`) that walks every `completed` row and
atomically moves each file to the new layout. See §Consequences.

## Consequences

### Positive

- Two user experiences land cleanly without an either/or trade-off.
- Adding a third layout later (e.g. "Plex with .srt subtitles" or
  "Jellyfin with `season-specials` convention") is a contained
  change — implement the trait, add a new `LibraryLayoutKind`
  variant, extend the migrator's `from → to` matrix.
- Unit tests treat the trait as pure data. No filesystem needed in
  the domain test suite.

### Negative / accepted risks

- **Layout switching moves real files.** A user who flips the
  toggle after downloading hundreds of VODs triggers a background
  task that writes to disk. We mitigate with the atomic-move
  semantics (ADR-0012), a `library_migrations` audit row, and
  progress events the UI renders as a non-blocking toast. Errors
  during migration are counted but do not cascade — partial
  migrations finish cleanly, and re-running the migration (flipping
  back then forward) picks up what was left behind.
- **The NFO is Kodi-compatible, not Plex-native.** Plex prefers
  `.meta.json` in some configurations. Infuse and Jellyfin both
  accept Kodi NFO; Plex's `Personal Media` agent also accepts it.
  We ship Kodi NFO because it's the widest-compatible lowest common
  denominator.
- **No subdirectory-per-show.** Both layouts keep VODs flat within
  a streamer folder (or a season folder). Grouping by game would
  multiply file paths and make media-server scanning confusing —
  every VOD has chapters from multiple games.

### Neutral

- Two layouts are enough for the foreseeable personas. The trait
  lets us add more without reworking the queue.

## Alternatives considered

1. **Single fixed layout.** Loses half the audience. Rejected.
2. **User-supplied path template** (`{streamer}/{date}_{id}.mp4`).
   Powerful but a support burden — invalid templates crash at
   runtime, users ask "why doesn't my Plex see this". Rejected;
   can be added later behind a power-user setting.
3. **No layout migration on switch; just change future downloads.**
   Leaves the library in a half-and-half state that neither Plex
   nor Finder users are happy with. Rejected.

## References

- `src-tauri/src/domain/library_layout.rs` — trait + both impls.
- `src-tauri/src/domain/nfo.rs` — Kodi NFO generator.
- `src-tauri/src/services/library_migrator.rs` — layout switch
  migrator.
- `docs/user-guide/library-layouts.md` — user-facing explanation.
