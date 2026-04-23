# ADR-0003 — Pinned sidecar binaries (yt-dlp, ffmpeg)

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CTO, Senior Engineer

## Context

Sightline downloads Twitch VODs by invoking **yt-dlp** as an external process and (in later phases) may invoke **ffmpeg** for thumbnail extraction. These tools are upstream projects with their own release cadence. Two tensions pull on us:

1. **Reproducibility.** Our CI and our users should run the same binary so we can diagnose regressions. An auto-updating sidecar breaks deterministic builds and makes incident response harder.
2. **Freshness.** Twitch changes its endpoints and protections frequently. A yt-dlp release is often the only fix. Being six months behind upstream is operationally painful.

Shipping unpinned sidecars (just running `yt-dlp` from the user's `PATH`) is the easiest but the worst for reproducibility.

## Decision

We ship **version-pinned, checksum-verified** sidecar binaries.

- Pinned versions live in `scripts/sidecars.lock` (YAML), with entries per OS + arch + binary name:
  ```
  yt-dlp:
    version: "2026.03.14"
    platforms:
      macos-arm64:
        url: "https://github.com/yt-dlp/yt-dlp/releases/download/2026.03.14/yt-dlp_macos"
        sha256: "<64 hex>"
      ...
  ```
- `pnpm bundle-sidecars` (wrapping `scripts/bundle-sidecars.sh` / `.ps1`) downloads each file, verifies the checksum, and places it under `src-tauri/binaries/` with Tauri's expected `<name>-<target-triple>` layout.
- CI refuses to build if any entry in the lockfile is missing a checksum or the downloaded bytes disagree.
- Users who want a newer yt-dlp edit the lockfile and re-run the bundler. There is no implicit "latest" path.

## Consequences

Positive:

- Same binary across CI, releases, and a user's machine. Bug reports are actionable.
- No implicit network access during a release build — the bundler runs once, in advance.
- Upgrades are explicit, reviewable PRs that touch `sidecars.lock`.

Negative:

- We occasionally lag upstream on hot fixes. The manual-upgrade discipline is the trade-off for reproducibility.
- Initial clone requires one extra bootstrap step (`pnpm bundle-sidecars`).
- Platform coverage is our responsibility: each supported OS/arch entry must be verified.

## Mitigations

- `scripts/bootstrap.sh` and `scripts/bootstrap.ps1` wrap the bundler so a fresh contributor runs one command.
- A monthly chore rotates the pinned versions; the ADR and changelog record the upstream changes consulted.
- In Phase 7, we consider signing the lockfile and emitting a verification event on first launch.

## Alternatives considered and rejected

- **Rely on the user's `PATH`.** Rejected — drifts across users, varies across OS package managers, and forbids offline installs.
- **Auto-update to latest on every launch.** Rejected — breaks reproducibility and can surprise users with behavior changes at the worst moment.
- **Compile yt-dlp from source.** Rejected — yt-dlp is a Python project, which would add a Python runtime dependency we do not otherwise need.

## Supersedes / superseded by

- None.
