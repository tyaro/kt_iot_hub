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
$releaseSteps = @(
  @{
    BuildSource = "manifest"
    ManifestPath = "drivers/postgres/ui/Cargo.toml"
    Label = "PostgreSQL driver-ui"
    ArtifactPath = (Join-Path $repoRoot "target\release\driver_ui_postgres.exe")
    DriverType = "postgres"
    ArtifactKind = "ui"
  },
  @{
    BuildSource = "manifest"
    ManifestPath = "drivers/postgres/driver/Cargo.toml"
    Label = "PostgreSQL runtime driver"
    ArtifactPath = (Join-Path $repoRoot "target\release\driver-postgres.exe")
    DriverType = "postgres"
    ArtifactKind = "runtime"
  },
  @{
    BuildSource = "manifest"
    ManifestPath = "drivers/joywatcher/ui/Cargo.toml"
    Label = "JoyWatcher driver-ui"
    ArtifactPath = (Join-Path $repoRoot "target\release\driver_ui_joywatcher.exe")
    DriverType = "joywatcher"
    ArtifactKind = "ui"
  },
  @{
    BuildSource = "manifest"
    ManifestPath = "drivers/joywatcher/driver/Cargo.toml"
    Label = "JoyWatcher runtime driver"
    ArtifactPath = (Join-Path $repoRoot "target\release\driver-joywatcher.exe")
    DriverType = "joywatcher"
    ArtifactKind = "runtime"
  },
  @{
    BuildSource = "target"
    PackageName = "joywatcher-bridge-x86"
    Target = $targetTriple
    Label = "JoyWatcher x86 bridge"
    ArtifactPath = (Join-Path $repoRoot "target\$targetTriple\release\joywatcher-bridge-x86.exe")
    DriverType = "joywatcher"
    TargetFileName = "joywatcher-bridge-x86.exe"
  }
)

Write-Host ">>> building and installing release artifacts into ops/driver-ui/..."
foreach ($releaseStep in $releaseSteps) {
  if ($releaseStep.BuildSource -eq "manifest") {
    Invoke-DriverManifestBuildInstall -RepoRoot $repoRoot -ScriptDir $scriptDir -ManifestPath $releaseStep.ManifestPath -Label $releaseStep.Label -ArtifactPath $releaseStep.ArtifactPath -DriverType $releaseStep.DriverType -ArtifactKind $releaseStep.ArtifactKind -BuildKind "release"
    continue
  }

  Invoke-DriverPackageTargetBuildInstall -RepoRoot $repoRoot -ScriptDir $scriptDir -PackageName $releaseStep.PackageName -Target $releaseStep.Target -Label $releaseStep.Label -ArtifactPath $releaseStep.ArtifactPath -DriverType $releaseStep.DriverType -TargetFileName $releaseStep.TargetFileName -BuildKind "release"
}

Write-Host ">>> staging from ops/driver-ui/ (source of truth) to core/src-tauri/driver-ui/..."
& (Join-Path $scriptDir "stage-driver-suite.ps1")

Write-Host ">>> done: release artifacts installed to ops/driver-ui/ and staged to core/src-tauri/driver-ui/ (one-way sync)" -ForegroundColor Green
