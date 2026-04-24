# ADR-0013 — Pinned, verified sidecar bundling for yt-dlp and ffmpeg

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 4)
- **Supersedes / informs.** [ADR-0003](0003-pinned-sidecar-binaries.md) (original
  "we will ship pinned sidecars" decision). This ADR operationalises it.

## Context

Phase 3 landed the full download engine against `YtDlpFake`, with a real-binary
fallback that looked on PATH. `scripts/bundle-sidecars.sh` was a stub; no
binaries were actually fetched, hashed, or bundled. Phase 3's session report
flagged this as the #1 open question — without it a `tauri build` ships an
app that does nothing useful until the end user installs yt-dlp and ffmpeg
themselves.

The goals for a real bundling story:

1. **Reproducible builds** — same source input on two machines produces the same
   bundle, byte-for-byte. This rules out "latest" tags and rolling URLs.
2. **Verifiable chain of custody** — every binary is verified against a
   committed SHA-256 *before* it is ever executed or extracted. A compromised
   CDN cannot substitute a trojan.
3. **Per-platform static binaries** — no runtime lib drift across macOS
   releases, glibc versions, or MSVC runtimes. Static where possible.
4. **Cross-platform tooling parity** — bash for macOS + Linux + CI, PowerShell
   for Windows devs who don't have WSL. Same lockfile, same behaviour.
5. **First-run experience** — `pnpm tauri build` on a fresh clone should
   succeed without requiring the developer to know about sidecar plumbing.

## Decision

Ship a pinned lockfile + two platform-native bundler scripts + a verifier that
the pre-push gate, CI, and runtime health-check all run.

### Lockfile (`scripts/sidecars.lock`)

Pipe-delimited for zero-dependency parsing in both bash and PowerShell. One row
per `(tool, target-triple)`; fields are `name|triple|url|sha256|binary_name|archive_kind|archive_entry`.

```
yt-dlp|aarch64-apple-darwin|https://github.com/.../yt-dlp_macos|e80c47…|yt-dlp_macos||
ffmpeg|x86_64-unknown-linux-gnu|https://github.com/BtbN/.../ffmpeg-…-linux64-gpl.tar.xz|e15ef9…|ffmpeg|tar.xz|ffmpeg-.../bin/ffmpeg
```

Triples use the Rust target-triple form, matching Tauri's `externalBin`
convention (Tauri copies `src-tauri/binaries/<name>-<triple>[.exe]` into the
produced installer).

### Sources

