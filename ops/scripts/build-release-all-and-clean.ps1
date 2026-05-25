# 通信ドライバ / ブリッジ / ドライバ登録UI / 本体 をまとめて release ビルドし、
# 生成後に不要な中間ファイルを削除するスクリプト。
#
# Usage:
#   .\ops\scripts\build-release-all-and-clean.ps1
#   .\ops\scripts\build-release-all-and-clean.ps1 -Bump patch|minor|major

param(
  [ValidateSet("patch", "minor", "major")]
  [string]$Bump
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-BumpedVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$CurrentVersion,

    [Parameter(Mandatory = $true)]
    [ValidateSet("patch", "minor", "major")]
    [string]$BumpType
  )

  if ($CurrentVersion -notmatch '^(\d+)\.(\d+)\.(\d+)$') {
    throw "Unsupported version format: $CurrentVersion"
  }

  $major = [int]$Matches[1]
  $minor = [int]$Matches[2]
  $patch = [int]$Matches[3]

  switch ($BumpType) {
    "patch" { $patch += 1 }
    "minor" {
      $minor += 1
      $patch = 0
    }
    "major" {
      $major += 1
      $minor = 0
      $patch = 0
    }
    default {
      throw "Unknown bump type: $BumpType"
    }
  }

  return "$major.$minor.$patch"
}

function Update-VersionInFile {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string]$Pattern,

    [Parameter(Mandatory = $true)]
    [scriptblock]$ReplaceEvaluator
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $regex = [regex]::new($Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
  $match = $regex.Match($content)
  if (-not $match.Success) {
    throw "Version pattern not found in: $Path"
  }

  $updated = $regex.Replace($content, $ReplaceEvaluator, 1)
  Set-Content -LiteralPath $Path -Value $updated -NoNewline
}

function Bump-ProjectVersions {
  param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("patch", "minor", "major")]
    [string]$BumpType,

    [Parameter(Mandatory = $true)]
    [string]$RepoRoot
  )

  $packageJsonPath = Join-Path $RepoRoot "package.json"
  $cargoTomlPath = Join-Path $RepoRoot "core/src-tauri/Cargo.toml"
  $tauriConfPath = Join-Path $RepoRoot "core/src-tauri/tauri.conf.json"

  $packageJsonContent = Get-Content -LiteralPath $packageJsonPath -Raw
  $packageVersionMatch = [regex]::Match(
    $packageJsonContent,
    '"name"\s*:\s*"kt_iot_hub"\s*,\s*"version"\s*:\s*"(\d+\.\d+\.\d+)"',
    [System.Text.RegularExpressions.RegexOptions]::Singleline
  )
  if (-not $packageVersionMatch.Success) {
    throw "Failed to detect current version from package.json"
  }

  $currentVersion = $packageVersionMatch.Groups[1].Value
  $nextVersion = Get-BumpedVersion -CurrentVersion $currentVersion -BumpType $BumpType

  Write-Host ">>> bump version: $currentVersion -> $nextVersion" -ForegroundColor Cyan

  Update-VersionInFile -Path $packageJsonPath -Pattern '("name"\s*:\s*"kt_iot_hub"\s*,\s*"version"\s*:\s*")(\d+\.\d+\.\d+)(")' -ReplaceEvaluator {
    param($m)
    "$($m.Groups[1].Value)$nextVersion$($m.Groups[3].Value)"
  }

  Update-VersionInFile -Path $cargoTomlPath -Pattern '(\[package\]\s+name\s*=\s*"kt_iot_hub"\s+version\s*=\s*")(\d+\.\d+\.\d+)(")' -ReplaceEvaluator {
    param($m)
    "$($m.Groups[1].Value)$nextVersion$($m.Groups[3].Value)"
  }

  Update-VersionInFile -Path $tauriConfPath -Pattern '("productName"\s*:\s*"kt_iot_hub"\s*,\s*"version"\s*:\s*")(\d+\.\d+\.\d+)(")' -ReplaceEvaluator {
    param($m)
    "$($m.Groups[1].Value)$nextVersion$($m.Groups[3].Value)"
  }
}

function Remove-PathIfExists {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if (Test-Path -LiteralPath $Path) {
    Remove-Item -LiteralPath $Path -Recurse -Force
    Write-Host "removed: $Path"
  }
}

function Remove-ReleaseIntermediateDirectories {
  param(
    [Parameter(Mandatory = $true)]
    [string]$BaseReleaseDir
  )

  if (-not (Test-Path -LiteralPath $BaseReleaseDir)) {
    return
  }

  $intermediateNames = @(
    ".fingerprint",
    "build",
    "deps",
    "examples",
    "incremental",
    "nsis",
    "resources",
    "wix"
  )

  foreach ($name in $intermediateNames) {
    Remove-PathIfExists -Path (Join-Path $BaseReleaseDir $name)
  }
}

function Remove-ReleaseSidecarFiles {
  param(
    [Parameter(Mandatory = $true)]
    [string]$BaseReleaseDir
  )

  if (-not (Test-Path -LiteralPath $BaseReleaseDir)) {
    return
  }

  $unwantedExtensions = @(
    ".cargo-lock",
    ".d",
    ".pdb",
    ".nsi",
    ".nsh",
    ".wixobj",
    ".wixpdb",
    ".wxl"
  )

  Get-ChildItem -LiteralPath $BaseReleaseDir -Recurse -File |
    Where-Object { $unwantedExtensions -contains $_.Extension } |
    ForEach-Object {
      Remove-Item -LiteralPath $_.FullName -Force
      Write-Host "removed: $($_.FullName)"
    }
}

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = (Resolve-Path (Join-Path $scriptDir "../..")).Path

Push-Location $repoRoot
try {
  $totalSteps = if ($Bump) { 4 } else { 3 }

  if ($Bump) {
    Write-Host ">>> step 1/${totalSteps}: bump project version ($Bump)"
    Bump-ProjectVersions -BumpType $Bump -RepoRoot $repoRoot
  }

  $driverStep = if ($Bump) { 2 } else { 1 }
  $appStep = if ($Bump) { 3 } else { 2 }
  $cleanupStep = if ($Bump) { 4 } else { 3 }

  Write-Host ">>> step ${driverStep}/${totalSteps}: build release driver suite"
  & (Join-Path $scriptDir "build-release-driver-suite.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "build-release-driver-suite.ps1 failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step ${appStep}/${totalSteps}: build release app bundle"
  & npm run tauri-build
  if ($LASTEXITCODE -ne 0) {
    throw "npm run tauri-build failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step ${cleanupStep}/${totalSteps}: cleanup release intermediate files"
  Remove-ReleaseSidecarFiles -BaseReleaseDir (Join-Path $repoRoot "target\release")
  Remove-ReleaseSidecarFiles -BaseReleaseDir (Join-Path $repoRoot "target\i686-pc-windows-msvc\release")
  Remove-ReleaseIntermediateDirectories -BaseReleaseDir (Join-Path $repoRoot "target\release")
  Remove-ReleaseIntermediateDirectories -BaseReleaseDir (Join-Path $repoRoot "target\i686-pc-windows-msvc\release")
  Remove-PathIfExists -Path (Join-Path $repoRoot "target\i686-pc-windows-msvc\.fingerprint")

  Write-Host ">>> done: release build + intermediate cleanup completed" -ForegroundColor Green
}
finally {
  Pop-Location
}
