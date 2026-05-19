# Stages driver artifacts from ops/driver-ui/<DriverType>/ (source of truth) to core/src-tauri/driver-ui/<DriverType/>.
# This script does not build binaries and performs one-way mirror sync for bundle staging.
# Usage: .\ops\scripts\stage-driver-artifacts.ps1 -DriverType postgres

param(
  [Parameter(Mandatory = $true)]
  [string]$DriverType
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")
$sourceDir = Join-Path $repoRoot "ops\driver-ui\$DriverType"
$targetDir = Join-Path $repoRoot "core\src-tauri\driver-ui\$DriverType"

if (-not (Test-Path $sourceDir -PathType Container)) {
  throw "Source driver directory not found: $sourceDir"
}

if (-not (Test-Path $targetDir)) {
  New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

function Get-RelativeChildPath([string]$basePath, [string]$fullPath) {
  $normalizedBasePath = [System.IO.Path]::GetFullPath($basePath)
  $normalizedFullPath = [System.IO.Path]::GetFullPath($fullPath)

  if (-not $normalizedBasePath.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $normalizedBasePath += [System.IO.Path]::DirectorySeparatorChar
  }

  if ($normalizedFullPath.StartsWith($normalizedBasePath, [System.StringComparison]::OrdinalIgnoreCase)) {
    return $normalizedFullPath.Substring($normalizedBasePath.Length)
  }

  throw "Path is not under base path. Base: $basePath Full: $fullPath"
}

function Remove-StaleArtifacts([string]$sourcePath, [string]$targetPath) {
  $sourceEntries = Get-ChildItem -Path $sourcePath -Recurse -Force
  $knownPaths = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)

  foreach ($entry in $sourceEntries) {
    $relativePath = Get-RelativeChildPath $sourcePath $entry.FullName
    [void]$knownPaths.Add($relativePath)
  }

  $targetEntries = Get-ChildItem -Path $targetPath -Recurse -Force |
    Sort-Object -Property FullName -Descending

  foreach ($entry in $targetEntries) {
    $relativePath = Get-RelativeChildPath $targetPath $entry.FullName
    if (-not $knownPaths.Contains($relativePath)) {
      Write-Host "  Removing stale artifact: $relativePath" -ForegroundColor DarkYellow
      Remove-Item -Path $entry.FullName -Recurse -Force
    }
  }
}

Write-Host ">>> staging driver artifacts"
Write-Host "  Source : $sourceDir"
Write-Host "  Target : $targetDir"

Remove-StaleArtifacts -sourcePath $sourceDir -targetPath $targetDir
Copy-Item -Path (Join-Path $sourceDir "*") -Destination $targetDir -Recurse -Force

Write-Host "Staged successfully (mirror sync): ops/driver-ui/$DriverType -> core/src-tauri/driver-ui/$DriverType" -ForegroundColor Green
