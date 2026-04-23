#!/usr/bin/env bash
# Dependency audit: run cargo audit and pnpm audit, emit a consolidated report.
#
# Exit code:
#   0 — no high-severity advisories
#   1 — at least one high-severity advisory
#
# The report is written to ./target/audit-report.md so CI can attach it.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mkdir -p target
report="$repo_root/target/audit-report.md"
: > "$report"

echo "# Dependency audit report" >> "$report"
echo "" >> "$report"
echo "_Generated $(date -u +'%Y-%m-%dT%H:%M:%SZ') UTC_" >> "$report"
echo "" >> "$report"

fail=0

# --- Rust ---------------------------------------------------------------------
echo "## Cargo (Rust)" >> "$report"
echo "" >> "$report"

if ! command -v cargo-audit >/dev/null 2>&1; then
  echo "cargo-audit not installed — install with: cargo install cargo-audit --locked" | tee -a "$report"
  fail=1
else
  pushd src-tauri >/dev/null
  if ! cargo audit --json 2>/dev/null > "$repo_root/target/cargo-audit.json"; then
    cargo_vulns="$(jq '.vulnerabilities.count // 0' "$repo_root/target/cargo-audit.json" 2>/dev/null || echo 0)"
    echo "- vulnerabilities: $cargo_vulns" >> "$report"
    if [ "$cargo_vulns" -gt 0 ]; then
      jq -r '.vulnerabilities.list[] | "  - [\(.advisory.severity)] \(.package.name) \(.package.version): \(.advisory.title)"' \
        "$repo_root/target/cargo-audit.json" 2>/dev/null >> "$report" || true
      fail=1
    fi
  else
    echo "- no advisories" >> "$report"
  fi
  popd >/dev/null
fi

echo "" >> "$report"

# --- pnpm ---------------------------------------------------------------------
echo "## pnpm (Node)" >> "$report"
echo "" >> "$report"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm not installed — run ./scripts/bootstrap.sh first" | tee -a "$report"
  fail=1
else
  pnpm_json="$(pnpm audit --prod --json 2>/dev/null || true)"
  echo "$pnpm_json" > "$repo_root/target/pnpm-audit.json"
  total="$(printf '%s' "$pnpm_json" | jq -r '.metadata.vulnerabilities.total // 0' 2>/dev/null || echo 0)"
  high="$(printf '%s' "$pnpm_json" | jq -r '.metadata.vulnerabilities.high // 0' 2>/dev/null || echo 0)"
  critical="$(printf '%s' "$pnpm_json" | jq -r '.metadata.vulnerabilities.critical // 0' 2>/dev/null || echo 0)"
  echo "- vulnerabilities: total=$total high=$high critical=$critical" >> "$report"
  if [ "$high" -gt 0 ] || [ "$critical" -gt 0 ]; then
    fail=1
  fi
fi

echo "" >> "$report"
echo "## Summary" >> "$report"
echo "" >> "$report"
if [ "$fail" = "0" ]; then
  echo "All green. No blocking advisories." >> "$report"
else
  echo "BLOCKING advisories present — see above." >> "$report"
fi

cat "$report"
exit "$fail"
