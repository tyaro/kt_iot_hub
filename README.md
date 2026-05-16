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
- 設定: `config/*.toml`

## Monorepo 構成（段階移行中）

- `src-tauri/` : 本体アプリ
- `packages/protocol-rs/` : 本体/ドライバUI間の共有プロトコル定義（Rust）
- `apps/driver-ui-postgres/` : PostgreSQL 用ドライバUI（別アプリ）

> 現在は段階移行のため、`apps/driver-ui-postgres` は最小実装です。
> まずは「本体と別物の実行ファイルとして起動できること」を優先し、
> 本格UIは次フェーズで実装します。

## ドライバUI実行ファイルの配置

ドライバUIは本体アプリ配下の `driver-ui/<driver_type>/` に配置します。

- 既定ファイル名: `registration-ui.exe`（Windows）
- 配置スクリプト:
  - `npm run driver-ui:dev`（`apps/driver-ui-postgres` をビルドして配置）
  - `npm run driver-ui:install -- -DriverType postgres -SourcePath C:/tools/postgres-tag-ui/postgres-tag-ui.exe`

本体は `driver_type` を使って、上記既定配置を探索して起動します。

## 補足

- `参考/` 配下はライセンス上の理由で Git 管理対象外です（`.gitignore` 設定済み）。
