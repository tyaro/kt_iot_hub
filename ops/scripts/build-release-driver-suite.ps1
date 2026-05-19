# Builds release artifacts under target/, installs them into ops/driver-ui/ (development source of truth),
# then stages from ops/driver-ui/ to core/src-tauri/driver-ui/ for installer bundling.
# Usage: .\ops\scripts\build-release-driver-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "driver-build-helpers.ps1")

$context = Get-DriverBuildContext -ScriptPath $PSCommandPath
$scriptDir = $context.ScriptDir
$repoRoot = $context.RepoRoot
$targetTriple = "i686-pc-windows-msvc"
$buildSteps = @(
  @{ Kind = "manifest"; ManifestPath = "drivers/postgres/ui/Cargo.toml"; Label = "PostgreSQL driver-ui" },
  @{ Kind = "manifest"; ManifestPath = "drivers/postgres/driver/Cargo.toml"; Label = "PostgreSQL runtime driver" },
  @{ Kind = "manifest"; ManifestPath = "drivers/joywatcher/ui/Cargo.toml"; Label = "JoyWatcher driver-ui" },
  @{ Kind = "manifest"; ManifestPath = "drivers/joywatcher/driver/Cargo.toml"; Label = "JoyWatcher runtime driver" },
  @{ Kind = "target"; PackageName = "joywatcher-bridge-x86"; Target = $targetTriple; Label = "JoyWatcher x86 bridge" }
)

foreach ($buildStep in $buildSteps) {
  if ($buildStep.Kind -eq "manifest") {
    Invoke-CargoBuildManifest -RepoRoot $repoRoot -ManifestPath $buildStep.ManifestPath -Label $buildStep.Label -BuildKind "release"
    continue
  }

  Invoke-CargoBuildPackageTarget -RepoRoot $repoRoot -PackageName $buildStep.PackageName -Target $buildStep.Target -Label $buildStep.Label -BuildKind "release"
}

$installSteps = @(
  @{ Kind = "ui"; DriverType = "postgres"; SourcePath = (Join-Path $repoRoot "target\release\driver_ui_postgres.exe") },
  @{ Kind = "runtime"; DriverType = "postgres"; SourcePath = (Join-Path $repoRoot "target\release\driver-postgres.exe") },
  @{ Kind = "ui"; DriverType = "joywatcher"; SourcePath = (Join-Path $repoRoot "target\release\driver_ui_joywatcher.exe") },
  @{ Kind = "runtime"; DriverType = "joywatcher"; SourcePath = (Join-Path $repoRoot "target\release\driver-joywatcher.exe") },
  @{ Kind = "runtime"; DriverType = "joywatcher"; SourcePath = (Join-Path $repoRoot "target\$targetTriple\release\joywatcher-bridge-x86.exe"); TargetFileName = "joywatcher-bridge-x86.exe" }
)

foreach ($installStep in $installSteps) {
  Assert-BuildArtifactExists -Path $installStep.SourcePath
}

Write-Host ">>> installing release artifacts into ops/driver-ui/..."
foreach ($installStep in $installSteps) {
  if ($installStep.Kind -eq "ui") {
    Install-DriverUiBuildArtifact -ScriptDir $scriptDir -DriverType $installStep.DriverType -SourcePath $installStep.SourcePath
    continue
  }

  $targetFileName = $null
  if ($installStep.ContainsKey("TargetFileName")) {
    $targetFileName = $installStep.TargetFileName
  }

  Install-DriverRuntimeBuildArtifact -ScriptDir $scriptDir -DriverType $installStep.DriverType -SourcePath $installStep.SourcePath -TargetFileName $targetFileName
}

Write-Host ">>> staging from ops/driver-ui/ (source of truth) to core/src-tauri/driver-ui/..."
& (Join-Path $scriptDir "stage-driver-suite.ps1")

Write-Host ">>> done: release artifacts installed to ops/driver-ui/ and staged to core/src-tauri/driver-ui/ (one-way sync)" -ForegroundColor Green
