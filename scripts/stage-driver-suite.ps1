# Stages all known driver artifacts from driver-ui/ to src-tauri/driver-ui/.
# This script only performs one-way sync for bundle staging.
# Usage: .\scripts\stage-driver-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath

$driverTypes = @(
  "postgres",
  "joywatcher"
)

foreach ($driverType in $driverTypes) {
  & (Join-Path $scriptDir "stage-driver-artifacts.ps1") -DriverType $driverType
}

Write-Host ">>> done: staged all driver artifacts to src-tauri/driver-ui/" -ForegroundColor Green
