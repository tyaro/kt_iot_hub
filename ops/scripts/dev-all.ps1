# 開発用一括ビルド＆起動スクリプト
# ops/scripts 配下から実行し、ワークスペースルートを自動解決する。

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = (Resolve-Path (Join-Path $scriptDir "../..")).Path

if (-not (Test-Path (Join-Path $repoRoot "core\src-tauri\Cargo.toml"))) {
	throw "workspace root を解決できませんでした: $repoRoot"
}

Push-Location $repoRoot
try {
	Write-Host "=== [1] デバッグビルド＆配置（本体 + ドライバ + 登録UI + ブリッジ） ==="
	& (Join-Path $repoRoot "ops\scripts\build-dev-all-and-stage.ps1")
	if ($LASTEXITCODE -ne 0) { throw "build-dev-all-and-stage.ps1 失敗" }

	Write-Host "=== [2] Tauri本体起動 ==="
	npm run tauri-dev
}
finally {
	Pop-Location
}
