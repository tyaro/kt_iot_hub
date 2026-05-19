# Build and install JoyWatcher UI/runtime/bridge artifacts under ops/driver-ui/joywatcher/
# and keep core/src-tauri/driver-ui/joywatcher/ in sync.
# Usage: .\ops\scripts\build-dev-joywatcher-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath

& (Join-Path $scriptDir "build-dev-joywatcher-ui.ps1")
& (Join-Path $scriptDir "build-dev-joywatcher-runtime.ps1")
& (Join-Path $scriptDir "build-dev-joywatcher-bridge-x86.ps1")

Write-Host ">>> done: JoyWatcher artifacts installed under ops/driver-ui/joywatcher/ and staged"
