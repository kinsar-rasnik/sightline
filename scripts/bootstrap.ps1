# Sightline bootstrap (Windows)
#
# Idempotent: safe to re-run. Verifies the toolchain, installs pnpm
# dependencies, and fetches pinned sidecar binaries. Does not touch the
# user's PowerShell profile.

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Step([string]$msg) { Write-Host "`n==> $msg" }
function Ok([string]$msg)   { Write-Host "    ok: $msg" }
function Fail([string]$msg) { Write-Error "    fail: $msg"; exit 1 }

if (Test-Path "package-lock.json") { Fail "found package-lock.json — this repo uses pnpm only (see ADR-0006)." }
if (Test-Path "yarn.lock")         { Fail "found yarn.lock — this repo uses pnpm only (see ADR-0006)." }

Step "checking rust"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Fail "rust not found — install from https://rustup.rs"
}
$rustVersion = (rustc --version).Split(" ")[1]
Ok "rust $rustVersion"

Step "checking node"
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  Fail "node not found — install Node 20+"
}
$nodeMajor = [int]((node -v).TrimStart("v").Split(".")[0])
if ($nodeMajor -lt 20) { Fail "node $nodeMajor is older than required 20" }
Ok "node $(node -v)"

Step "checking pnpm"
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
  Fail "pnpm not found — install with 'npm i -g pnpm@10' or 'corepack enable pnpm'"
}
$pnpmMajor = [int]((pnpm --version).Split(".")[0])
if ($pnpmMajor -lt 9) { Fail "pnpm $(pnpm --version) is older than required 9" }
Ok "pnpm $(pnpm --version)"

Step "installing pnpm dependencies"
pnpm install --frozen-lockfile
Ok "pnpm install"

Step "bundling sidecars"
if (Test-Path ".\scripts\bundle-sidecars.ps1") {
  & .\scripts\bundle-sidecars.ps1
  Ok "sidecars bundled"
} else {
  Write-Host "    skip: bundle-sidecars.ps1 not yet implemented (lands with ADR-0003 in Phase 3)"
}

Step "done"
Write-Host "Next: pnpm tauri dev"
