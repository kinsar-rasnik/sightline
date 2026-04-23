# ADR-0006 — Package manager and lockfile policy

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CTO, Senior Engineer

## Context

Deterministic builds are a non-negotiable governance principle (see the synthetic-workforce blueprint §2). On the Node side we have npm, yarn (classic, berry), pnpm, and bun. Each has a different lockfile format, disk footprint, and CI story.

Requirements:

- **Reproducible installs.** A lockfile that pins every transitive, with integrity hashes.
- **Fast CI.** pnpm's content-addressable store shaves minutes off a matrix build once cached.
- **Workspace support.** Even if we ship a single package today, we may split a CLI or docs workspace later.
- **Good fit with Tauri tooling.** Tauri's `beforeDevCommand` / `beforeBuildCommand` must be trivially invocable.

On the Rust side, Cargo is the default; `Cargo.lock` policy is the only debate.

## Decision

Node side:

- **pnpm** is the only supported package manager.
- `pnpm-lock.yaml` is committed. `packageManager` in `package.json` pins the pnpm version.
- Hoisting is controlled by `.npmrc` — `strict-peer-dependencies=true`, `auto-install-peers=true`, `prefer-workspace-packages=true`.
- `npm` and `yarn` are actively discouraged; CI fails if it detects a `package-lock.json` or `yarn.lock`.

Rust side:

- `Cargo.lock` is committed for the application crate (we ship binaries).
- Dependency ranges use **minimum-compatible** semver (`serde = "1"`), relying on the lockfile for exact versions. Exceptions require an inline comment.

Both sides:

- Dependency upgrades happen in dedicated PRs titled `chore(deps): ...`, each with a CHANGELOG-style note in the PR body covering why.
- Dependabot is enabled for both `cargo` and `pnpm` (weekly, grouped).
- `cargo audit` and `pnpm audit --prod` run on CI and block on high-severity advisories.

## Consequences

Positive:

- **Reproducible installs on CI and on a contributor's fresh clone.** No silent minor-version drift.
- **Fast matrix runs.** pnpm's content-addressable store is friendly to GitHub Actions caches.
- **Clear upgrade discipline.** Dependency bumps are observable, reviewable, and isolated.

Negative:

- **Lockfile churn on Dependabot days.** Accepted — the alternative is drift.
- **Slight onboarding friction.** New contributors sometimes reach for `npm i`. The bootstrap script and CONTRIBUTING.md head this off.
- **Cargo.lock commits show up in review noise.** Mitigation: include the lockfile diff but read only the summary at the top.

## Mitigations

- `scripts/bootstrap.sh` and `.ps1` detect stray `package-lock.json` / `yarn.lock` and fail early.
- CONTRIBUTING.md documents pnpm as the only supported manager.
- A top-level `.npmrc` pins the engine to pnpm.

## Alternatives considered and rejected

- **yarn (berry).** Rejected — PnP's file resolution friction with Tauri and some native-binding crates.
- **npm.** Rejected — slower, lockfile less deterministic historically (though improving).
- **bun.** Rejected for now — the ecosystem is still catching up on Windows / Tauri integration. Revisit in Phase 7.

## Supersedes / superseded by

- None.
