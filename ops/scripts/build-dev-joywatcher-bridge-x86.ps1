# Build JoyWatcher x86 bridge EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-bridge-x86.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")
$targetTriple = "i686-pc-windows-msvc"

Write-Host ">>> cargo build joywatcher x86 bridge (debug / $targetTriple)..."
Push-Location $repoRoot
try {
	cargo build -p joywatcher-bridge-x86 --target $targetTriple
	if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE)" }
} finally {
	Pop-Location
}

$sourcePath = Join-Path $repoRoot "target\$targetTriple\debug\joywatcher-bridge-x86.exe"
if (-not (Test-Path $sourcePath)) {
	throw "Build artifact not found: $sourcePath"
}

Write-Host ">>> build succeeded"

Write-Host ">>> installing beside JoyWatcher runtime..."
& (Join-Path $scriptDir "install-driver-runtime.ps1") `
	-DriverType "joywatcher" `
	-SourcePath $sourcePath `
	-TargetFileName "joywatcher-bridge-x86.exe" `
	-StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/joywatcher-bridge-x86.exe installed and staged"