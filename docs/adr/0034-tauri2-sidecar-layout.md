# ADR-0034 — Tauri 2 sidecar bundle layout (corrects ADR-0013)

- **Status.** Accepted
- **Date.** 2026-04-26 (v2.0.2 hotfix)
- **Related.** [ADR-0013](0013-sidecar-bundling.md) — the original
  pinned-sidecar bundling decision.  The lockfile + verifier + CI
  pipeline from ADR-0013 are unchanged; only the *runtime path
  resolution* assumption documented in ADR-0013 §"Runtime
  integration" is corrected here.

## Context

v2.0.1 macOS `.dmg` shipped on 2026-04-26 and a real-user bug report
the same evening showed:

```
sidecar: spawn: No such file or directory (os error 2)
```

…the moment the user opened Settings → Video Quality (which
triggers the Phase 8 encoder-detection probe).

Diagnosis:

1. The bundled sidecars (`yt-dlp-aarch64-apple-darwin`,
   `ffmpeg-aarch64-apple-darwin`) are physically present and
   executable inside the `.app`, at
   `Sightline.app/Contents/MacOS/`.
2. `src-tauri/src/lib.rs::resolve_sidecar` resolves via
   `tauri::path::resolve(BaseDirectory::Resource)`, which on macOS
   maps to `Sightline.app/Contents/Resources/`.
3. `Contents/Resources/` does not contain the sidecars.  The
   probe falls through every candidate and `resolve_sidecar`
   returns `None`; the caller then falls back to invoking
   `yt-dlp` / `ffmpeg` on PATH, which on a typical macOS install
   does not exist.

Root cause: **Tauri 2 changed the sidecar bundle layout**
relative to Tauri 1.  ADR-0013 documents the Tauri 1 convention
(sidecars under `Contents/Resources/binaries/<name>-<triple>`),
which is what the original `resolve_sidecar` was written for.
Tauri 2's `tauri::bundle::externalBin` configuration now places
sidecars next to the main executable in every bundle format we
ship.

This regression survived v2.0.0 + v2.0.1 because:

- The CI sidecar-smoke step (ADR-0013 §"CI integration") tests
  the binaries under `src-tauri/binaries/` directly, not via the
  bundled-app resolution path.
- No CI step actually opens a built `.dmg` / `.AppImage` /
  `.msi`, navigates to a sidecar-dependent feature, and asserts
  the spawn succeeds.

The hotfix has to (a) correct the runtime resolution and (b) add
a CI test that catches this class of bug going forward.

## Decision

### Tauri 2 bundle layouts (concrete paths)

| Bundle | Main binary path | Sidecar path |
| --- | --- | --- |
| macOS `.app` | `<App>.app/Contents/MacOS/sightline` | `<App>.app/Contents/MacOS/<name>-<triple>` |
| Linux `.deb` | `/usr/bin/sightline` | `/usr/bin/<name>-<triple>` |
| Linux `.AppImage` | `<mount>/usr/bin/sightline` (FUSE-mounted to `/tmp/.mount_<random>/` at run time) | same `<mount>/usr/bin/` |
| Windows `.msi` / `-setup.exe` | `<install dir>/sightline.exe` | `<install dir>/<name>-<triple>.exe` |

The unifying invariant is: **the sidecar lives in the same
directory as the launched main binary** on every supported OS.
This is what `find_sidecar_in_dir(dir, name, triple, ext)` probes
when called with `dir = current_exe().parent()`.

### Probe order (post-fix)

```
1. current_exe().canonicalize().parent()
   → find_sidecar_in_dir tries `<name>-<triple>[.exe]`, then `<name>[.exe]`.
2. BaseDirectory::Resource
   → resolve `binaries/<name>-<triple>[.exe]`, then `binaries/<name>[.exe]`.
3. src-tauri/binaries/ (repo-relative)
   → same two-form probe.
```

Step 1 is the canonical Tauri 2 path.  Step 2 is retained for
forward compatibility (a future bundle format could place
sidecars under Resources/) and as an extra safety net for users
on legacy builds.  Step 3 is the dev workflow — `pnpm tauri dev`
runs out of `src-tauri/target/...` and the sidecars live in
`src-tauri/binaries/` next to the lockfile.

### `canonicalize()` rationale

`current_exe()` may return a symlink-bearing path on Linux
(AppImage's FUSE-mounted squashfs binary is an `AppRun` shim
that `exec()`s the real binary).  Canonicalising before the
`.parent()` strip ensures the directory we probe is the one
the kernel actually launched the executable from.  We use
`unwrap_or(exe_raw)` so a canonicalisation failure (very
unusual on a running binary) doesn't break the probe — it just
falls back to the raw path, which is correct on macOS and
Windows where symlinks aren't part of the AppImage layer.

