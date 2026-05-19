# Installs a registration UI executable into <app-root>/driver-ui/<driver-type>/.
# Typical source is target/release/*.exe; destination is development source of truth driver-ui/.

param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType,

  [Parameter(Mandatory = $true)]
  [string]$SourcePath,

  [string]$AppRoot,

  [string]$TargetFileName = "registration-ui.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $AppRoot = $repoRoot.Path
}

$resolvedSource = Resolve-Path $SourcePath
if (-not (Test-Path $resolvedSource -PathType Leaf)) {
  throw "SourcePath がファイルではありません: $SourcePath"
}

$targetDir = Join-Path $AppRoot (Join-Path "driver-ui" $DriverType)
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
Write-Host "Search path: <app-root>/driver-ui/$DriverType/registration-ui(.exe)" -ForegroundColor Cyan
