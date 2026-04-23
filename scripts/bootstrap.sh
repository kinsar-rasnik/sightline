#!/usr/bin/env bash
# Sightline bootstrap (macOS + Linux)
#
# Idempotent: safe to re-run. Verifies the toolchain, installs pnpm
# dependencies, and fetches pinned sidecar binaries. Does not touch the
# user's shell rc files.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

step() { printf "\n==> %s\n" "$*"; }
ok() { printf "    ok: %s\n" "$*"; }
fail() { printf "    fail: %s\n" "$*" >&2; exit 1; }

# ------------------------------------------------------------------
# Stray lockfile guard (pnpm is the only supported package manager).
# ------------------------------------------------------------------
if [ -f package-lock.json ] || [ -f yarn.lock ]; then
  fail "found package-lock.json or yarn.lock — this repo uses pnpm only (see ADR-0006). Remove the stray lockfile."
fi

# ------------------------------------------------------------------
# Toolchain checks.
# ------------------------------------------------------------------
step "checking rust"
command -v cargo >/dev/null 2>&1 || fail "rust not found — install from https://rustup.rs"
rustc --version
required_rust="1.90.0"
have_rust="$(rustc --version | awk '{print $2}')"
printf -v sorted "%s\n%s" "$required_rust" "$have_rust"
if [ "$(printf "%s" "$sorted" | sort -V | head -n1)" != "$required_rust" ]; then
  fail "rust ${have_rust} is older than required ${required_rust}"
fi
ok "rust >= ${required_rust}"

step "checking node"
command -v node >/dev/null 2>&1 || fail "node not found — install Node 20+ (nvm / fnm recommended)"
node_major="$(node -v | sed 's/^v//' | cut -d. -f1)"
if [ "$node_major" -lt 20 ]; then
  fail "node $node_major is older than required 20"
fi
ok "node $(node -v)"

step "checking pnpm"
command -v pnpm >/dev/null 2>&1 || fail "pnpm not found — install with 'npm i -g pnpm@10' or 'corepack enable pnpm'"
pnpm_major="$(pnpm --version | cut -d. -f1)"
if [ "$pnpm_major" -lt 9 ]; then
  fail "pnpm $(pnpm --version) is older than required 9"
fi
ok "pnpm $(pnpm --version)"

# ------------------------------------------------------------------
# Platform deps (Linux webview libs). macOS is covered by Xcode CLT.
# ------------------------------------------------------------------
if [ "$(uname -s)" = "Linux" ]; then
  step "checking linux webview deps"
  missing=()
  for pkg in libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev; do
    dpkg -s "$pkg" >/dev/null 2>&1 || missing+=("$pkg")
  done
  if [ "${#missing[@]}" -gt 0 ]; then
    printf "    missing apt packages: %s\n" "${missing[*]}" >&2
    printf "    install with: sudo apt-get install %s\n" "${missing[*]}" >&2
    exit 1
  fi
  ok "linux webview deps present"
fi

# ------------------------------------------------------------------
# pnpm install with frozen lockfile.
# ------------------------------------------------------------------
step "installing pnpm dependencies"
pnpm install --frozen-lockfile
ok "pnpm install"

# ------------------------------------------------------------------
# Sidecar bundler (yt-dlp, ffmpeg). Stub in Phase 1.
# ------------------------------------------------------------------
step "bundling sidecars"
if [ -x "./scripts/bundle-sidecars.sh" ]; then
  ./scripts/bundle-sidecars.sh
  ok "sidecars bundled"
else
  printf "    skip: bundle-sidecars.sh not yet implemented (lands with ADR-0003 in Phase 3)\n"
fi

step "done"
printf "Next: pnpm tauri dev\n"
