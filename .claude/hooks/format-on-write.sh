#!/usr/bin/env bash
# PostToolUse hook: format the file that was just written or edited.
#
# Contract:
# - Reads a Claude Code PostToolUse payload on stdin.
# - Formats only the one file named in `tool_input.file_path`.
# - Silent on success. Stderr on failure. Never modifies other files.
# - Fast: individual invocations should complete in under 2s.

set -euo pipefail

payload="$(cat)"

if ! command -v jq >/dev/null 2>&1; then
  echo "format-on-write: jq not found, skipping" >&2
  exit 0
fi

path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // empty')"
[ -z "$path" ] && exit 0
[ ! -f "$path" ] && exit 0

ext="${path##*.}"

case "$ext" in
  rs)
    if command -v rustfmt >/dev/null 2>&1; then
      rustfmt --edition 2024 --quiet "$path" 2>&1 || {
        echo "format-on-write: rustfmt failed on $path" >&2
        exit 0
      }
    fi
    ;;
  ts|tsx|js|jsx|json|md|css|yaml|yml)
    if command -v pnpm >/dev/null 2>&1 && [ -f "./node_modules/.bin/prettier" ]; then
      ./node_modules/.bin/prettier --log-level silent --write "$path" 2>&1 || {
        echo "format-on-write: prettier failed on $path" >&2
        exit 0
      }
    fi
    ;;
  sh)
    if command -v shfmt >/dev/null 2>&1; then
      shfmt -w -i 2 -ci "$path" 2>&1 || {
        echo "format-on-write: shfmt failed on $path" >&2
        exit 0
      }
    fi
    ;;
  *)
    ;;
esac

exit 0
