# ADR-0019 — Asset protocol scope for local playback

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 5)
- **Related.**
  [ADR-0011](0011-library-layout-pluggability.md) (path computation) ·
  [ADR-0012](0012-staging-atomic-move.md) (library-root containment
  invariant) ·
  [ADR-0014](0014-tray-daemon-architecture.md) (tray + daemon surface
  that survives a window close).

## Context

The Phase-5 player feeds the webview an `asset:/` or
`tauri://localhost/` URL that resolves to a local `.mp4` file. Tauri
makes this safe by narrowing the asset-protocol's allow-list to a
configured set of paths; without that narrowing, the webview could
load any file the app process can see.

The download pipeline's sanitiser + the
`pipeline_inner::starts_with(&library_root)` guard (introduced in
ADR-0012) already ensure that `final_path` never escapes the library
root on write. But the player reads from the filesystem via a
different code path — the webview's `<video>` element fetches the
asset URL directly. That path needs its own defence in depth.

## Decision

Two concentric safeguards, both required:

### 1. `MediaAssetsService` is the ONLY source of player paths

A new service in `src-tauri/src/services/media_assets.rs` is the
single choke point between "a vod_id the renderer asked for" and "an
absolute path the renderer can open". The service's
`get_video_source` handler:

1. Reads the `downloads` row for the vod_id; if the state isn't
   `completed`, returns `{ state: "partial" }` without a path.
2. Verifies `final_path.exists()`; if not, returns `{ state:
   "missing" }` — the player shows a "re-download" CTA.
3. Runs `guarded_path(&library_root, &final_path)` — canonicalises
   both sides where possible and rejects anything whose canonical
   path doesn't start with the canonical library root.
4. Returns `{ state: "ready", path: <absolute> }`.

The renderer's player component never constructs paths itself — it
calls `commands.getVideoSource({ vodId })` and feeds the returned
path through Tauri's `convertFileSrc` before handing it to the
`<video src>` attribute.

`get_vod_assets` (for thumbnail + preview frames) applies the same
`guarded_path` check to every path it returns.

`request_remux` — the recovery action the player offers when the
webview can't decode the file — looks up the target path via the
same service-level API and runs `guarded_path` before invoking
ffmpeg. An attacker who could override the webview's fetch URL can't
trick the remux command into running on an arbitrary file, because
the ffmpeg arg list is built from the DB row, not the webview input.

### 2. Tauri asset-protocol scope (future narrowing)

Tauri 2 lets us constrain the set of directories the asset protocol
will serve from, via the `fs` / `asset` capability allow-list. Today
our `capabilities/default.json` only grants `core:default` and the
webview uses the default asset handler which serves from the app's
resource directory. A future change (phase 5 follow-up) will
register a capability that pins the asset protocol to
`{libraryRoot}/**` so a hypothetical bypass of the service layer
still can't escape the library.

The service-layer guard is the primary defence; capability narrowing
is the belt-and-braces layer the Phase-7 release-polish pass adds.

## Alternatives considered

### A. Hand the webview raw filesystem paths

Rejected. `convertFileSrc` already does the right thing — the
primary question is *which* paths we're willing to convert.

### B. Serve video bytes through an in-process HTTP server

Rejected. Adds a concurrency + memory story (HTTP range handling,
backpressure, tls) for no benefit over Tauri's built-in asset
protocol.

### C. Copy every VOD into a dedicated "play-cache" outside the
library root

Rejected. Doubles disk usage and introduces a sync problem (file
moved externally) for negligible security gain — the asset-protocol
allow-list can already scope to the real library root.

## Consequences

**Positive.**
- The renderer has one entry point for playback paths. Anything
  trying to load from outside the library root fails at the DB
  layer before the filesystem ever sees the request.
- Missing-file + partial-download + decode-failure each surface as
  a distinct `VideoSourceState` discriminant, so the UI can render
  the right recovery action without parsing error messages.
- `request_remux` is safe by construction — its arg list is built
  from the DB row, not the webview.

**Costs accepted.**
- Every player open is a round-trip to `get_video_source`. Sub-
  millisecond on the local DB; imperceptible in practice.
- Capability-narrowing is a Phase-7 follow-up rather than in this
  phase — tracked in Open questions in the session report.

## Follow-ups

1. Tauri capability that narrows the asset protocol to
   `{libraryRoot}/**`. Phase 7 polish alongside code-signing.
2. Support for a secondary library root (user pointed Sightline at
   two disks). Would need to widen `guarded_path` to accept a
   Vec<PathBuf>.
3. Remux output format. We currently produce `.mp4`; if we ever
   support external players that prefer `.mkv` we'd want to
   parameterise the destination extension.
