# Build postgres driver-ui EXE (debug) and install it under ops/driver-ui/postgres/
# Usage: .\ops\scripts\build-dev-driver-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

Write-Host ">>> cargo build postgres driver-ui (debug)..."
Push-Location $repoRoot
try {
    cargo build --manifest-path drivers/postgres/ui/Cargo.toml
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
    -SourcePath $sourcePath `
    -StageToBundle

Write-Host ">>> done: ops/driver-ui/postgres/registration-ui.exe installed and staged"
