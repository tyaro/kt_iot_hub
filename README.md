# kt_iot_hub

オフライン環境向け IoT HUB（Tauri v2 + Rust + Svelte）です。

## ドキュメント入口

- 設計ドキュメント入口: [`docs/design.md`](./docs/design.md)
- プロジェクト概要・用語集: [`docs/overview.md`](./docs/overview.md)
- タグ登録 UI 方針: [`docs/ui-registration.md`](./docs/ui-registration.md)
- 設定仕様（TOML）: [`docs/config-spec.md`](./docs/config-spec.md)

## 現在の主な構成

- フロントエンド: Svelte 5 + Vite
- バックエンド: Rust（Tauri v2）
- 設定: `config/*.toml`（ローカル永続化用。Git 管理対象外）

## Monorepo 構成（段階移行中）

- `src-tauri/` : 本体アプリ
- `packages/protocol-rs/` : 本体/ドライバUI間の共有プロトコル定義（Rust）
- `apps/postgres/ui/` : PostgreSQL 用ドライバ登録UI（別アプリ）
- `apps/postgres/driver/` : PostgreSQL 通信ランタイム（別プロセス）

### `dist/` と `apps/postgres/ui/assets/` の違い

- ルートの `dist/` は、本体 Svelte UI を `npm run build` した結果の出力先です。
- `src-tauri/tauri.conf.json` の `frontendDist` は `../dist` を参照します。
- `apps/postgres/ui/assets/` は、PostgreSQL 登録UI (`registration-ui.exe`) が読み込む静的画面資産です。
- `apps/postgres/ui/tauri.conf.json` の `frontendDist` は `./assets` を参照します。

つまり、**本体アプリの画面はルート `dist/`、登録UI の画面は `apps/postgres/ui/assets/`** です。

> 現在は段階移行のため、`apps/postgres/ui` は最小実装です。
> まずは「本体と別物の実行ファイルとして起動できること」を優先し、
> 本格UIは次フェーズで実装します。

## ランタイム制御ポリシー（重要）

- gRPC サーバーは本体の基盤機能として常時稼働します。
- 「サーバ起動/停止」操作の対象は以下のみです。
  - ドライバ通信ランタイム（`DriverProcessManager` 管理）
  - MQTT パブリッシャ（`PublisherManager`）
- gRPC は登録UI・ドライバIPCで継続利用するため、ランタイム停止では止めません。
- gRPC 停止は本体終了時（graceful shutdown）のみ行います。

## ドライバ実行ファイルの配置

登録UI と通信ランタイムは、同じ `driver-ui/<driver_type>/` 配下に置くことを推奨します。

- 既定ファイル名:
  - `registration-ui.exe`（登録UI, Windows）
  - `driver-<type>.exe`（通信ランタイム, Windows）
- 配置スクリプト:
  - `npm run driver-ui:dev`（`apps/postgres/ui` をビルドして配置）
  - `npm run driver-runtime:dev`（`apps/postgres/driver` をビルドして配置）
  - `npm run driver-ui:install -- -DriverType postgres -SourcePath C:/tools/postgres-tag-ui/postgres-tag-ui.exe`
  - `npm run driver-runtime:install -- -DriverType postgres -SourcePath C:/tools/driver-postgres.exe`

設定画面の「ドライバ設置ベースパス」は、登録UI と通信ランタイムの両方に使われます。
たとえば以下のどちらでも動作します。

- `<app-dir>`
  - `driver-ui/postgres/registration-ui.exe`
  - `driver-ui/postgres/driver-postgres.exe`
- `<app-dir>/driver-ui`
  - `postgres/registration-ui.exe`
  - `postgres/driver-postgres.exe`

ここで `<app-dir>` は本体実行ファイル `kt_iot_hub.exe` が置かれているディレクトリです。
たとえば本体が `<app-dir>/kt_iot_hub.exe` にある場合、`<app-dir>` を設定すると
`driver-ui/<type>/registration-ui.exe` と `driver-ui/<type>/driver-<type>.exe` を探索します。

本体は次の順にランタイム実行ファイルを探索します。

1. 設定された driver-ui ベースパス配下の同居配置
2. `DRIVER_BIN_DIR` 配下
3. 本体実行ファイルと同じディレクトリ
4. PATH 上で解決できる実行名

PostgreSQL ランタイム本体の crate は `apps/postgres/driver` です。

見つからない場合は、ランタイム開始時にエラーを返します。

## 補足

- `参考/` 配下はライセンス上の理由で Git 管理対象外です（`.gitignore` 設定済み）。
