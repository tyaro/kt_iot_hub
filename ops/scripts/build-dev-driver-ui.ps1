# Build postgres driver-ui EXE (debug) and install it under ops/driver-ui/postgres/
# Usage: .\ops\scripts\build-dev-driver-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-CargoBuildManifest -RepoRoot $repoRoot -ManifestPath "drivers/postgres/ui/Cargo.toml" -Label "postgres driver-ui"
Write-Host ">>> build succeeded"

$sourcePath = Join-Path $repoRoot "target\debug\driver_ui_postgres.exe"
Assert-BuildArtifactExists -Path $sourcePath

Write-Host ">>> installing as postgres driver-ui..."
Install-DriverUiBuildArtifact -ScriptDir $scriptDir -DriverType "postgres" -SourcePath $sourcePath -StageToBundle

Write-Host ">>> done: ops/driver-ui/postgres/registration-ui.exe installed and staged"
