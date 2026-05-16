# kt_iot_hub 設計ドキュメント

> オフライン環境で稼働する IoT ハブアプリケーションの設計ドキュメント入口です。詳細は責務単位で分割しています。

## ドキュメント構成

| ファイル | 内容 |
| -------- | ---- |
| [`overview.md`](./overview.md) | 目的、技術スタック、用語（DriverType/Connection/ScanGroup/Tag の正本）、非機能要件 |
| [`architecture.md`](./architecture.md) | 全体構成、設計原則、ドメインモデル、Tag Bus、データフロー |
| [`ui-registration.md`](./ui-registration.md) | 3ペイン UI、ドライバ別タグ登録 UI、PostgreSQL 登録フロー |
| [`decisions.md`](./decisions.md) | 方針決定事項、実現可能性レビュー |
| [`config-spec.md`](./config-spec.md) | TOML 設定ファイル仕様、`driver_spec`、`scan_group` |
| [`templates/driver-ui-request-template.json`](./templates/driver-ui-request-template.json) | 本体→ドライバUI 連携JSONテンプレート |
| [`templates/driver-ui-request-fields.md`](./templates/driver-ui-request-fields.md) | 本体→ドライバUI `--input-json` フィールド仕様 |
| [`templates/driver-ui-response-template.json`](./templates/driver-ui-response-template.json) | ドライバUI→本体 返却JSONテンプレート |
| [`templates/driver-ui-response-fields.md`](./templates/driver-ui-response-fields.md) | ドライバUI→本体 `--output-json` フィールド仕様 |
| [`deployment.md`](./deployment.md) | インストーラ、Mosquitto 同梱、Windows サービス、配布設計 |
| [`roadmap.md`](./roadmap.md) | Phase 別実装計画、リスク、品質戦略、マイルストーン |

## 重要方針サマリ

- 本体は Tauri v2 + Rust + Svelte 5 で構成する。
- MQTT ブローカーは本体に内蔵せず、Mosquitto を外部プロセス / Windows サービスとして扱う。
- タグ定義・ドライバ設定・パブリッシャ設定は TOML を正本とする。
- ドライバ固有のタグ登録 UI は本体 UI に差し込まず、別ウィンドウ/別プロセスとして提供する。
- 接続先（ドライバ）管理の導線はタグ管理画面へ統合し、接続先作成・ScanGroup作成・タグ作成は外部ドライバ UI で一連実行する。
- ドライバ固有情報はタグ定義内の `driver_spec` に格納し、本体は詳細解釈しない。
- PostgreSQL のようなテーブル単位読出しは `scan_group` で周期管理する。
- 将来的なドライバ分離は子プロセス + IPC（gRPC / Named Pipe）を基本方針とする。

## 現在の実装状況メモ

- Phase 1 の PoC として、AppState、TagRegistry、TagBus、PostgreSQL ドライバ、MQTT Publisher、タグ一覧 UI、ドライバ設定 UI を実装中。
- 現在のフロントエンドは Svelte 5 + Vite 構成。
- PostgreSQL 実 DB 連携は `sql/bootstrap_postgres.sql` 実行後に確認する。

## 更新ルール

- 新しい設計判断はまず [`decisions.md`](./decisions.md) に追記する。
- 実装フェーズやタスクの変更は [`roadmap.md`](./roadmap.md) に追記する。
- TOML スキーマや例の変更は [`config-spec.md`](./config-spec.md) に追記する。
- ドライバ別登録 UI の仕様変更は [`ui-registration.md`](./ui-registration.md) に追記する。
