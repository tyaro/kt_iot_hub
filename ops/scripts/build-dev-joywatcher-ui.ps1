# Build JoyWatcher driver UI EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-ui.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-CargoBuildManifest -RepoRoot $repoRoot -ManifestPath "drivers/joywatcher/ui/Cargo.toml" -Label "JoyWatcher driver-ui"

$sourcePath = Join-Path $repoRoot "target\debug\driver_ui_joywatcher.exe"
Assert-BuildArtifactExists -Path $sourcePath

Write-Host ">>> build succeeded"
Write-Host ">>> installing as JoyWatcher driver-ui..."
Install-DriverUiBuildArtifact -ScriptDir $scriptDir -DriverType "joywatcher" -SourcePath $sourcePath -StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/registration-ui.exe installed and staged"