### CI coverage

`src-tauri/tests/sidecar_smoke.rs` now contains both:

- **Real-binary smoke tests** (legacy ADR-0013 layer): probe the
  raw `src-tauri/binaries/<name>-<triple>[.exe]` and run
  `--version` to confirm the bundler produced something runnable.
- **Bundle-layout simulation tests** (NEW): synthesise the
  expected directory layout for each OS in a tempdir
  (Sightline.app/Contents/MacOS/, /usr/bin/,
  Program Files/Sightline/) and assert
  `find_sidecar_in_dir` discovers the sidecar in the right
  place.  These run on every OS in the CI matrix without a
  full `pnpm tauri build`.

The simulation tests catch the regression class shipped in
v2.0.1: any future Tauri-version-bump or bundler-change that
moves the sidecar destination breaks the test before it
reaches production.

## Alternatives considered

### A. Switch to `tauri-plugin-shell` and use `Command::new_sidecar`

`tauri-plugin-shell` provides a `Command::new_sidecar(name)` API
that internally handles the Tauri 2 path-resolution.  Switching
would eliminate `resolve_sidecar` entirely.

Rejected for v2.0.2 scope: pulling a new plugin into the
dependency graph is broader than the hotfix mandate, has its own
permission/capability surface to audit, and changes the
`tokio::process::Command` -> spawn pattern that the existing
download / encoder-detection code already uses.  Filed as a
v2.1 candidate.

### B. Hardcode `Contents/MacOS/` etc. per OS

Possible — `cfg(target_os = ...)` branches in `resolve_sidecar`
that explicitly point at the OS-specific bundle subdirectory.
Rejected because `current_exe().parent()` already gives us this
information dynamically without baking platform knowledge into
the resolver.  The dynamic approach also handles future
bundle-format changes (e.g., a `.flatpak` target) without code
changes.

### C. Fix at bundler level (force sidecars into `Resources/`)

Tauri 2's `externalBin` config doesn't expose a destination
override.  The bundler decides where the sidecar lives per
target.  Fighting Tauri's convention would require a custom
bundle-post-processing step that copies binaries from
`Contents/MacOS/` to `Contents/Resources/binaries/`.  Rejected
as fragile (every Tauri version bump risks breaking the
post-processing) and against the upstream pattern.

## Consequences

**Positive.**
- macOS builds work end-to-end on first launch (encoder
  detection, downloads, re-encodes).
- The same fix lands Linux + Windows since all three converge
  on "same dir as main binary" — no per-OS branching.
- The bundle-layout simulation tests run on every PR matrix
  job, catching regressions before tag.

**Costs accepted.**
- The hotfix doesn't *test* the actual bundle on each OS at
  CI time (we only simulate the layout).  A future enhancement
  could add a `pnpm tauri build && spawn-and-grep` step to the
  release workflow, but the cost (~10-15 min per OS per release)
  isn't justified for a Phase 8 follow-up — the simulation
  tests catch the regression class we've actually seen.

**Risks.**
- Tauri 2.x might further change the bundle layout in a future
  version bump.  The simulation tests would fail; we'd update
  the layouts and re-pin the Tauri version.
- The `canonicalize()` fallback is a `unwrap_or(raw)` — if both
  the canonical path and raw `.parent()` resolve to a directory
  without the sidecar, we silently fall through to step 2.
  Acceptable because step 2 is itself a fallback for the case
  step 1 missed.

## Follow-ups

- Adopt `tauri-plugin-shell::Command::new_sidecar` (alternative A)
  in v2.1 if the plugin's permission surface is worth the cleanup.
- Add a release-workflow post-bundle step that opens each
  produced bundle and checks the expected sidecar paths exist.
  Cheap (filesystem inspection only, no execution); catches
  bundler-side regressions complementary to the simulation tests.
- Mark the relevant subsection of ADR-0013 ("Runtime integration")
  as superseded by this ADR.

## References

- `src-tauri/src/lib.rs::resolve_sidecar` + `find_sidecar_in_dir`
- `src-tauri/tests/sidecar_smoke.rs` (bundle-layout simulation
  tests)
- ADR-0013 (sidecar bundling pipeline, unchanged)
- Tauri 2 docs on `bundle.externalBin`:
  <https://v2.tauri.app/develop/sidecar/>
