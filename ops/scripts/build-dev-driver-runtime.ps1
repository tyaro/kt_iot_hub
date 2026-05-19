# Build postgres runtime driver EXE (debug)
# Usage: .\scripts\build-dev-driver-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-DriverManifestBuildInstall `
	-RepoRoot $repoRoot `
	-ScriptDir $scriptDir `
	-ManifestPath "drivers/postgres/driver/Cargo.toml" `
	-Label "postgres runtime driver" `
	-ArtifactPath (Join-Path $repoRoot "target\debug\driver-postgres.exe") `
	-DriverType "postgres" `
	-ArtifactKind "runtime" `
	-StageToBundle

Write-Host ">>> done: ops/driver-ui/postgres/driver-postgres.exe installed and staged"
