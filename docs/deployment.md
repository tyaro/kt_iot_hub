# インストーラ・配布設計

## インストーラ構成

- **方式**: Tauri 標準の MSI / NSIS インストーラを利用する。
- **対象 OS**: Windows。
- **表示言語**: 日本語（WiX: `ja-JP`, NSIS: `Japanese`）。

## 同梱物

- Tauri アプリ本体（`kt_iot_hub.exe`）
- Mosquitto Windows バイナリ
- 既定設定ファイル（`mosquitto.conf`）
- 必要に応じて JoyWatcher ランタイム DLL（再配布可否確認後）
- ドライバ実行ファイル（`driver-ui/postgres/driver-postgres.exe`, `driver-ui/joywatcher/driver-joywatcher.exe`）
- ドライバ登録UI（`driver-ui/postgres/registration-ui.exe`, `driver-ui/joywatcher/registration-ui.exe`）
- JoyWatcher x86 ブリッジ（`driver-ui/joywatcher/joywatcher-bridge-x86.exe`）

### ディレクトリ責務（driver artifacts）

- 開発時正本: `driver-ui/<type>/`
- bundle staging: `src-tauri/driver-ui/<type>/`

`src-tauri/driver-ui/` はインストーラ同梱用のステージング先であり、
正本は常に `driver-ui/` とする。

### 同梱用アーティファクト準備手順

1. `npm run driver-suite:release` を実行し、`driver-ui/` と `src-tauri/driver-ui/` 配下へ release 成果物を配置する。
2. `npm run tauri-build:bundle-drivers` を実行して本体インストーラをビルドする。

補足（責務分離）:

- `build-*`: `target/` へビルド
- `install-*`: `target/` から `driver-ui/`（正本）へ配置
- `stage-*`: `driver-ui/`（正本）から `src-tauri/driver-ui/`（staging）へ同期

`src-tauri/tauri.conf.json` の `bundle.resources` で `src-tauri/driver-ui/` 配下の実行ファイルを指定し、
インストーラへ同梱する。インストール後の既定配置先は `<app-dir>/resources/driver-ui/`。

## インストール時の処理

1. アプリ本体を `%ProgramFiles%\kt_iot_hub\` にインストールする。
2. Mosquitto をインストールする。
3. `mosquitto.conf` を配置する。
4. Mosquitto を Windows サービスとして登録する。
5. サービスを開始する。
6. 必要に応じてファイアウォール例外を追加する。

## アンインストール時の処理

1. サービス停止。
2. サービス削除。
3. アプリファイル削除。
4. 設定ファイルは保持オプションを提供する。

## ブローカー管理 UI

- アプリのシステムペインに MQTT ブローカー状態を表示する。
- サービスの開始/停止/再起動を UI から実行可能にする。
- ブローカー設定（リスナーポート、認証、TLS）の編集 UI を提供する。
- 保存時に `mosquitto.conf` を再生成し、サービスを再起動する。

## セキュリティ既定値

- 初期構成では localhost のみ Listen する。
- 外部公開を有効化する際は TLS + 認証を要求する。
- 認証情報・TLS 証明書のパスは `mosquitto.conf` に格納する。
- パスワードは Mosquitto の `password_file` 機構で別ファイル化する。

## JoyWatcher DLL の扱い

- `JoyWaApi.dll` は JoyWatcher 製品インストールで既定配置される `C:\Windows\SysWOW64` を優先探索する。
- 現行の x86 bridge 実装は `LoadLibraryW` / `GetProcAddress` による動的ロードのため、`JoyWaApi.lib` の配布は必須ではない。

## ライセンス確認

Mosquitto の同梱・再配布に関して、EPL/EDL ライセンス条件を確認する。
