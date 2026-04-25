#!/usr/bin/env bash
#
# release-notes.sh — generate Markdown release notes from Conventional
# Commits between two refs.
#
# Usage:
#   ./scripts/release-notes.sh                 # auto: previous tag → HEAD
#   ./scripts/release-notes.sh v0.1.0          # explicit previous ref
#   ./scripts/release-notes.sh v0.1.0 v1.0.0   # explicit start..end
#
# Behaviour:
#   - Reads `git log <start>..<end> --pretty=format:'%h|%s'`.
#   - Buckets subjects by Conventional Commit prefix:
#       feat       → ## Features
#       fix        → ## Bug fixes
#       perf       → ## Performance
#       refactor / test / docs / chore / style / build / ci
#                  → ## Other
#       no prefix  → ## Uncategorised
#   - Emits Markdown to stdout (workflow captures into release-notes.md).
#
# The `parse_subject` helper is exported so a Vitest spec can exercise
# the parsing logic without spawning git (see
# scripts/release-notes.test.ts).
set -euo pipefail

START=""
END="HEAD"

if [[ "${1:-}" != "" && "${1:-}" != "--from-stdin" ]]; then
  START="$1"
fi
if [[ "${2:-}" != "" ]]; then
  END="$2"
fi

# parse_subject: prints the bucket name for a single commit subject.
# We avoid bash 4+ `;&` fallthroughs because macOS ships /bin/bash 3.2,
# and the GitHub Actions runners we target should not differ in how
# this script runs in dev vs CI.
parse_subject() {
  local subject="$1"
  case "$subject" in
    feat\(*\):*|feat:*)         echo "feat" ;;
    fix\(*\):*|fix:*)           echo "fix" ;;
    perf\(*\):*|perf:*)         echo "perf" ;;
    refactor\(*\):*|refactor:*) echo "other" ;;
    test\(*\):*|test:*)         echo "other" ;;
    docs\(*\):*|docs:*)         echo "other" ;;
    chore\(*\):*|chore:*)       echo "other" ;;
    style\(*\):*|style:*)       echo "other" ;;
    build\(*\):*|build:*)       echo "other" ;;
    ci\(*\):*|ci:*)             echo "other" ;;
    *)                          echo "uncategorised" ;;
  esac
}

if [[ "${1:-}" == "--from-stdin" ]]; then
  # Test/dev mode: read pre-built `<sha>|<subject>` lines from stdin.
  LINES=$(cat)
else
  if [[ -z "$START" ]]; then
    # No explicit start ref — pick the most recent tag reachable from the parent
    # of the end ref.  `2>/dev/null || true` keeps the very first release case
    # (no prior tag) from blowing up; we fall back to the full repo history.
    START=$(git describe --tags --abbrev=0 "${END}^" 2>/dev/null || true)
  fi
  RANGE="${START:+${START}..}${END}"
  LINES=$(git log "$RANGE" --no-merges --pretty=format:'%h|%s')
fi

declare -a feat=() fix=() perf=() other=() uncat=()

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  sha="${line%%|*}"
  subject="${line#*|}"
  bucket=$(parse_subject "$subject")
  case "$bucket" in
    feat) feat+=("- ${subject} (${sha})") ;;
    fix)  fix+=("- ${subject} (${sha})") ;;
    perf) perf+=("- ${subject} (${sha})") ;;
    other) other+=("- ${subject} (${sha})") ;;
    *)     uncat+=("- ${subject} (${sha})") ;;
  esac
done <<< "$LINES"

emit_section() {
  local heading="$1"
  shift
  if [[ "$#" -gt 0 ]]; then
    printf '## %s\n\n' "$heading"
    for entry in "$@"; do
      printf '%s\n' "$entry"
    done
    printf '\n'
  fi
}

if [[ -n "$START" && "${1:-}" != "--from-stdin" ]]; then
  printf 'Released from `%s` (commits %s..%s).\n\n' "$END" "$START" "$END"
elif [[ "${1:-}" != "--from-stdin" ]]; then
  printf 'Released from `%s` (initial release).\n\n' "$END"
fi

emit_section "Features" "${feat[@]+"${feat[@]}"}"
emit_section "Bug fixes" "${fix[@]+"${fix[@]}"}"
emit_section "Performance" "${perf[@]+"${perf[@]}"}"
emit_section "Other" "${other[@]+"${other[@]}"}"
emit_section "Uncategorised" "${uncat[@]+"${uncat[@]}"}"
