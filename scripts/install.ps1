#!/usr/bin/env pwsh
#
# Windows setup wizard (installer + updater) for the Olympus cycling app.
#
# What it does:
#   1. Detects the CPU architecture and maps it to a release artifact.
#   2. Resolves the release to install (latest from GitHub by default,
#      override with -Version or $env:OLYMPUS_VERSION).
#   3. Downloads the artifact, verifies its SHA-256 when one is published.
#   4. Installs the binary and an olympus.cmd launcher.
#   5. Seeds the data home (data/workouts + a first-run profile.json).
#   6. Optionally adds the install folder to your user PATH (-AddToPath).
#   7. Prints the Windows Bluetooth setup note.
#
# Re-run this script any time to update to the latest release.
#
# Artifact convention (produced by scripts/release.* on the release host):
#   <release>/download/<tag>/olympus-<tag>-windows-<arch>.zip
#   <release>/download/<tag>/olympus-<tag>-windows-<arch>.zip.sha256
#   arch: x86_64 | arm64
#   Archive layout: olympus.exe + data\workouts\*.zwo

[CmdletBinding()]
param(
  [string]$Version = $env:OLYMPUS_VERSION,
  [string]$BaseUrl = $env:OLYMPUS_BASE_URL,
  [string]$InstallDir = "$env:LOCALAPPDATA\Olympus",
  [switch]$AddToPath,
  [switch]$RequireChecksum
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$Msg) Write-Host $Msg }

if ([string]::IsNullOrEmpty($BaseUrl)) {
  $BaseUrl = "https://github.com/draconis-engineering/olympus/releases"
}

# --- Detect architecture ----------------------------------------------------
$native = $env:PROCESSOR_ARCHITEW6432
if ([string]::IsNullOrEmpty($native)) { $native = $env:PROCESSOR_ARCHITECTURE }
$arch = switch ($native) {
  "AMD64" { "x86_64" }
  "ARM64" { "arm64" }
  default { $native }
}

# --- Resolve the release to install ----------------------------------------
if ([string]::IsNullOrEmpty($Version)) {
  $api = "https://api.github.com/repos/draconis-engineering/olympus/releases/latest"
  $json = Invoke-RestMethod -Headers @{ "User-Agent" = "olympus-installer" } -Uri $api
  $Version = $json.tag_name
  Write-Step "Resolved latest release: $Version"
} else {
  Write-Step "Installing release: $Version"
}

$artifact = "olympus-$Version-windows-$arch.zip"
$url = "$BaseUrl/download/$Version/$artifact"
$sumUrl = "$url.sha256"

# --- Download + verify ------------------------------------------------------
$tmp = Join-Path $env:TEMP ("olympus-install-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Step "Fetching $url"
  $zip = Join-Path $tmp $artifact
  if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
    & curl.exe -fSL -o $zip $url
    if ($LASTEXITCODE -ne 0) { throw "download failed; check the release exists (got $url)" }
  } else {
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $zip
  }

  $expectedHash = $null
  if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
    $sumOut = & curl.exe -fsSL $sumUrl 2>$null
    if ($LASTEXITCODE -eq 0 -and $sumOut) { $expectedHash = (($sumOut -join " ") -split "\s+")[0].Trim() }
  } else {
    try { $expectedHash = ((Invoke-WebRequest -UseBasicParsing -Uri $sumUrl).Content -split "\s+")[0].Trim() } catch { }
  }
  if ($expectedHash) {
    $actualHash = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLower()
    if ($expectedHash.ToLower() -ne $actualHash) { throw "checksum mismatch (expected $expectedHash, got $actualHash)" }
    Write-Step "Checksum OK ($expectedHash)"
  } elseif ($RequireChecksum) {
    throw "no checksum published for ${artifact}; refusing (re-run without -RequireChecksum to allow)"
  } else {
    Write-Step "Warning: no checksum published for this artifact; skipping verification"
  }

  # --- Unpack ---------------------------------------------------------------
  Write-Step "Unpacking $artifact"
  $unpack = Join-Path $tmp "unpacked"
  Expand-Archive -Path $zip -DestinationPath $unpack
  $binSrc = Get-ChildItem -Path $unpack -Recurse -Filter "olympus.exe" -File | Select-Object -First 1
  if (-not $binSrc) { throw "release artifact contains no olympus.exe" }

  # --- Install binary + launcher --------------------------------------------
  $binDir = Join-Path $InstallDir "bin"
  New-Item -ItemType Directory -Path $binDir -Force | Out-Null
  Copy-Item -Path $binSrc.FullName -Destination (Join-Path $binDir "olympus.exe") -Force
  Write-Step "Installed $binDir\olympus.exe"

  $launcher = Join-Path $InstallDir "olympus.cmd"
  "@echo off
cd /d `"$InstallDir`"
`"$binDir\olympus.exe`" %*" | Set-Content -Path $launcher -Encoding Ascii
  Write-Step "Installed launcher $launcher"

  # --- Seed the data home ---------------------------------------------------
  $workoutsDir = Join-Path $InstallDir "data\workouts"
  New-Item -ItemType Directory -Path $workoutsDir -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $InstallDir "data\.fit") -Force | Out-Null
  New-Item -ItemType Directory -Path (Join-Path $InstallDir "data\user") -Force | Out-Null
  Get-ChildItem -Path $unpack -Recurse -Filter "*.zwo" -File | ForEach-Object {
    $dest = Join-Path $workoutsDir $_.Name
    if (-not (Test-Path -LiteralPath $dest)) { Copy-Item -Path $_.FullName -Destination $dest }
  }
  $bundledWorkouts = (Get-ChildItem -Path $workoutsDir -Filter "*.zwo" -File).Count
  Write-Step "Seeded $workoutsDir with $bundledWorkouts bundled workouts"

  $profile = Join-Path $InstallDir "data\user\profile.json"
  if (-not (Test-Path -LiteralPath $profile)) {
@'
{
  "username": "Rider",
  "weight": 75.0,
  "height": 180.0,
  "ftp": 200,
  "max_hr": 180
}
'@ | Set-Content -Path $profile -Encoding UTF8
    Write-Step "Wrote first-run profile: $profile"
  }

  # --- Optional PATH entry --------------------------------------------------
  if ($AddToPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
      $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { $userPath + ";" + $InstallDir }
      [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
      Write-Step "Added $InstallDir to your user PATH (open a new terminal to use it)"
    }
  }

  Write-Step ""
  Write-Step "Done. Run `"$launcher`" (or `"olympus`" once on PATH) -- ride data lives in $InstallDir\data."
  Write-Step "To update later, just re-run this script (it always installs the latest release)."
  Write-Step ""
  Write-Step "Windows Bluetooth: no extra setup -- grant Bluetooth permission when Windows first asks."
}
finally {
  Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}