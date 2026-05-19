# Installs a runtime driver executable.
#
# Default: Copies to ops/driver-ui/<driver-type>/ (development source of truth, see .deploymentinfo)
# When -BinDir specified: Copies to <BinDir>/<driver-type>/ (custom location, legacy)
#
# Recommended: Use npm run driver-runtime:install or stage-driver-artifacts.ps1 for proper flow.
#
# Usage: .\ops\scripts\install-driver-runtime.ps1 -DriverType postgres -SourcePath C:\build\driver-postgres.exe

param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType,

  [Parameter(Mandatory = $true)]
  [string]$SourcePath,

  [string]$BinDir,

  [string]$TargetFileName,

  [switch]$StageToBundle
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

if ($StageToBundle -and -not [string]::IsNullOrWhiteSpace($BinDir)) {
  throw "-StageToBundle can only be used when BinDir is not specified. Install into ops/driver-ui first, then stage to bundle."
}

if ([string]::IsNullOrWhiteSpace($BinDir)) {
  $BinDir = Join-Path $repoRoot.Path (Join-Path "ops/driver-ui" $DriverType)
}

if ([string]::IsNullOrWhiteSpace($TargetFileName)) {
  $TargetFileName = "driver-$DriverType.exe"
}

$resolvedSource = Resolve-Path $SourcePath
if (-not (Test-Path $resolvedSource -PathType Leaf)) {
  throw "SourcePath is not a file: $SourcePath"
}

if (-not (Test-Path $BinDir)) {
  New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}

$targetPath = Join-Path $BinDir $TargetFileName
Copy-Item -Path $resolvedSource -Destination $targetPath -Force

Write-Host "Driver runtime installed:" -ForegroundColor Green
Write-Host "  DriverType : $DriverType"
Write-Host "  Source     : $resolvedSource"
Write-Host "  Target     : $targetPath"
Write-Host ""
Write-Host "Search order: configured driver-ui base path -> DRIVER_BIN_DIR -> main app directory -> PATH" -ForegroundColor Cyan

if ($StageToBundle) {
  Write-Host ">>> staging driver artifacts to core/src-tauri/driver-ui/..."
  & (Join-Path $scriptDir "stage-driver-artifacts.ps1") -DriverType $DriverType
}
