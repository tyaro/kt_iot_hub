# Build postgres runtime driver EXE (debug)
# Usage: .\scripts\build-dev-driver-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-CargoBuildManifest -RepoRoot $repoRoot -ManifestPath "drivers/postgres/driver/Cargo.toml" -Label "postgres runtime driver"

$sourcePath = Join-Path $repoRoot "target\debug\driver-postgres.exe"
Assert-BuildArtifactExists -Path $sourcePath

Write-Host ">>> build succeeded"

Write-Host ">>> installing beside registration UI..."
Install-DriverRuntimeBuildArtifact -ScriptDir $scriptDir -DriverType "postgres" -SourcePath $sourcePath -StageToBundle

Write-Host ">>> done: ops/driver-ui/postgres/driver-postgres.exe installed and staged"
