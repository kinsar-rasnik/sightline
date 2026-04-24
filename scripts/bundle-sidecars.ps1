# bundle-sidecars.ps1 — Windows twin of bundle-sidecars.sh.
#
# See scripts/bundle-sidecars.sh for the behaviour contract. This script
# reads the same scripts/sidecars.lock and emits binaries into
# src-tauri/binaries/<tool>-<target-triple>[.exe]
#
# Usage:
#   scripts\bundle-sidecars.ps1
#   scripts\bundle-sidecars.ps1 -Triple x86_64-pc-windows-msvc
#   scripts\bundle-sidecars.ps1 -All
#   scripts\bundle-sidecars.ps1 -Force
#   scripts\bundle-sidecars.ps1 -DryRun

[CmdletBinding()]
param(
  [string]$Triple = "",
  [switch]$All,
  [switch]$Force,
  [switch]$DryRun,
  [string]$Cache = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

$Lockfile = "scripts/sidecars.lock"
$OutDir   = "src-tauri/binaries"

if ([string]::IsNullOrEmpty($Cache)) {
  if ($env:SIGHTLINE_SIDECAR_CACHE) {
    $Cache = $env:SIGHTLINE_SIDECAR_CACHE
  } else {
    $Cache = Join-Path $env:LOCALAPPDATA "sightline-sidecars"
  }
}

New-Item -ItemType Directory -Force -Path $Cache, $OutDir | Out-Null

function Detect-Triple {
  # PowerShell 7+ exposes $IsWindows/$IsLinux/$IsMacOS; Windows PowerShell 5 does not.
  $os = if ($IsWindows -or $PSVersionTable.PSEdition -eq 'Desktop') { 'windows' }
        elseif ($IsLinux)  { 'linux' }
        elseif ($IsMacOS)  { 'macos' }
        else { throw "cannot detect host OS" }

  $arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString().ToLower()
  switch ("$os/$arch") {
    'windows/x64'    { 'x86_64-pc-windows-msvc' }
    'windows/arm64'  { 'x86_64-pc-windows-msvc' }  # ARM64 host, x64 sidecar via emulation
    'linux/x64'      { 'x86_64-unknown-linux-gnu' }
    'linux/arm64'    { 'aarch64-unknown-linux-gnu' }
    'macos/x64'      { 'x86_64-apple-darwin' }
    'macos/arm64'    { 'aarch64-apple-darwin' }
    default { throw "unsupported host: $os/$arch — pass -Triple explicitly" }
  }
}

if (-not $All -and [string]::IsNullOrEmpty($Triple)) {
  $Triple = Detect-Triple
}

function Out-Suffix([string]$t) {
  if ($t -like '*-pc-windows-msvc') { return '.exe' } else { return '' }
}

function SHA256-OfFile([string]$path) {
  return (Get-FileHash -Algorithm SHA256 -Path $path).Hash.ToLower()
}

function Download-File([string]$url, [string]$dest) {
  # Invoke-WebRequest handles redirects and TLS 1.2+ in recent PowerShell; force
  # TLS just in case we're on Windows PowerShell 5.1 with an older default.
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $dest
}

function Extract-Entry([string]$archive, [string]$kind, [string]$entry, [string]$dest) {
  # Defensive: reject archive_entry values that would escape the tmp dir.
  if ($entry.StartsWith('/') -or $entry.StartsWith('\') -or $entry -match '\.\.') {
    throw "archive_entry must be a safe relative path (got: $entry)"
  }
  $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
  New-Item -ItemType Directory -Force -Path $tmp | Out-Null
  try {
    switch ($kind) {
      'zip'    { Expand-Archive -LiteralPath $archive -DestinationPath $tmp -Force }
      'tar.xz' {
        # Windows 10+ ships bsdtar as `tar`, which understands xz via built-in libarchive.
        & tar -C $tmp -xf $archive
        if ($LASTEXITCODE -ne 0) { throw "tar extract failed for $archive" }
      }
      default { throw "unsupported archive kind: $kind" }
    }
    $src = Join-Path $tmp $entry
    if (-not (Test-Path $src)) {
      throw "archive entry not found: $entry inside $archive"
    }
    Move-Item -Force -Path $src -Destination $dest
  } finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  }
}

function Process-Row($row) {
  $name    = $row.name
  $triple  = $row.triple
  $url     = $row.url
  $sha     = $row.sha.ToLower()
  $binary  = $row.binary
  $akind   = $row.archive_kind
  $aentry  = $row.archive_entry
  $extractedSha = $row.extracted_sha.ToLower()

  $suffix  = Out-Suffix $triple
  $final   = Join-Path $OutDir "$name-$triple$suffix"

  # Fast cache-hit path requires a pinned hash; otherwise re-extract.
  if (-not $Force -and (Test-Path $final)) {
    $existing = SHA256-OfFile $final
    if ([string]::IsNullOrEmpty($akind)) {
      if ($existing -eq $sha) {
        Write-Host "skip  $final (sha256 match)" -ForegroundColor DarkGray
        return
      }
    } elseif (-not [string]::IsNullOrEmpty($extractedSha) -and $existing -eq $extractedSha) {
      Write-Host "skip  $final (extracted sha256 match)" -ForegroundColor DarkGray
      return
    }
  }

  $downloadName = if ([string]::IsNullOrEmpty($akind)) { "$name-$triple-raw$suffix" } else { "$name-$triple.$akind" }
  $cached = Join-Path $Cache "$sha-$downloadName"

  if ($DryRun) {
    Write-Host "plan  $name $triple  <- $url" -ForegroundColor DarkGray
    Write-Host "       sha256=$sha" -ForegroundColor DarkGray
    Write-Host "       cache=$cached  -> $final" -ForegroundColor DarkGray
    return
  }

  $need = -not (Test-Path $cached)
  if (-not $need) {
    $got = SHA256-OfFile $cached
    if ($got -ne $sha) { $need = $true }
  }

  if ($need) {
    Write-Host "==> download $name $triple  <- $url" -ForegroundColor White
    $tmp = "$cached.partial"
    if (Test-Path $tmp) { Remove-Item -Force $tmp }
    try {
      Download-File $url $tmp
    } catch {
      if (Test-Path $tmp) { Remove-Item -Force $tmp }
      throw "download failed: $url — $($_.Exception.Message)"
    }
    $got = SHA256-OfFile $tmp
    if ($got -ne $sha) {
      Remove-Item -Force $tmp
      throw "sha256 mismatch for $url: expected $sha, got $got"
    }
    Move-Item -Force -Path $tmp -Destination $cached
    Write-Host "ok verified $name $triple (sha256=$sha)" -ForegroundColor Green
  } else {
    Write-Host "cache hit $name $triple" -ForegroundColor DarkGray
  }

  if ([string]::IsNullOrEmpty($akind)) {
    Copy-Item -Force -Path $cached -Destination $final
  } else {
    Extract-Entry $cached $akind $aentry $final
  }
  Write-Host "ok installed $final" -ForegroundColor Green

  if (-not [string]::IsNullOrEmpty($akind) -and [string]::IsNullOrEmpty($extractedSha)) {
    $gotExtracted = SHA256-OfFile $final
    Write-Host ("   extracted_sha256={0} (paste into scripts/sidecars.lock to pin)" -f $gotExtracted) -ForegroundColor DarkGray
  }
}

if (-not (Test-Path $Lockfile)) { throw "$Lockfile not found" }

$rows = @()
foreach ($line in Get-Content $Lockfile) {
  $trim = $line.Trim()
  if ([string]::IsNullOrEmpty($trim) -or $trim.StartsWith('#')) { continue }
  $parts = $line.Split('|')
  if ($parts.Count -lt 7) { continue }
  $row = [pscustomobject]@{
    name           = $parts[0]
    triple         = $parts[1]
    url            = $parts[2]
    sha            = $parts[3]
    binary         = $parts[4]
    archive_kind   = $parts[5]
    archive_entry  = $parts[6]
    extracted_sha  = if ($parts.Count -gt 7) { $parts[7] } else { "" }
  }
  if ($All -or $row.triple -eq $Triple) { $rows += $row }
}

if ($rows.Count -eq 0) {
  if ($All) { throw "no rows parsed from $Lockfile" }
  else     { throw "no lockfile entry for triple: $Triple" }
}

foreach ($r in $rows) { Process-Row $r }

Write-Host ""
Write-Host ("OK bundle-sidecars installed {0} binaries" -f $rows.Count) -ForegroundColor Green
