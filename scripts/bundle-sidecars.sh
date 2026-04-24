#!/usr/bin/env bash
# bundle-sidecars.sh — fetch, verify, and place pinned yt-dlp + ffmpeg binaries.
#
# Reads scripts/sidecars.lock for URL + SHA-256 per (tool, target-triple),
# downloads the pinned asset, verifies the hash BEFORE the binary is ever
# executed or extracted, and places the final executable at
#   src-tauri/binaries/<tool>-<target-triple>[.exe]
#
# The output path matches Tauri's externalBin naming convention.
#
# Usage:
#   scripts/bundle-sidecars.sh                 # detect host triple
#   scripts/bundle-sidecars.sh --triple x86_64-unknown-linux-gnu
#   scripts/bundle-sidecars.sh --all           # fetch every pinned triple (CI)
#   scripts/bundle-sidecars.sh --cache DIR     # override download cache
#   scripts/bundle-sidecars.sh --dry-run       # print plan, do not download
#   scripts/bundle-sidecars.sh --force         # redownload even if hash matches
#
# Hash verification is non-negotiable. If a download's SHA-256 does not match
# the lockfile the script aborts with exit 3 and the candidate file is deleted.
# Nothing is extracted or run until after verification.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

LOCKFILE="scripts/sidecars.lock"
OUT_DIR="src-tauri/binaries"
DEFAULT_CACHE="${SIGHTLINE_SIDECAR_CACHE:-${HOME}/.cache/sightline-sidecars}"

TRIPLE=""
FETCH_ALL=0
CACHE_DIR=""
DRY_RUN=0
FORCE=0

if [ -t 1 ]; then
  BOLD=$'\033[1m'; GREEN=$'\033[32m'; RED=$'\033[31m'; DIM=$'\033[2m'; RESET=$'\033[0m'
else
  BOLD=""; GREEN=""; RED=""; DIM=""; RESET=""
fi

log()  { printf '%s==> %s%s\n' "$BOLD" "$1" "$RESET"; }
ok()   { printf '%s ok %s — %s\n' "$GREEN" "$RESET" "$1"; }
fail() { printf '%s FAIL %s — %s\n' "$RED" "$RESET" "$1" >&2; exit 3; }
dim()  { printf '%s%s%s\n' "$DIM" "$1" "$RESET"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --triple) TRIPLE="$2"; shift 2 ;;
    --all)    FETCH_ALL=1; shift ;;
    --cache)  CACHE_DIR="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force)  FORCE=1; shift ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *) fail "unknown flag: $1" ;;
  esac
done

CACHE_DIR="${CACHE_DIR:-$DEFAULT_CACHE}"
mkdir -p "$CACHE_DIR" "$OUT_DIR"

detect_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        x86_64)        echo "x86_64-apple-darwin" ;;
        *) fail "unknown macOS arch: $arch" ;;
      esac ;;
    Linux)
      case "$arch" in
        aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
        x86_64)        echo "x86_64-unknown-linux-gnu" ;;
        *) fail "unknown Linux arch: $arch" ;;
      esac ;;
    MINGW*|MSYS*|CYGWIN*) echo "x86_64-pc-windows-msvc" ;;
    *) fail "unsupported host OS: $os — use --triple to override" ;;
  esac
}

if [ "$FETCH_ALL" -eq 0 ] && [ -z "$TRIPLE" ]; then
  TRIPLE="$(detect_triple)"
fi

# Map target-triple → output filename suffix.
out_suffix_for() {
  case "$1" in
    *-pc-windows-msvc) echo ".exe" ;;
    *) echo "" ;;
  esac
}

# Stream-hash a local file.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    fail "neither sha256sum nor shasum found on PATH"
  fi
}

download() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --silent --show-error --max-time 600 -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget --quiet --output-document="$out" "$url"
  else
    fail "neither curl nor wget found on PATH"
  fi
}

