# Installs a registration UI executable.
# Default destination: <repo-root>/ops/driver-ui/<driver-type>/ (development source of truth).
# When -AppRoot is specified, destination: <AppRoot>/driver-ui/<driver-type>/ (bundle staging).

param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType,

  [Parameter(Mandatory = $true)]
  [string]$SourcePath,

  [string]$AppRoot,

  [string]$TargetFileName = "registration-ui.exe",

  [switch]$StageToBundle
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

if ($StageToBundle -and -not [string]::IsNullOrWhiteSpace($AppRoot)) {
  throw "-StageToBundle can only be used when AppRoot is not specified. Install into ops/driver-ui first, then stage to bundle."
}

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $targetDir = Join-Path $repoRoot.Path (Join-Path "ops/driver-ui" $DriverType)
} else {
  $targetDir = Join-Path $AppRoot (Join-Path "driver-ui" $DriverType)
}

$resolvedSource = Resolve-Path $SourcePath
if (-not (Test-Path $resolvedSource -PathType Leaf)) {
  throw "SourcePath is not a file: $SourcePath"
}

if (-not (Test-Path $targetDir)) {
  New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$targetPath = Join-Path $targetDir $TargetFileName
Copy-Item -Path $resolvedSource -Destination $targetPath -Force

Write-Host "Driver UI installed:" -ForegroundColor Green
Write-Host "  DriverType : $DriverType"
Write-Host "  Source     : $resolvedSource"
Write-Host "  Target     : $targetPath"
Write-Host ""
Write-Host "Search path: <base>/driver-ui/$DriverType/registration-ui(.exe)" -ForegroundColor Cyan

if ($StageToBundle) {
  Write-Host ">>> staging driver artifacts to core/src-tauri/driver-ui/..."
  & (Join-Path $scriptDir "stage-driver-artifacts.ps1") -DriverType $DriverType
}
