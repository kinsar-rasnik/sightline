# ADR-0027 — Asset protocol scope narrowing (closes ADR-0019 follow-up)

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 7)
- **Supersedes.** Nothing.
- **Related.**
  [ADR-0019](0019-asset-protocol-scope.md) — established the
  service-layer choke point and tracked capability-narrowing as a
  Phase-7 polish follow-up. ·
  [ADR-0011](0011-library-layout-pluggability.md) — defines what
  "library root" means.

## Context

[ADR-0019](0019-asset-protocol-scope.md) made
`MediaAssetsService` the only path that hands the renderer a video
file path. The service-layer guard rejects any path whose canonical
form doesn't start with the canonical library root, and `request_remux`
sources its ffmpeg arg list from the DB row rather than the renderer.

That is the primary defence. Phase 5 deferred a secondary
defence — narrowing Tauri's asset protocol to the library root as a
capability-level allow-list — to a Phase-7 polish pass. The
deferral was tracked as a low-severity finding in the Phase-6
security review.

This ADR closes that follow-up.

## Decision

Two changes:

1. **Tauri capability `library` (new file
   `src-tauri/capabilities/library.json`)** scoped to the configured
   `library_root`, declared at runtime via
   `app.asset_protocol_scope().allow(library_root)`. The
   `tauri.conf.json` `app.security.assetProtocolScope` block names
   `$APPDATA/sightline/library/**` as the static (compile-time) scope
   for installs that haven't yet picked a custom library root. The
   runtime call extends the allow-list to the user's chosen
   `library_root`, scoped to a `**` glob.
2. **Setup hook update** — when the user changes their `library_root`
   via `update_settings`, the runtime capability scope is recomputed
   and pushed to the Tauri asset-protocol allowlist before the
   settings update returns. The pre-existing path stays in the
   allowlist for the lifetime of the process so any in-flight
   `<video>` element that was loaded under the old root still plays;
   the new library root joins the set, no path is removed mid-
   session. (A library-root change is rare; the allow-list capping
   is purely paranoia.)

The full guard stack now looks like:

```
webview <video src="asset:/...">
  ↓
Tauri asset protocol — checks against assetProtocolScope (allowlist)
  ↓                                        ❌ "scope mismatch"
Resource layer — opens the file, returns bytes
  ↓
(All player paths originate from MediaAssetsService::guarded_path,
 ADR-0019, which already validates against the library_root.)
```

The scope narrowing is **belt-and-braces**: the service-layer guard
is still the primary defence, but a renderer that bypasses
`getVideoSource` and constructs an `asset:/` URL by hand now hits
the protocol allowlist before any filesystem call.

### Why a runtime allow-list extension (rather than purely static
config)

The library root is **user-configurable** (Phase 3 settings, plus
the migrator from ADR-0011). A static scope in `tauri.conf.json` can
only know the bundled-default path. We update the allow-list at
process start (after `Db::migrate` settles, so the persisted
`library_root` is readable) and again on `update_settings(
{ libraryRoot: ... })`. This is the same cadence the existing
`download_queue.spawn` re-reads `library_root` already.

The static `assetProtocolScope` entry covers the
`$APPDATA/sightline/library/**` default so first-launch installs
where the user hasn't yet picked a library root still play their
test downloads (which land under the default).

### Capability declaration

`src-tauri/capabilities/library.json` is the new permission file:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "library",
  "description": "Asset-protocol allow-list for the configured library_root. Granted to the main window only; updated at runtime when library_root changes.",
  "windows": ["main"],
  "permissions": [
    {
      "identifier": "core:asset:default",
      "allow": [
        { "path": "$APPDATA/sightline/library/**" }
      ]
    }
  ]
}
```

Note: Tauri 2's permission scoping uses `core:asset:default` (or
similar) — the actual permission name is finalised against
`tauri-plugin-fs` 2.x docs at implementation time. Runtime
extension is via the Tauri-2 `app.asset_protocol_scope()`
extension API.

## Alternatives considered

### A. Don't narrow — keep the service-layer guard as the only
defence

Rejected. The service-layer guard relies on the renderer always
calling `getVideoSource` first. A bug or future feature that
constructs an `asset:/` URL without round-tripping the service
would slip past. Defence-in-depth catches that regression class.

### B. Static scope only (no runtime extension), forced
library_root location

Rejected. Forcing a fixed library-root path (e.g.
`$HOME/Sightline/library`) breaks the Phase-3 user-chosen-root
posture and wouldn't survive Plex / NAS users who want to point at
an external disk.

### C. Per-VOD allow-listing

Rejected. The asset-protocol allow-list operates on path globs;
adding/removing a glob per VOD would mean N glob registrations and
de-registrations on every library navigation. Disproportionate
complexity for a v1 polish item.

### D. Defer to v1.1

Rejected. The deferral was already taken once (Phase 5 → Phase 7);
deferring again would push it past v1.0 and the security-review
finding would carry into the public release. ADR-0019's follow-up
is closed here.

## Consequences

**Positive.**
- A renderer-side bypass of `getVideoSource` no longer reaches
  the filesystem: the Tauri asset protocol rejects the request
  before resource I/O.
- The `library` capability file becomes the central place to
  reason about which paths the webview can load. Future pen
  testers grep for `assetProtocolScope` and find the answer in one
  place.

**Costs accepted.**
- One new capability file (~20 lines).
- Runtime allow-list extension means a 1-time `O(1)` Tauri API
  call at startup and on `library_root` change. Imperceptible.
- The static fallback path (`$APPDATA/sightline/library/**`) is
  always allow-listed even after the user picks a custom root.
  Acceptable: nothing in the default path could harm a user who
  never used it (the directory is empty), and we keep it as a
  failsafe so a corrupt settings row doesn't lock the player out.

**Risks.**
- Tauri's asset-protocol scope syntax changes between minor
  versions. The capability JSON pins the schema URL so the build
  fails fast on a bad upgrade rather than silently de-narrowing.
- A future "secondary library root" feature (mentioned in
  ADR-0019's follow-ups) would need to list both roots. Add a
  note when we ship that.

## Follow-ups

- Secondary library root → both go in the allow-list (deferred
  from ADR-0019).
- Asset-protocol scope test in the integration suite. v1 ships
  the unit test that verifies the capability file's JSON
  structure; a runtime test that asserts a non-library path
  fails to load is a Phase-8 hardening item.

## References

- `src-tauri/capabilities/library.json`
- `src-tauri/tauri.conf.json` (`app.security.assetProtocolScope`)
- `src-tauri/src/lib.rs` (`extend_asset_scope` setup hook)
- `docs/adr/0019-asset-protocol-scope.md`