# Extract archive_entry from archive and move to final binary path.
# The `entry` value is a trusted-by-commit lockfile field; even so we
# reject any form with a parent-directory traversal or absolute path so
# a malicious lockfile PR can't write outside `$dest`.
#
# Cleanup note: we do NOT use `trap 'rm -rf "$tmp"' RETURN` here. Bash's
# RETURN trap is NOT function-scoped by default (it only becomes scoped
# when `set -o functrace` is on, which it isn't). A RETURN trap set
# inside this function fires on EVERY subsequent function return, and
# the trap body references `$tmp` — which is out of scope in the caller
# and fatal under `set -u` ("tmp: unbound variable" on the caller's
# return line). Instead we do explicit cleanup on every exit path.
# `fail` always exits the whole script, so any leaked tmp dir there is
# the OS temp cleaner's problem.
extract_entry() {
  local archive="$1" kind="$2" entry="$3" dest="$4"
  case "$entry" in
    /*|*'..'*) fail "archive_entry must be a safe relative path (got: $entry)" ;;
  esac
  local tmp
  tmp="$(mktemp -d)"
  case "$kind" in
    zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -q -o "$archive" -d "$tmp"
      else
        rm -rf "$tmp"
        fail "unzip required to extract $archive"
      fi
      ;;
    tar.xz)
      if command -v tar >/dev/null 2>&1; then
        tar -C "$tmp" -xf "$archive"
      else
        rm -rf "$tmp"
        fail "tar required to extract $archive"
      fi
      ;;
    *)
      rm -rf "$tmp"
      fail "unsupported archive kind: $kind"
      ;;
  esac
  if [ ! -f "$tmp/$entry" ]; then
    rm -rf "$tmp"
    fail "archive entry not found: $entry inside $archive"
  fi
  mv "$tmp/$entry" "$dest"
  rm -rf "$tmp"
}

# Process a single lockfile row.
process_row() {
  local name="$1" triple="$2" url="$3" sha="$4" binary_name="$5" archive_kind="$6" archive_entry="$7" extracted_sha="${8-}"
  local suffix
  suffix="$(out_suffix_for "$triple")"
  local final="$OUT_DIR/${name}-${triple}${suffix}"

  # Fast cache-hit path: only skip the download/extract if we can prove the
  # installed binary matches a pinned hash. For raw binaries that's the
  # top-level sha256; for archive entries it's `extracted_sha256` (required
  # for a skip). The older "presence implies trusted" path is removed —
  # see phase-04.md security review findings.
  if [ "$FORCE" -eq 0 ] && [ -f "$final" ]; then
    local existing_sha
    existing_sha="$(sha256_of "$final")"
    if [ -z "$archive_kind" ]; then
      if [ "$existing_sha" = "$sha" ]; then
        dim "skip  $final (sha256 match)"
        return 0
      fi
    elif [ -n "$extracted_sha" ] && [ "$existing_sha" = "$extracted_sha" ]; then
      dim "skip  $final (extracted sha256 match)"
      return 0
    fi
    # Either hash mismatches or `extracted_sha256` not pinned yet — fall
    # through to re-download/extract against the verified archive.
  fi

  local download_name="$binary_name"
  if [ -z "$archive_kind" ]; then
    download_name="${name}-${triple}-raw${suffix:-.bin}"
  else
    download_name="${name}-${triple}.${archive_kind}"
  fi
  local cached="$CACHE_DIR/${sha}-${download_name}"

  if [ "$DRY_RUN" -eq 1 ]; then
    dim "plan  $name $triple  ← $url"
    dim "       sha256=$sha"
    dim "       cache=$cached  → $final"
    return 0
  fi

  if [ ! -f "$cached" ] || [ "$(sha256_of "$cached")" != "$sha" ]; then
    log "download $name $triple  ← $url"
    local tmp="${cached}.partial"
    # Clean a stale partial from a previous failed run.
    [ -f "$tmp" ] && rm -f "$tmp"
    if ! download "$url" "$tmp"; then
      rm -f "$tmp"
      fail "download failed: $url"
    fi
    local got
    got="$(sha256_of "$tmp")"
    if [ "$got" != "$sha" ]; then
      rm -f "$tmp"
      fail "sha256 mismatch for $url: expected $sha, got $got"
    fi
    mv "$tmp" "$cached"
    ok "verified $name $triple (sha256=$sha)"
  else
    dim "cache hit $name $triple"
  fi

  # Place into src-tauri/binaries/<name>-<triple>[.exe]
  if [ -z "$archive_kind" ]; then
    cp -f "$cached" "$final"
  else
    extract_entry "$cached" "$archive_kind" "$archive_entry" "$final"
  fi
  # Ensure executable bit on POSIX platforms.
  case "$triple" in
    *-pc-windows-msvc) : ;;
    *) chmod +x "$final" ;;
  esac
  ok "installed $final"

  # For archive entries, print the extracted binary's hash so a developer
  # can pin it into the lockfile's `extracted_sha256` column. Future runs
  # then take the fast skip path above.
  if [ -n "$archive_kind" ] && [ -z "$extracted_sha" ]; then
    local got_extracted
    got_extracted="$(sha256_of "$final")"
    dim "   extracted_sha256=$got_extracted (paste into scripts/sidecars.lock to pin)"
  fi
}

matching_rows() {
  # Emit rows that match the requested triple (or all rows when --all).
  local target="${1:-}"
  # Strip comments and blank lines; leave pipe-delimited rows intact.
  grep -v -E '^\s*(#|$)' "$LOCKFILE" | while IFS='|' read -r name triple url sha binary archive_kind archive_entry extracted_sha; do
    # Belt-and-braces for Windows dev runs: if .gitattributes is bypassed
    # and the lockfile arrives with CRLF, the last pipe-delimited field
    # keeps the `\r`. Strip it so downstream comparisons (sha matches,
    # empty-extracted_sha pin path) behave identically on every OS.
    extracted_sha="${extracted_sha%$'\r'}"
    if [ "$FETCH_ALL" -eq 1 ] || [ "$triple" = "$target" ]; then
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$name" "$triple" "$url" "$sha" "$binary" "$archive_kind" "$archive_entry" "${extracted_sha-}"
    fi
  done
}

if [ ! -f "$LOCKFILE" ]; then
  fail "$LOCKFILE not found"
fi

processed=0
while IFS=$'\t' read -r name triple url sha binary archive_kind archive_entry extracted_sha; do
  [ -z "$name" ] && continue
  process_row "$name" "$triple" "$url" "$sha" "$binary" "$archive_kind" "$archive_entry" "${extracted_sha-}"
  processed=$((processed + 1))
done < <(matching_rows "$TRIPLE")

if [ "$processed" -eq 0 ]; then
  if [ "$FETCH_ALL" -eq 1 ]; then
    fail "no rows parsed from $LOCKFILE"
  else
    fail "no lockfile entry for triple: $TRIPLE"
  fi
fi

printf '\n%s✓ bundle-sidecars installed %d binaries%s\n' "$GREEN$BOLD" "$processed" "$RESET"
