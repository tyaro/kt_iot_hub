# 通信ドライバ / ブリッジ / ドライバ登録UI / 本体 をまとめて release ビルドし、
# 生成後に不要な中間ファイルを削除するスクリプト。
#
# Usage:
#   .\ops\scripts\build-release-all-and-clean.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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
  Write-Host ">>> step 1/3: build release driver suite"
  & (Join-Path $scriptDir "build-release-driver-suite.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "build-release-driver-suite.ps1 failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step 2/3: build release app bundle"
  & npm run tauri-build
  if ($LASTEXITCODE -ne 0) {
    throw "npm run tauri-build failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step 3/3: cleanup release intermediate files"
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
