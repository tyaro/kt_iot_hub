# Build JoyWatcher driver UI EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

Write-Host ">>> cargo build JoyWatcher driver-ui (debug)..."
Push-Location $repoRoot
try {
    cargo build --manifest-path drivers/joywatcher/ui/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
} finally {
    Pop-Location
}

$sourcePath = Join-Path $repoRoot "target\debug\driver_ui_joywatcher.exe"
if (-not (Test-Path $sourcePath)) {
    throw "Build artifact not found: $sourcePath"
}

Write-Host ">>> build succeeded"
Write-Host ">>> installing as JoyWatcher driver-ui..."
& (Join-Path $scriptDir "install-driver-ui.ps1") `
    -DriverType "joywatcher" `
    -SourcePath $sourcePath

Write-Host ">>> done: ops/driver-ui/joywatcher/registration-ui.exe installed"
