<!-- Follow Conventional Commits in the PR title: feat(scope): summary -->

## Summary

<!-- One or two sentences on what this change does and why. -->

## Screenshots or recordings

<!-- Required for any user-facing change. Delete this section otherwise. -->

## Testing

<!-- How did you verify? Command transcripts, or steps a reviewer can re-run. -->

## Checklist

- [ ] Branch is rebased on latest `main` (no merge commits).
- [ ] All commits follow Conventional Commits.
- [ ] `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` passes locally.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` passes locally.
- [ ] New or changed behavior has tests (unit, integration, or UI).
- [ ] Public Rust items and IPC commands have doc comments.
- [ ] User-facing changes are captured under **Breaking changes** or **Behavior changes** below.
- [ ] An ADR is added or updated under `docs/adr/` when this change picks between two or more credible approaches.

## Breaking changes

<!-- List any behavior that existing users will notice. Delete this section if none. -->

## Related

<!-- Closes #123, Part of #456, Supersedes #789, etc. -->
