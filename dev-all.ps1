
# 開発用一括ビルド＆起動スクリプト
# ワークスペースルート（D:\develop\kt_iot_hub）で実行する前提

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

if (-not (Test-Path (Join-Path $repoRoot "core\src-tauri\Cargo.toml"))) {
	throw "workspace root で dev-all.ps1 を実行してください: $repoRoot"
}

Push-Location $repoRoot
try {
	Write-Host "=== [1] Tauri本体ビルド ==="
	Push-Location (Join-Path $repoRoot "core\src-tauri")
	try {
		cargo build
		if ($LASTEXITCODE -ne 0) { throw "Tauri本体ビルド失敗" }
	}
	finally {
		Pop-Location
	}

	Write-Host "=== [2] JoyWatcherドライバ群ビルド＆配置 ==="
	& (Join-Path $repoRoot "ops\scripts\build-dev-joywatcher-suite.ps1")

	Write-Host "=== [3] Postgresドライバ群ビルド＆配置 ==="
	& (Join-Path $repoRoot "ops\scripts\build-dev-driver-runtime.ps1")
	& (Join-Path $repoRoot "ops\scripts\build-dev-driver-ui.ps1")

	Write-Host "=== [4] Tauri本体起動 ==="
	npm run tauri-dev
}
finally {
	Pop-Location
}
