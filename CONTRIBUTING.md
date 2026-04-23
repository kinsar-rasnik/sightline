# Contributing to Sightline

Thanks for your interest! This guide covers what you need to contribute code, documentation, or bug reports to Sightline.

## Code of conduct

All contributors are expected to follow the [Contributor Covenant 2.1](CODE_OF_CONDUCT.md). Be kind, be specific, assume good faith.

## Ways to contribute

- **Bug reports** — open an issue using the **Bug report** template. Include OS, app version, steps to reproduce, and expected vs. actual behavior.
- **Feature requests** — open an issue using the **Feature request** template. Explain the user problem first; propose a solution second.
- **Documentation fixes** — open a PR directly. Typos, clarifications, and missing examples are always welcome.
- **Code** — see below.

## Development setup

Prerequisites:

- **Rust** stable, 1.90 or newer. Install via <https://rustup.rs>.
- **Node.js** 20 LTS or newer, and **pnpm** 9 or newer (`npm i -g pnpm`).
- **Platform build deps** for Tauri 2 — see <https://v2.tauri.app/start/prerequisites/>.
- **Git** 2.40 or newer.

One-time bootstrap:

```bash
./scripts/bootstrap.sh          # macOS / Linux
# or
./scripts/bootstrap.ps1         # Windows (PowerShell)
```

This installs pnpm deps, verifies the Rust toolchain, and downloads pinned sidecar binaries (yt-dlp, ffmpeg) into `src-tauri/binaries/`.

Daily workflow:

```bash
pnpm tauri dev      # run in development mode
pnpm test           # frontend tests (vitest)
cargo test          # backend tests
pnpm typecheck
pnpm lint
```

Before opening a PR, run the full quality gate:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
pnpm typecheck
pnpm lint
pnpm test
```

## Branching and commits

- Branch from `main`: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`.
- Use **Conventional Commits**: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `build:`, `ci:`.
  - Example: `feat(poll): defer VODs while streamer is live`
- Keep commits small and logical. One concept per commit. No "WIP" commits in a final PR — rebase them away.
- Reference issues in the body, not the title: `Closes #42`.

## Pull request checklist

- [ ] Branch is rebased on latest `main` (no merge commits).
- [ ] All commits follow Conventional Commits.
- [ ] The quality gate passes locally (see above).
- [ ] New or changed behavior has tests.
- [ ] Public APIs (Rust + IPC commands) have doc comments.
- [ ] User-facing changes are noted in the PR description under **Breaking changes** or **Behavior changes**.
- [ ] If an architectural decision was made, an ADR is added under `docs/adr/`.

## Architecture Decision Records

If your change picks between two or more credible approaches, add an ADR. Copy `docs/adr/0001-stack-choice-tauri-rust.md` as a template. Name the file with the next available four-digit number.

## Issue triage

All issues are labeled within 48 hours. Labels:

- `bug` — something is broken.
- `feat` — new capability.
- `docs` — documentation-only change.
- `good-first-issue` — small, well-scoped, ideal for newcomers.
- `help-wanted` — maintainers would welcome a community PR.
- `wontfix` / `duplicate` — explained in a comment before closing.

## Questions

Open a discussion rather than an issue, or ping a maintainer on the tracker.
