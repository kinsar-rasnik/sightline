---
description: Commit and PR conventions
glob: "**/*"
---

# Commit and PR rules

## Conventional Commits

- Subject: `type(scope): summary` where `type ∈ {feat, fix, docs, chore, refactor, test, build, ci, perf, style}`.
- Summary in the imperative, under 72 chars, no trailing period.
- Body (when present) explains **why**, not what.
- Footer holds `Closes #N` and co-author trailers.

## Scope hygiene

- One logical change per commit. If a refactor enables a feature, land the refactor first.
- No "WIP", "fix earlier", "typo" commits on main — rebase them away.
- Formatting-only commits use `style:` and touch no other content.

## Quality gate

- Every commit on `main` passes the full quality gate locally before push:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `pnpm typecheck`
  - `pnpm lint`
  - `pnpm test`
- If CI fails on `main`, that is an incident — stop and fix, do not layer commits.

## PRs

- Branch from latest `main`. Rebase on conflict; no merge commits in topic branches.
- Title follows the same Conventional Commits format.
- Description includes: summary, screenshots (UI), testing notes, breaking changes, links to ADRs.
- The PR checklist in `.github/PULL_REQUEST_TEMPLATE.md` is authoritative.

## Reversibility

- If a change is not trivially reversible via `git revert`, the PR description explains the downgrade path.
- Schema migrations are forward-only; a rollback ADR is filed when reverting a merged migration is genuinely needed.
