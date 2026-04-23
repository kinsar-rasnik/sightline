#!/usr/bin/env bash
# Stop hook: run the fast part of the quality gate when a session ends.
#
# Contract:
# - Reads a Claude Code Stop hook payload on stdin.
# - Honors `stop_hook_active` to avoid infinite loops.
# - Runs a *fast* subset of the quality gate (cargo check, pnpm typecheck).
# - On failure, prints an actionable summary to stderr and exits non-zero
#   so the transcript highlights the issue. It does not rewrite files.

set -euo pipefail

payload="$(cat)"

if ! command -v jq >/dev/null 2>&1; then
  echo "stop-gate: jq not found, skipping" >&2
  exit 0
fi

stop_active="$(printf '%s' "$payload" | jq -r '.stop_hook_active // false')"
if [ "$stop_active" = "true" ]; then
  exit 0
fi

root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -z "$root" ] && exit 0

cd "$root"

fail=0

if [ -f "src-tauri/Cargo.toml" ]; then
  if ! (cd src-tauri && cargo check --quiet 2>&1 | tail -20); then
    echo "stop-gate: cargo check failed — run 'cargo check' in src-tauri for details" >&2
    fail=1
  fi
fi

if [ -f "package.json" ] && [ -d "node_modules" ]; then
  if ! pnpm typecheck 2>&1 | tail -20; then
    echo "stop-gate: pnpm typecheck failed — run 'pnpm typecheck' for details" >&2
    fail=1
  fi
fi

exit "$fail"
