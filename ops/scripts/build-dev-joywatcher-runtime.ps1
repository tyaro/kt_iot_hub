# Build JoyWatcher runtime driver EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

Write-Host ">>> cargo build JoyWatcher runtime driver (debug)..."
Push-Location $repoRoot
try {
    cargo build --manifest-path drivers/joywatcher/driver/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
} finally {
    Pop-Location
}

$sourcePath = Join-Path $repoRoot "target\debug\driver-joywatcher.exe"
if (-not (Test-Path $sourcePath)) {
    throw "Build artifact not found: $sourcePath"
}

Write-Host ">>> build succeeded"
Write-Host ">>> installing beside JoyWatcher registration UI..."
& (Join-Path $scriptDir "install-driver-runtime.ps1") `
    -DriverType "joywatcher" `
    -SourcePath $sourcePath `
    -StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/driver-joywatcher.exe installed and staged"
