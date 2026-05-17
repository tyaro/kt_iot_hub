# Build postgres runtime driver EXE (debug)
# Usage: .\scripts\build-dev-driver-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

Write-Host ">>> cargo build postgres runtime driver (debug)..."
Push-Location $repoRoot
try {
	cargo build --manifest-path apps/postgres/driver/Cargo.toml
	if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
} finally {
	Pop-Location
}

$sourcePath = Join-Path $repoRoot "target\debug\driver-postgres.exe"
if (-not (Test-Path $sourcePath)) {
	throw "Build artifact not found: $sourcePath"
}

Write-Host ">>> build succeeded"

Write-Host ">>> installing beside registration UI..."
& (Join-Path $scriptDir "install-driver-runtime.ps1") `
	-DriverType "postgres" `
	-SourcePath $sourcePath

Write-Host ">>> done: driver-ui/postgres/driver-postgres.exe installed"
