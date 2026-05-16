# Build postgres driver-ui EXE (debug) and install it under driver-ui/postgres/
# Usage: .\scripts\build-dev-driver-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

Write-Host ">>> cargo build driver-ui-postgres (debug)..."
Push-Location $repoRoot
try {
    cargo build --manifest-path apps/driver-ui-postgres/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
} finally {
    Pop-Location
}
Write-Host ">>> build succeeded"

$sourcePath = Join-Path $repoRoot "target\debug\driver_ui_postgres.exe"
if (-not (Test-Path $sourcePath)) {
    throw "Build artifact not found: $sourcePath"
}

Write-Host ">>> installing as postgres driver-ui..."
& (Join-Path $scriptDir "install-driver-ui.ps1") `
    -DriverType "postgres" `
    -SourcePath $sourcePath

Write-Host ">>> done: driver-ui/postgres/registration-ui.exe installed"
