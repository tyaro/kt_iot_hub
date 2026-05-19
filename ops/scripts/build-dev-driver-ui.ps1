# Build postgres driver-ui EXE (debug) and install it under ops/driver-ui/postgres/
# Usage: .\ops\scripts\build-dev-driver-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-DriverManifestBuildInstall `
	-RepoRoot $repoRoot `
	-ScriptDir $scriptDir `
	-ManifestPath "drivers/postgres/ui/Cargo.toml" `
	-Label "postgres driver-ui" `
	-ArtifactPath (Join-Path $repoRoot "target\debug\driver_ui_postgres.exe") `
	-DriverType "postgres" `
	-ArtifactKind "ui" `
	-StageToBundle

Write-Host ">>> done: ops/driver-ui/postgres/registration-ui.exe installed and staged"
