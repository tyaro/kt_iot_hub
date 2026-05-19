# Build JoyWatcher x86 bridge EXE (debug) and install it under ops/driver-ui/joywatcher/
# Usage: .\ops\scripts\build-dev-joywatcher-bridge-x86.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot
$targetTriple = "i686-pc-windows-msvc"

Invoke-DriverPackageTargetBuildInstall `
	-RepoRoot $repoRoot `
	-ScriptDir $scriptDir `
	-PackageName "joywatcher-bridge-x86" `
	-Target $targetTriple `
	-Label "JoyWatcher x86 bridge" `
	-ArtifactPath (Join-Path $repoRoot "target\$targetTriple\debug\joywatcher-bridge-x86.exe") `
	-DriverType "joywatcher" `
	-TargetFileName "joywatcher-bridge-x86.exe" `
	-StageToBundle

Write-Host ">>> done: ops/driver-ui/joywatcher/joywatcher-bridge-x86.exe installed and staged"