# debug ビルドで生成された不要ファイルを掃除して、ワークスペースをすっきりさせる。
#
# Usage:
#   .\ops\scripts\cleanup-debug-workspace.ps1

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

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = (Resolve-Path (Join-Path $scriptDir "../..")).Path

Push-Location $repoRoot
try {
  Write-Host ">>> cleanup debug outputs"

  # Cargo debug outputs
  Remove-PathIfExists -Path (Join-Path $repoRoot "target\debug")
  Remove-PathIfExists -Path (Join-Path $repoRoot "target\i686-pc-windows-msvc\debug")
  Remove-PathIfExists -Path (Join-Path $repoRoot "core\src-tauri\target\debug")
  Remove-PathIfExists -Path (Join-Path $repoRoot "core\src-tauri\target\i686-pc-windows-msvc\debug")

  # debug中間物（releaseは保持）
  Remove-PathIfExists -Path (Join-Path $repoRoot "target\.fingerprint")
  Remove-PathIfExists -Path (Join-Path $repoRoot "core\src-tauri\target\.fingerprint")

  # 旧debug配布先（存在時）
  Remove-PathIfExists -Path (Join-Path $repoRoot "target\debug\driver-ui")
  Remove-PathIfExists -Path (Join-Path $repoRoot "core\src-tauri\target\debug\driver-ui")

  Write-Host ">>> done: debug workspace cleanup completed" -ForegroundColor Green
}
finally {
  Pop-Location
}