| Tool | Platform | Publisher | Notes |
|------|----------|-----------|-------|
| yt-dlp | all 5 | [`yt-dlp/yt-dlp` releases](https://github.com/yt-dlp/yt-dlp/releases) | Hashes copied from the upstream `SHA2-256SUMS` asset on the pinned tag. |
| ffmpeg | macOS arm64 | [osxexperts.net](https://www.osxexperts.net/) (ffmpeg 8.1) | Linked from ffmpeg.org's download page as the canonical static-build publisher for macOS. Hash is the SHA-256 of the upstream `.zip` streamed via `curl -sL | shasum -a 256` at pin time. |
| ffmpeg | macOS x86_64 | [osxexperts.net](https://www.osxexperts.net/) (ffmpeg 8.0) | 8.1 Intel not currently published; the 8.0 build is the most recent Intel static available. |
| ffmpeg | Linux x86_64 + arm64, Windows x86_64 | [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) autobuild tag | Hashes copied from the upstream `checksums.sha256` asset. Dated autobuild tags are immutable; `latest` is not used. |

Static linking is preferred everywhere. The `-gpl` BtbN variant is used
intentionally — it bundles x264/x265/libvpx which we need for the Twitch
source formats.

### Bundler scripts

- `scripts/bundle-sidecars.sh` (bash, macOS + Linux + CI `ubuntu-latest` +
  `macos-latest`).
- `scripts/bundle-sidecars.ps1` (PowerShell 7+, Windows devs + CI
  `windows-latest`).

Both scripts implement the same four-step pipeline per row:

1. **Detect** the host target-triple (or honour `--triple`).
2. **Fetch** the pinned URL to a content-addressed cache outside the repo
   (`~/.cache/sightline-sidecars` on Unix, `%LOCALAPPDATA%\sightline-sidecars`
   on Windows). The cache key includes the SHA-256, so a lockfile bump never
   collides with an older download.
3. **Verify** SHA-256 before anything else happens. A mismatch deletes the
   candidate and aborts with exit 3. *The downloaded file is never executed,
   extracted, or renamed prior to this step.*
4. **Install** the binary to `src-tauri/binaries/<name>-<triple>[.exe]` —
   either by copy (raw-binary downloads like yt-dlp) or by extracting the
   `archive_entry` from the archive (ffmpeg's tarball/zip).

The `--all` flag fetches every triple (used by release builds that want a
multi-arch bundle). `--dry-run` prints the plan. `--force` bypasses the
"already installed, hash matches" shortcut.

### Verifier (`scripts/verify-sidecars.sh`)

- Used by: `scripts/verify.sh` (pre-push), CI (after `bundle-sidecars`,
  before the Rust test run), and eventually `cmd_health_check` (Phase 5 adds
  the tray tooltip).
- Checks per lockfile row for the host triple: file exists at
  `src-tauri/binaries/<name>-<triple>[.exe]`, SHA-256 matches for raw-binary
  rows, and — with `--smoke` — `yt-dlp --version` / `ffmpeg -version` exits 0.
- JSON mode (`--json`) emits a machine-readable report for the IPC layer to
  surface a "sidecars unavailable" banner without re-implementing the parse.

### CI integration

New steps in the `test` matrix, before `cargo test`:

```yaml
- name: cache sidecar downloads
  uses: actions/cache@v4
  with:
    path: ~/.cache/sightline-sidecars
    key: sidecars-${{ runner.os }}-${{ hashFiles('scripts/sidecars.lock') }}

- name: bundle sidecars (unix)
  if: matrix.os != 'windows-latest'
  run: ./scripts/bundle-sidecars.sh

- name: bundle sidecars (windows)
  if: matrix.os == 'windows-latest'
  shell: pwsh
  run: ./scripts/bundle-sidecars.ps1

- name: verify sidecars
  run: ./scripts/verify-sidecars.sh --smoke
```

The cache key hashes the lockfile so bumps invalidate it automatically; a
fresh lockfile will cold-download once per OS, then every subsequent run is
cache-hit. This is the first time real sidecars execute in CI — the smoke
step is deliberate: it confirms the bundled binary can actually start on each
OS, not just that the bytes landed.

### Runtime integration

- `src-tauri/build.rs` re-exports the Rust `TARGET` env var as
  `TARGET_TRIPLE` so runtime code can build the same path the bundler
  produced.
- `lib.rs::resolve_sidecar` now tries, in order, `binaries/<name>-<triple>[.exe]`
  (canonical), then `binaries/<name>[.exe]` (Tauri's auto-stripped name), then
  a repo-relative `src-tauri/binaries/<name>-<triple>[.exe]` (so
  `pnpm tauri dev` resolves without shipping a bundle).
- `tauri.conf.json` declares `bundle.externalBin: ["binaries/yt-dlp", "binaries/ffmpeg"]`.
  Tauri's bundler will refuse to build if either file is missing for the
  target triple — the error is actionable ("run scripts/bundle-sidecars.sh").

### First-run + refresh procedure

Refresh (when upstream ships a new yt-dlp release we want):

1. Pull the new `SHA2-256SUMS` from the yt-dlp release page.
2. Open `scripts/sidecars.lock` and replace the five `yt-dlp` rows with the
   new tag + hashes (one hash for `yt-dlp_macos`, one for `yt-dlp_linux`, one
   for `yt-dlp_linux_aarch64`, one for `yt-dlp.exe`; the two macOS rows share
   a hash because the same universal binary is used).
3. Bump the comment header block's `Hashes copied from…` line to the new
   URL.
4. Run `scripts/bundle-sidecars.sh --force` locally to prove the new hashes
   match. Commit.

Same flow for ffmpeg. For BtbN, pick a recent `autobuild-YYYY-MM-DD-HH-MM`
tag and pull its `checksums.sha256`; for macOS, `curl -sL $URL | shasum -a 256`
the osxexperts zip and paste the result.

## Alternatives considered

### A. `cargo install yt-dlp-bin` / brew-install at runtime

Rejected. Runtime install hands trust to the user's package manager (apt, brew,
MS Store, winget) and breaks the reproducible-build promise. The moment a
user's brew cache pulls a different yt-dlp than CI tested with, we cannot
reproduce their bug reports. Also "runs without network at install time" is
a non-goal we want to keep.

### B. Git LFS for the binaries

Rejected. LFS inflates clone time for any contributor (even doc-only changes
pull the full binary set), and GitHub's LFS bandwidth quota is a real cost.
Content-addressed cache outside the repo gives the same de-duplication without
pulling 200+ MB on every clone.

### C. Docker image with sidecars baked in

Rejected for Phase 4 — we target native desktop builds, not container
distribution. The CI runners are already platform-native; a Docker layer adds
complexity without reducing the attack surface (we'd still need per-platform
binaries inside the container).

### D. Build ffmpeg from source in CI

Rejected. Build time would dwarf the rest of CI (20-30 minutes per platform).
Static-build publishers do this for us with verifiable hashes; we pin their
output.

## Consequences

**Positive.**
- `pnpm tauri build` now produces a real, self-contained installer on every
  supported platform.
- An attacker compromising a CDN can't substitute a trojaned yt-dlp — the
  hash check catches it before any code runs.
- `scripts/verify-sidecars.sh` becomes the single source of truth the runtime
  health-check will call.
- CI exercises real sidecars on all three platforms on every push; the
  smoke-test step will catch "the binary won't even load" regressions the
  moment they appear.

**Costs accepted.**
- The lockfile needs manual refresh when we want a newer yt-dlp or ffmpeg.
  Low frequency (quarterly-ish for yt-dlp; rarer for ffmpeg).
- BtbN autobuild tags are permanent but not semver-tagged; readers have to
  trust that a `ffmpeg-N-124093-g…` tag captures a particular commit of
  master. This is acceptable because every release of theirs ships with a
  hash checksum file.
- First CI run per-lockfile bump is slow (cold download of ~140 MB of
  ffmpeg per OS). `actions/cache` keyed on the lockfile hash amortises this
  to a one-time cost per bump.

**Risks.**
- **Source takedown.** If osxexperts disappears we have no macOS ffmpeg. The
  mitigation is to use the existing cache — because hashes are pinned,
  a long-lived local cache remains valid even if the URL 404s. Secondary
  mitigation would be a contingency mirror, deferred until we see evidence
  of a real risk.
- **Hash-file integrity.** The upstream `SHA2-256SUMS` file is fetched over
  HTTPS from github.com; we trust the github.com CA chain. yt-dlp also
  publishes a `SHA2-256SUMS.sig` signed with their release key; a future
  revision of this ADR could add signature verification. Explicit
  deferred-to-Phase-7 item.

## Follow-ups

1. Add GPG signature verification for yt-dlp's `SHA2-256SUMS.sig` (Phase 7
   release-polish track).
2. Add a `cmd_health_check` extension that invokes
   `scripts/verify-sidecars.sh --json` and surfaces the result in the
   Settings "Diagnostics" tile (Phase 5).
3. Consider an auto-refresh path that opens a PR bumping the pins when
   yt-dlp ships a release with a security advisory. Nice-to-have, not on
   the roadmap yet.
