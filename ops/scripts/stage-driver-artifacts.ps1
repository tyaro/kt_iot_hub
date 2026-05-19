# Stages driver artifacts from driver-ui/<DriverType>/ (source of truth) to src-tauri/driver-ui/<DriverType>/.
# This script does not build binaries and does not copy in reverse direction.
# Usage: .\scripts\stage-driver-artifacts.ps1 -DriverType postgres

param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$sourceDir = Join-Path $repoRoot "driver-ui\$DriverType"
$targetDir = Join-Path $repoRoot "src-tauri\driver-ui\$DriverType"

if (-not (Test-Path $sourceDir -PathType Container)) {
  throw "Source driver directory not found: $sourceDir"
}

if (-not (Test-Path $targetDir)) {
  New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

Write-Host ">>> staging driver artifacts"
Write-Host "  Source : $sourceDir"
Write-Host "  Target : $targetDir"

Copy-Item -Path (Join-Path $sourceDir "*") -Destination $targetDir -Recurse -Force

Write-Host "Staged successfully (one-way): driver-ui/$DriverType -> src-tauri/driver-ui/$DriverType" -ForegroundColor Green
