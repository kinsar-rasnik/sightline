#!/usr/bin/env bash
# scripts/verify.sh — full local quality gate.
#
# Runs the same checks CI runs, in roughly the same order. Exit non-zero
# on the first failure. Prints a compact summary at the end.
#
# Usage:
#   ./scripts/verify.sh          # run everything
#   ./scripts/verify.sh --rust   # skip frontend
#   ./scripts/verify.sh --web    # skip Rust
#   ./scripts/verify.sh --fast   # skip the Rust integration tests and the
#                                # Vite production build (quickest sanity
#                                # pass — ~30% of full runtime)
#   ./scripts/verify.sh --no-sidecars  # skip the sidecar hash check
#                                # (useful on a fresh clone before the
#                                #  binaries have been fetched)
#
# hotfix-camelcase.md §Follow-up #2 and hotfix-ci.md both asked for a
# scripted gate that runs pre-push. This is that script. Required by
# CONTRIBUTING.md before every push.

set -euo pipefail

MODE="all"
FAST=0
SKIP_SIDECARS=0
for arg in "$@"; do
  case "$arg" in
    --rust) MODE="rust" ;;
    --web)  MODE="web" ;;
    --fast) FAST=1 ;;
    --no-sidecars) SKIP_SIDECARS=1 ;;
    -h|--help)
      sed -n '2,18p' "$0"
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

# Colors when stdout is a TTY; plain text otherwise (CI, pipes).
if [ -t 1 ]; then
  # shellcheck disable=SC2034
  BOLD=$'\033[1m'; GREEN=$'\033[32m'; RED=$'\033[31m'; DIM=$'\033[2m'; RESET=$'\033[0m'
else
  BOLD=""; GREEN=""; RED=""; DIM=""; RESET=""
fi

step() {
  printf '%s==> %s%s\n' "$BOLD" "$1" "$RESET"
}

pass() {
  printf '%s ok %s — %s\n' "$GREEN" "$RESET" "$1"
}

fail() {
  printf '%s FAIL %s — %s\n' "$RED" "$RESET" "$1" >&2
  exit 1
}

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

run_rust() {
  step "Rust: cargo fmt --check"
  (cd src-tauri && cargo fmt --all -- --check) || fail "cargo fmt"
  pass "cargo fmt"

  step "Rust: cargo clippy --all-targets --all-features -- -D warnings"
  (cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings) || fail "cargo clippy"
  pass "cargo clippy"

  if [ "$FAST" -eq 0 ]; then
    step "Rust: cargo test --all-features"
    (cd src-tauri && cargo test --all-features) || fail "cargo test"
    pass "cargo test"
  else
    printf '%sskip%s cargo test (--fast)\n' "$DIM" "$RESET"
  fi
}

run_web() {
  step "Web: pnpm typecheck"
  pnpm typecheck || fail "pnpm typecheck"
  pass "pnpm typecheck"

  step "Web: pnpm lint"
  pnpm lint || fail "pnpm lint"
  pass "pnpm lint"

  step "Web: pnpm test"
  pnpm test || fail "pnpm test"
  pass "pnpm test"

  if [ "$FAST" -eq 0 ]; then
    step "Web: pnpm build"
    pnpm build || fail "pnpm build"
    pass "pnpm build"
  else
    printf '%sskip%s pnpm build (--fast)\n' "$DIM" "$RESET"
  fi
}

run_sidecars() {
  step "Sidecars: scripts/verify-sidecars.sh"
  # Tolerated if the binaries just aren't downloaded on this machine yet —
  # the gate flags them with a clear instruction, but we don't hard-fail
  # the pre-push unless the binaries are present *and* corrupt (exit 1
  # from verify-sidecars covers both cases; the actionable recovery is the
  # same: run scripts/bundle-sidecars.sh).
  if ./scripts/verify-sidecars.sh; then
    pass "sidecars"
  else
    fail "verify-sidecars (run scripts/bundle-sidecars.sh to install pinned yt-dlp + ffmpeg)"
  fi
}

case "$MODE" in
  all)
    if [ "$SKIP_SIDECARS" -eq 0 ]; then run_sidecars; fi
    run_rust
    run_web
    ;;
  rust)
    if [ "$SKIP_SIDECARS" -eq 0 ]; then run_sidecars; fi
    run_rust
    ;;
  web) run_web ;;
esac

printf '\n%s✓ verify passed%s\n' "$GREEN$BOLD" "$RESET"
