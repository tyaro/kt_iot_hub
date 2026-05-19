# Build JoyWatcher runtime driver EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-runtime.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot

Invoke-DriverManifestBuildInstall `
	-RepoRoot $repoRoot `
	-ScriptDir $scriptDir `
	-ManifestPath "drivers/joywatcher/driver/Cargo.toml" `
	-Label "JoyWatcher runtime driver" `
	-ArtifactPath (Join-Path $repoRoot "target\debug\driver-joywatcher.exe") `
	-DriverType "joywatcher" `
	-ArtifactKind "runtime" `
	-StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/driver-joywatcher.exe installed and staged"
