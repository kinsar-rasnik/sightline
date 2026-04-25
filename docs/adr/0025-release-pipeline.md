# ADR-0025 — Release pipeline (GitHub Releases, unsigned binaries)

- **Status.** Accepted
- **Date.** 2026-04-25 (Phase 7)
- **Related.**
  [ADR-0006](0006-package-manager-and-lockfile-policy.md) (pinned
  toolchains the workflow reuses) ·
  [ADR-0013](0013-sidecar-bundling.md) (sidecars get bundled into
  the OS-specific installer in this pipeline) ·
  [ADR-0017](0017-audit-policy-until-release.md) (audit hardening
  that lands alongside).

## Context

v1.0 ships. We need a repeatable, observable way to turn a tagged
commit into installable artifacts on macOS, Windows, and Linux,
without manual hand-off. Sightline is a hobby-scale OSS project, not
a SaaS — there is no operations team paid to wait on a slow build.
The pipeline must be:

- triggerable from a single `git push` of a version tag,
- runnable on the GitHub-hosted matrix (no self-hosted runners),
- public and reproducible (any contributor can build the same
  artifact locally),
- explicit about its security posture (we're not Apple Developer
  ID-signed, we're not Microsoft EV-signed, the user has to consent
  to that on first launch).

## Decision

A new workflow `.github/workflows/release.yml`. Trigger:
`push: tags: ['v*.*.*']`. Two jobs:

1. **`build`** — runs on the matrix `[ubuntu-latest, macos-latest,
   windows-latest]`. Each job:
   - checks out the tag,
   - sets up Node 24 + Rust stable + pnpm via `packageManager`,
   - bundles sidecars (re-uses `scripts/bundle-sidecars.{sh,ps1}`,
     same SHA-256-verified path Phase 3 already uses),
   - runs `pnpm tauri build` which produces the per-OS bundle,
   - uploads the bundle directory as a GitHub Actions artifact.
2. **`release`** — runs on `ubuntu-latest`, `needs: build`. It:
   - downloads all three artifacts,
   - generates release notes from Conventional Commits via
     `scripts/release-notes.sh` (between the previous tag and the
     current one),
   - publishes a GitHub Release using
     `softprops/action-gh-release@v2`, attaching every artifact
     file as an asset. `prerelease: false`,
     `draft: false`, `make_latest: true`.

The workflow has `permissions: contents: write` for the release job
only. `build` is read-only (`permissions: contents: read`).

### Why unsigned binaries

Sightline is open source, MIT licensed, single-developer scoped, and
distributed via GitHub Releases. The two paths to "verified by the
OS without a security warning" are:

- **macOS** — Apple Developer ID + notarisation, $99/year, requires
  a CI secret with the cert.
- **Windows** — EV code-signing certificate, ~$200-400/year,
  requires either a hardware token or a cloud-signing service that
  also costs money.

