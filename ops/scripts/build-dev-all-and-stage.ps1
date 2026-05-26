# 通信ドライバ / ブリッジ / ドライバ登録UI / 本体 をまとめて debug ビルドし、
# 生成物を ops/driver-ui/ と core/src-tauri/driver-ui/ へ配置（stage）するスクリプト。
#
# Flow:
#   1. Build+Install+Stage: ドライバ/UI/ブリッジ（PostgreSQL + JoyWatcher）
#   2. Build: 本体（kt_iot_hub）debug
#
# Usage:
#   .\ops\scripts\build-dev-all-and-stage.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = (Resolve-Path (Join-Path $scriptDir "../..")).Path

Push-Location $repoRoot
try {
  Write-Host ">>> step 1/4: build debug JoyWatcher suite (ui/runtime/bridge)"
  & (Join-Path $scriptDir "build-dev-joywatcher-suite.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "build-dev-joywatcher-suite.ps1 failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step 2/4: build debug PostgreSQL runtime"
  & (Join-Path $scriptDir "build-dev-driver-runtime.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "build-dev-driver-runtime.ps1 failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step 3/4: build debug PostgreSQL driver-ui"
  & (Join-Path $scriptDir "build-dev-driver-ui.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "build-dev-driver-ui.ps1 failed (exit code $LASTEXITCODE)"
  }

  Write-Host ">>> step 4/4: build debug app (kt_iot_hub)"
  & cargo build --manifest-path "core/src-tauri/Cargo.toml"
  if ($LASTEXITCODE -ne 0) {
    throw "cargo build (core/src-tauri/Cargo.toml) failed (exit code $LASTEXITCODE)"
  }

  Write-Host ""
  Write-Host ">>> done: debug build + deploy(stage) completed" -ForegroundColor Green
  Write-Host "    app               : target/debug/kt_iot_hub.exe"
  Write-Host "    driver source     : ops/driver-ui/<driver-type>/"
  Write-Host "    bundle staging    : core/src-tauri/driver-ui/<driver-type>/"
}
finally {
  Pop-Location
}
