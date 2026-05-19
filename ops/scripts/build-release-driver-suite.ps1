# Builds release artifacts under target/, installs them into ops/driver-ui/ (development source of truth),
# then stages from ops/driver-ui/ to core/src-tauri/driver-ui/ for installer bundling.
# Usage: .\ops\scripts\build-release-driver-suite.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")
$targetTriple = "i686-pc-windows-msvc"
$bundleRoot = Join-Path $repoRoot "core\src-tauri"

function Invoke-CargoBuildRelease([string]$manifestPath, [string]$label) {
  Write-Host ">>> cargo build $label (release)..."
  Push-Location $repoRoot
  try {
    cargo build --release --manifest-path $manifestPath
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE): $label" }
  }
  finally {
    Pop-Location
  }
}

function Invoke-CargoBuildReleaseTarget([string]$packageName, [string]$target, [string]$label) {
  Write-Host ">>> cargo build $label (release / $target)..."
  Push-Location $repoRoot
  try {
    cargo build --release -p $packageName --target $target
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit code $LASTEXITCODE): $label" }
  }
  finally {
    Pop-Location
  }
}

Invoke-CargoBuildRelease "drivers/postgres/ui/Cargo.toml" "PostgreSQL driver-ui"
Invoke-CargoBuildRelease "drivers/postgres/driver/Cargo.toml" "PostgreSQL runtime driver"
Invoke-CargoBuildRelease "drivers/joywatcher/ui/Cargo.toml" "JoyWatcher driver-ui"
Invoke-CargoBuildRelease "drivers/joywatcher/driver/Cargo.toml" "JoyWatcher runtime driver"
Invoke-CargoBuildReleaseTarget "joywatcher-bridge-x86" $targetTriple "JoyWatcher x86 bridge"

$postgresUiExe = Join-Path $repoRoot "target\release\driver_ui_postgres.exe"
$postgresDriverExe = Join-Path $repoRoot "target\release\driver-postgres.exe"
$joywatcherUiExe = Join-Path $repoRoot "target\release\driver_ui_joywatcher.exe"
$joywatcherDriverExe = Join-Path $repoRoot "target\release\driver-joywatcher.exe"
$joywatcherBridgeExe = Join-Path $repoRoot "target\$targetTriple\release\joywatcher-bridge-x86.exe"

if (-not (Test-Path $postgresUiExe)) { throw "Build artifact not found: $postgresUiExe" }
if (-not (Test-Path $postgresDriverExe)) { throw "Build artifact not found: $postgresDriverExe" }
if (-not (Test-Path $joywatcherUiExe)) { throw "Build artifact not found: $joywatcherUiExe" }
if (-not (Test-Path $joywatcherDriverExe)) { throw "Build artifact not found: $joywatcherDriverExe" }
if (-not (Test-Path $joywatcherBridgeExe)) { throw "Build artifact not found: $joywatcherBridgeExe" }

Write-Host ">>> installing release artifacts into ops/driver-ui/..."
& (Join-Path $scriptDir "install-driver-ui.ps1") -DriverType "postgres" -SourcePath $postgresUiExe
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "postgres" -SourcePath $postgresDriverExe

& (Join-Path $scriptDir "install-driver-ui.ps1") -DriverType "joywatcher" -SourcePath $joywatcherUiExe
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "joywatcher" -SourcePath $joywatcherDriverExe
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "joywatcher" -SourcePath $joywatcherBridgeExe -TargetFileName "joywatcher-bridge-x86.exe"

$postgresPrimaryUiExe = Join-Path $repoRoot "ops\driver-ui\postgres\registration-ui.exe"
$postgresPrimaryDriverExe = Join-Path $repoRoot "ops\driver-ui\postgres\driver-postgres.exe"
$joywatcherPrimaryUiExe = Join-Path $repoRoot "ops\driver-ui\joywatcher\registration-ui.exe"
$joywatcherPrimaryDriverExe = Join-Path $repoRoot "ops\driver-ui\joywatcher\driver-joywatcher.exe"
$joywatcherPrimaryBridgeExe = Join-Path $repoRoot "ops\driver-ui\joywatcher\joywatcher-bridge-x86.exe"

Write-Host ">>> staging from ops/driver-ui/ (source of truth) to core/src-tauri/driver-ui/..."
& (Join-Path $scriptDir "install-driver-ui.ps1") -DriverType "postgres" -SourcePath $postgresPrimaryUiExe -AppRoot $bundleRoot
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "postgres" -SourcePath $postgresPrimaryDriverExe -BinDir (Join-Path $bundleRoot "driver-ui\postgres")
& (Join-Path $scriptDir "install-driver-ui.ps1") -DriverType "joywatcher" -SourcePath $joywatcherPrimaryUiExe -AppRoot $bundleRoot
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "joywatcher" -SourcePath $joywatcherPrimaryDriverExe -BinDir (Join-Path $bundleRoot "driver-ui\joywatcher")
& (Join-Path $scriptDir "install-driver-runtime.ps1") -DriverType "joywatcher" -SourcePath $joywatcherPrimaryBridgeExe -TargetFileName "joywatcher-bridge-x86.exe" -BinDir (Join-Path $bundleRoot "driver-ui\joywatcher")

Write-Host ">>> done: release artifacts installed to ops/driver-ui/ and staged to core/src-tauri/driver-ui/ (one-way sync)" -ForegroundColor Green
