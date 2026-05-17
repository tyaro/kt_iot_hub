param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType,

  [Parameter(Mandatory = $true)]
  [string]$SourcePath,

  [string]$BinDir,

  [string]$TargetFileName
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

if ([string]::IsNullOrWhiteSpace($BinDir)) {
  $BinDir = Join-Path $repoRoot.Path (Join-Path "driver-ui" $DriverType)
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
