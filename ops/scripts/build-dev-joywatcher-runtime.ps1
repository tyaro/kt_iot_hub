# Build JoyWatcher runtime driver EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-CargoBuildManifest -RepoRoot $repoRoot -ManifestPath "drivers/joywatcher/driver/Cargo.toml" -Label "JoyWatcher runtime driver"

$sourcePath = Join-Path $repoRoot "target\debug\driver-joywatcher.exe"
Assert-BuildArtifactExists -Path $sourcePath

Write-Host ">>> build succeeded"
Write-Host ">>> installing beside JoyWatcher registration UI..."
Install-DriverRuntimeBuildArtifact -ScriptDir $scriptDir -DriverType "joywatcher" -SourcePath $sourcePath -StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/driver-joywatcher.exe installed and staged"
