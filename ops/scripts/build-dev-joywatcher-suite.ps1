# Build and install JoyWatcher UI/runtime/bridge artifacts under driver-ui/joywatcher/
# Usage: .\scripts\build-dev-joywatcher-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath

& (Join-Path $scriptDir "build-dev-joywatcher-ui.ps1")
& (Join-Path $scriptDir "build-dev-joywatcher-runtime.ps1")
& (Join-Path $scriptDir "build-dev-joywatcher-bridge-x86.ps1")

Write-Host ">>> done: JoyWatcher artifacts installed under driver-ui/joywatcher/"
