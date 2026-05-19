# Installs a runtime driver executable into ops/driver-ui/<driver-type>/ (or a specified BinDir).
# Typical source is target/release/*.exe; destination is development source of truth by default.

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
  throw "-StageToBundle は BinDir 未指定時のみ利用できます。ops/driver-ui を正本に install した後で staging してください。"
}

if ([string]::IsNullOrWhiteSpace($BinDir)) {
  $BinDir = Join-Path $repoRoot.Path (Join-Path "ops/driver-ui" $DriverType)
}

if ([string]::IsNullOrWhiteSpace($TargetFileName)) {
  $TargetFileName = "driver-$DriverType.exe"
}

$resolvedSource = Resolve-Path $SourcePath
if (-not (Test-Path $resolvedSource -PathType Leaf)) {
  throw "SourcePath がファイルではありません: $SourcePath"
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