Both costs fall on a single maintainer for a non-commercial project.
Both add a CI secret that, if leaked, lets an attacker sign
software as Sightline. The cost-benefit for v1 doesn't add up —
users who care about cryptographic provenance can build from source
(first-class supported path, see `docs/INSTALL.md` "Build from
source").

The release ships **unsigned** binaries with explicit per-OS
instructions for the first-launch warning:

- **macOS** — Right-click the .dmg/.app → Open, or
  `xattr -d com.apple.quarantine /Applications/Sightline.app`.
- **Windows** — SmartScreen "More info" → "Run anyway" on the
  installer.
- **Linux** — `chmod +x` the AppImage; `dpkg -i` the .deb.

We document the path explicitly in `docs/INSTALL.md` so the OS
warning isn't surprising.

### Build matrix and artifact naming

Four targets, one runner each (the dual-arch macOS path covers Apple
Silicon natively):

| Target                       | Runner          | Artifacts          |
| ---------------------------- | --------------- | ------------------ |
| `x86_64-apple-darwin`        | macos-13        | `.dmg`             |
| `aarch64-apple-darwin`       | macos-latest    | `.dmg`             |
| `x86_64-pc-windows-msvc`     | windows-latest  | `.msi`, `.exe`     |
| `x86_64-unknown-linux-gnu`   | ubuntu-latest   | `.AppImage`, `.deb` |

The two macOS targets ship as separate `.dmg` files
(`Sightline_<version>_x64.dmg` and `Sightline_<version>_aarch64.dmg`)
rather than a fat universal2 binary; users pick the one matching
their CPU. Universal2 doubles the binary size for no functional gain
and adds an `lipo` link step to the bundle pipeline; the separate-
artifact path is simpler and more transparent.

Linux ships AppImage + deb on a single ubuntu-latest job. The
glibc-2.35 baseline (ubuntu-22.04 image) covers every mainstream
desktop distro. Older-glibc builds would need a self-hosted runner;
out of scope for v1.

### Release notes generation

`scripts/release-notes.sh` is a small bash script (with a TS Vitest
test mocking the git-log output for unit coverage). It:

1. Resolves the previous tag via `git describe --tags --abbrev=0
   HEAD^` (falling back to `HEAD~50` when there is no previous
   tag, which is the v1.0 case).
2. Runs `git log <prev>..HEAD --pretty=format:'%h %s'`.
3. Buckets each line by Conventional Commit type:
   - `feat` → ## Features
   - `fix` → ## Bug fixes
   - `perf` → ## Performance
   - `refactor`, `test`, `docs`, `chore`, `style`, `build`, `ci` →
     ## Other
4. Emits Markdown to stdout, which the workflow captures into
   `release-notes.md` and passes to `softprops/action-gh-release`
   via `body_path`.

Subjects without a recognised Conventional Commit prefix go in a
trailing `## Uncategorized` bucket, but that's a fallback — repo
policy enforces the prefix on every commit (see
`.claude/rules/commits.md`).

The script is intentionally not a Node tool — it's invoked from the
workflow runner and the workflow already has `bash` everywhere.

### What runs after a tag push

```
git tag v1.0.0 && git push origin v1.0.0
        │
        ▼
release.yml triggers
        │
   ┌────┼────┐                     ┌─────────────┐
   ▼    ▼    ▼                     ▼             ▼
build   build   build         release-notes   action-gh-release@v2
ubuntu  macos   windows                          │
   │    │    │                                   │
   └────┴────┘                                   ▼
   artifacts up-                              GitHub Release
   loaded to job                              v1.0.0 (public,
                                              with assets)
```

The CI workflow (`ci.yml`) is unchanged — it triggers on `push:
branches: [main]` and on PRs, never on tags. Release runs are
orthogonal to CI and don't gate on it: by the time a tag exists,
the underlying commit has already passed CI on `main`.

## Alternatives considered

### A. Sign and notarise on macOS via Apple Developer ID

Rejected for v1. Cost + secret-management overhead for a non-
commercial single-maintainer project. See "Why unsigned binaries"
above. Re-evaluate post-1.0 if Sightline grows a maintainer team or
a sponsorship.

### B. Sparkle / tauri-plugin-updater for in-app self-update

Rejected for v1. Self-update needs a signing infrastructure (an
unsigned self-replacing binary is a malware vector) and a hosted
update channel. ADR-0026 ships a notification-only update checker
instead — the user clicks through to GitHub Releases and downloads
manually.

### C. Self-hosted runners for older glibc

Rejected for v1. Glibc 2.35 (ubuntu-22.04 / 24.04 image base) covers
mainstream desktops. We don't ship to LTS-server distros. The flexion
point would be a "Sightline runs on my Pop_OS 22.04" issue; defer.

### D. Reproducible builds infrastructure (lockfile pinning + bit-
identical bundles)

Out of scope for v1. The Cargo.lock + pnpm-lock.yaml + sidecar
SHA-256-verification (ADR-0013) gives us source determinism but the
Tauri bundler does not currently produce bit-identical bundles
across runs. Worth tracking but not blocking.

### E. Publish to package managers (Homebrew, winget, AUR)

Out of scope for v1. Each one has its own metadata + maintenance
overhead. Add post-1.0 once the tag-push-to-release flow has been
exercised a few times and the asset-naming scheme has stabilised.

## Consequences

**Positive.**
- Releasing v1.0.0 is `git tag v1.0.0 && git push --tags`. No manual
  upload step.
- Every release has cryptographic provenance via the GitHub Actions
  log + the artifact's SHA (visible on the release page). A user
  who really wants to verify can compare against a local
  `pnpm tauri build` output.
- Build-from-source is the recommended path for security-sensitive
  users — `INSTALL.md` documents it as first-class.

**Costs accepted.**
- Unsigned binaries trip OS-level "unverified developer" warnings on
  first launch. Documented per-OS workaround.
- macOS ships x86_64-only for v1. ARM-native is a Q3 follow-up.
- One CI secret (`GITHUB_TOKEN`, automatic) plus zero developer
  certs on either OS.

**Risks.**
- An attacker who compromises the repo's tag-push capability could
  publish a malicious release. Mitigated by branch protection on
  `main` (PR + green CI required) and the senior-engineer-only
  tag-push convention. Tags don't bypass branch protection but do
  bypass review; the tag is set by the senior engineer (Claude
  Code) only after the merge to `main` lands.
- A future OS update that hardens the gatekeeper warning could
  break the documented workaround. We track this as a
  documentation-update risk; the scripted fallback `xattr -d` path
  is durable.

## Follow-ups

- Apple Developer ID signing + notarisation (re-evaluate when
  there's funding).
- Windows EV signing (same).
- Homebrew tap, winget manifest, AUR package (post-1.0).
- Package SHA-256 manifest as a release asset so users can verify
  manually (low-effort follow-up).

## References

- `.github/workflows/release.yml` (created in this phase)
- `scripts/release-notes.sh` (created in this phase)
- `scripts/release-notes.test.ts` (Vitest unit coverage)
- `src-tauri/tauri.conf.json` (`bundle` block updated for v1)
- `docs/INSTALL.md` (created in this phase)
