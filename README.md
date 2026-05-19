# kt_iot_hub

オフライン環境向け IoT HUB（Tauri v2 + Rust + Svelte）です。

## ドキュメント入口

- 設計ドキュメント入口: [`docs/design.md`](./docs/design.md)
- プロジェクト概要・用語集: [`docs/overview.md`](./docs/overview.md)
- タグ登録 UI 方針: [`docs/ui-registration.md`](./docs/ui-registration.md)
- ドライバUI / 通信ドライバ作成ガイド: [`drivers/docs/driver-development.md`](./drivers/docs/driver-development.md)
- ドライバ実装チェックフロー: [`drivers/docs/driver-implementation-flow.md`](./drivers/docs/driver-implementation-flow.md)
- 設定仕様（TOML）: [`docs/config-spec.md`](./docs/config-spec.md)

## 現在の主な構成

- フロントエンド: Svelte 5 + Vite
- バックエンド: Rust（Tauri v2）
- 設定: `ops/config/*.toml`（ローカル永続化用。Git 管理対象外）

## Monorepo 構成（段階移行中）

- `core/src-tauri/` : 本体アプリ
- `packages/protocol-rs/` : 本体/ドライバUI間の共有プロトコル定義（Rust）
- `drivers/postgres/ui/` : PostgreSQL 用ドライバ登録UI（別アプリ）
- `drivers/postgres/driver/` : PostgreSQL 通信ランタイム（別プロセス）

### `core/dist/` と `drivers/postgres/ui/assets/` の違い

- `core/dist/` は、本体 Svelte UI を `npm run build` した結果の出力先です。
- `core/src-tauri/tauri.conf.json` の `frontendDist` は `../dist` を参照します。
- `drivers/postgres/ui/assets/` は、PostgreSQL 登録UI (`registration-ui.exe`) が読み込む静的画面資産です。
- `drivers/postgres/ui/tauri.conf.json` の `frontendDist` は `./assets` を参照します。

つまり、**本体アプリの画面は `core/dist/`、登録UI の画面は `drivers/postgres/ui/assets/`** です。

> 現在は段階移行のため、`drivers/postgres/ui` は最小実装です。
> まずは「本体と別物の実行ファイルとして起動できること」を優先し、
> 本格UIは次フェーズで実装します。

## ランタイム制御ポリシー（重要）

- gRPC サーバーは本体の基盤機能として常時稼働します。
- 「サーバ起動/停止」操作の対象は以下のみです。
  - ドライバ通信ランタイム（`DriverProcessManager` 管理）
  - MQTT パブリッシャ（`PublisherManager`）
- gRPC は登録UI・ドライバIPCで継続利用するため、ランタイム停止では止めません。
- gRPC 停止は本体終了時（graceful shutdown）のみ行います。

## Driver and Registration UI Deployment

### 配置責務

- 開発時正本: `ops/driver-ui/<driver_type>/`
  - `registration-ui.exe`
  - `driver-<type>.exe`
- 同梱ステージング: `core/src-tauri/driver-ui/<driver_type>/`
  - インストーラ bundle 直前の同期先
  - 正本ではない（直接編集しない）

二重配置の理由は、**開発・検証時の正本管理**と**インストーラ同梱用ステージング**を分離するためです。

### 配置・同梱の実行手順

- `npm run driver-ui:dev`（`drivers/postgres/ui` をビルドして正本へ配置）
- `npm run driver-runtime:dev`（`drivers/postgres/driver` をビルドして正本へ配置）
- `npm run driver-suite:release`（release 成果物を正本へ配置し、staging へ同期）
- `npm run tauri-build:bundle-drivers`（staging を同梱してインストーラをビルド）

手動配置が必要な場合:

- `npm run driver-ui:install -- -DriverType postgres -SourcePath C:/tools/postgres-tag-ui/postgres-tag-ui.exe`
- `npm run driver-runtime:install -- -DriverType postgres -SourcePath C:/tools/driver-postgres.exe`

### 実行時探索

設定画面の「ドライバ設置ベースパス」は、登録UI と通信ランタイムの両方に使われます。
同梱インストーラ版の既定値は通常 `<app-dir>/resources` です。

`<app-dir>` は `kt_iot_hub.exe` が置かれているディレクトリで、以下のような配置を探索できます。

- `<app-dir>/resources/driver-ui/postgres/registration-ui.exe`
- `<app-dir>/resources/driver-ui/postgres/driver-postgres.exe`
- `<app-dir>/driver-ui/postgres/registration-ui.exe`
- `<app-dir>/driver-ui/postgres/driver-postgres.exe`

通信ランタイムは次の順で探索します。

1. 設定された driver-ui ベースパス配下の同居配置
2. `DRIVER_BIN_DIR` 配下
3. 本体実行ファイルと同じディレクトリ
4. PATH 上で解決できる実行名

見つからない場合は、ランタイム開始時にエラーを返します。

### インストーラ同梱対象（v0.2.0）

- `ops/driver-ui/postgres/registration-ui.exe`
- `ops/driver-ui/postgres/driver-postgres.exe`
- `ops/driver-ui/joywatcher/registration-ui.exe`
- `ops/driver-ui/joywatcher/driver-joywatcher.exe`
- `ops/driver-ui/joywatcher/joywatcher-bridge-x86.exe`

## JoyWatcher DLL の扱い

- `JoyWaApi.dll` は JoyWatcher インストール環境の既定配置先である `C:\Windows\SysWOW64` を優先探索します。
- `JoyWaApi.lib` は現行の x86 bridge 実行方式では不要です。
- タグ管理画面では、接続先・スキャングループ・タグ定義を JSON でインポート / エクスポートできます。

## 補足

- `参考/` 配下はライセンス上の理由で Git 管理対象外です（`.gitignore` 設定済み）。
