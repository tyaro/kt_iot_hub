# Stages all manifest-backed driver artifacts from ops/driver-ui/ to core/src-tauri/driver-ui/.
# This script only performs one-way sync for bundle staging.
# Usage: .\ops\scripts\stage-driver-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")
$sourceRoot = Join-Path $repoRoot "ops\driver-ui"

if (-not (Test-Path $sourceRoot -PathType Container)) {
  throw "Driver source root not found: $sourceRoot"
}

$driverTypes = Get-ChildItem -Path $sourceRoot -Directory |
  Where-Object { Test-Path (Join-Path $_.FullName "driver-manifest.json") -PathType Leaf } |
  Sort-Object -Property Name |
  ForEach-Object { $_.Name }

if (-not $driverTypes -or $driverTypes.Count -eq 0) {
  Write-Host ">>> no manifest-backed driver artifacts found under ops/driver-ui/" -ForegroundColor Yellow
  return
}

Write-Host ">>> staging manifest-backed driver artifacts: $($driverTypes -join ', ')"

foreach ($driverType in $driverTypes) {
  & (Join-Path $scriptDir "stage-driver-artifacts.ps1") -DriverType $driverType
}

Write-Host ">>> done: staged all manifest-backed driver artifacts to core/src-tauri/driver-ui/" -ForegroundColor Green
