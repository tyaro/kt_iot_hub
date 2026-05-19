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
  throw "-StageToBundle は AppRoot 未指定時のみ利用できます。ops/driver-ui を正本に install した後で staging してください。"
}

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $targetDir = Join-Path $repoRoot.Path (Join-Path "ops/driver-ui" $DriverType)
} else {
  $targetDir = Join-Path $AppRoot (Join-Path "driver-ui" $DriverType)
}

$resolvedSource = Resolve-Path $SourcePath
if (-not (Test-Path $resolvedSource -PathType Leaf)) {
  throw "SourcePath がファイルではありません: $SourcePath"
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
