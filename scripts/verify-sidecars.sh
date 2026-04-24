#!/usr/bin/env bash
# verify-sidecars.sh — quick sanity check for the host's bundled sidecars.
#
# Confirms that src-tauri/binaries/<name>-<triple>[.exe] exists for the host
# triple, and (when the binary is the raw download) matches the lockfile
# SHA-256. Used by:
#   - scripts/verify.sh  (pre-push gate)
#   - cmd_health_check   (runtime guard; the cmd calls this with --json)
#   - CI                 (after bundle-sidecars, before tauri build)
#
# Exit codes:
#   0  all required sidecars present and, where applicable, hash-verified
#   1  at least one sidecar is missing or has the wrong hash
#   2  lockfile not found or lockfile parse error
#
# Usage:
#   scripts/verify-sidecars.sh            # host triple only
#   scripts/verify-sidecars.sh --triple x86_64-unknown-linux-gnu
#   scripts/verify-sidecars.sh --smoke    # also run `--version` on each binary
#   scripts/verify-sidecars.sh --json     # machine-readable report on stdout

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

LOCKFILE="scripts/sidecars.lock"
OUT_DIR="src-tauri/binaries"

TRIPLE=""
SMOKE=0
JSON=0

while [ $# -gt 0 ]; do
  case "$1" in
    --triple) TRIPLE="$2"; shift 2 ;;
    --smoke)  SMOKE=1; shift ;;
    --json)   JSON=1; shift ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

if [ ! -f "$LOCKFILE" ]; then
  echo "lockfile missing: $LOCKFILE" >&2
  exit 2
fi

detect_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in arm64|aarch64) echo "aarch64-apple-darwin" ;; x86_64) echo "x86_64-apple-darwin" ;; esac ;;
    Linux)
      case "$arch" in aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;; x86_64) echo "x86_64-unknown-linux-gnu" ;; esac ;;
    MINGW*|MSYS*|CYGWIN*) echo "x86_64-pc-windows-msvc" ;;
  esac
}

TRIPLE="${TRIPLE:-$(detect_triple)}"
if [ -z "$TRIPLE" ]; then
  echo "could not detect host triple" >&2
  exit 2
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

suffix_for() {
  case "$1" in *-pc-windows-msvc) echo ".exe" ;; *) echo "" ;; esac
}

errors=0
entries="["
sep=""

while IFS='|' read -r name triple url sha binary archive_kind archive_entry; do
  # Skip comments / blanks.
  case "$name" in ""|\#*) continue ;; esac
  [ "$triple" = "$TRIPLE" ] || continue
  suffix="$(suffix_for "$triple")"
  final="$OUT_DIR/${name}-${triple}${suffix}"
  status="ok"
  detail=""
  if [ ! -f "$final" ]; then
    status="missing"
    detail="expected at $final"
    errors=$((errors + 1))
  else
    if [ -z "$archive_kind" ]; then
      got="$(sha256_of "$final")"
      if [ "$got" != "$sha" ]; then
        status="hash_mismatch"
        detail="expected $sha got $got"
        errors=$((errors + 1))
      fi
    fi
    if [ "$status" = "ok" ] && [ "$SMOKE" -eq 1 ]; then
      case "$name" in
        yt-dlp) "$final" --version >/dev/null 2>&1 || { status="smoke_failed"; detail="$final --version"; errors=$((errors + 1)); } ;;
        ffmpeg) "$final" -version  >/dev/null 2>&1 || { status="smoke_failed"; detail="$final -version";  errors=$((errors + 1)); } ;;
      esac
    fi
  fi

  if [ "$JSON" -eq 1 ]; then
    entries="${entries}${sep}{\"name\":\"$name\",\"triple\":\"$triple\",\"path\":\"$final\",\"status\":\"$status\",\"detail\":\"$detail\"}"
    sep=","
  else
    case "$status" in
      ok)
        if [ "$SMOKE" -eq 1 ]; then
          printf ' ok  %s  (hash + smoke)  %s\n' "$name-$triple" "$final"
        else
          printf ' ok  %s  %s\n' "$name-$triple" "$final"
        fi
        ;;
      *) printf ' FAIL %s  %s — %s\n' "$name-$triple" "$status" "$detail" >&2 ;;
    esac
  fi
done < "$LOCKFILE"

if [ "$JSON" -eq 1 ]; then
  entries="${entries}]"
  printf '{"triple":"%s","errors":%d,"sidecars":%s}\n' "$TRIPLE" "$errors" "$entries"
fi

if [ "$errors" -gt 0 ]; then
  if [ "$JSON" -eq 0 ]; then
    echo >&2
    echo "verify-sidecars: $errors issue(s). Run scripts/bundle-sidecars.sh to install." >&2
  fi
  exit 1
fi
exit 0
