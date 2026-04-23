#!/usr/bin/env bash
# bundle-sidecars.sh — fetch pinned yt-dlp and ffmpeg binaries into src-tauri/binaries/.
#
# Phase 1 stub: validates that scripts/sidecars.lock parses and prints a dry-run.
# The real implementation lands in Phase 3 alongside ADR-0003.
#
# When implemented, behavior will be:
#   1. Read scripts/sidecars.lock (YAML) — each entry has url + sha256.
#   2. For the current OS + arch, download the matching files.
#   3. Verify sha256; reject on mismatch.
#   4. Write to src-tauri/binaries/<name>-<target-triple>.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

lock="scripts/sidecars.lock"
if [ ! -f "$lock" ]; then
  echo "bundle-sidecars: $lock not found — Phase 3 creates this file." >&2
  exit 0
fi

echo "bundle-sidecars (dry run)"
echo "  lockfile: $lock"
echo "  os: $(uname -s)  arch: $(uname -m)"
echo "  (Phase 3 will download and verify binaries to src-tauri/binaries/)"
