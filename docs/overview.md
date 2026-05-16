# プロジェクト概要

## 目的

工場・現場などの**オフライン環境**で、各種 IoT 機器・SCADA・DB から値を収集し、MQTT を中核プロトコルとして配信する **IoT ハブ**を構築する。
将来的に OPC UA / OPC DA サーバとしても稼働可能な拡張性を持たせる。

## 技術スタック

| 区分 | 採用技術 |
| ------ | ---------- |
| アプリ基盤 | Tauri v2 |
| バックエンド | Rust (Edition 2021 以降) |
| フロントエンド | Svelte 5 |
| MQTT ブローカー | Mosquitto（外部サービス / Windows サービス） |
| MQTT クライアント | `rumqttc` |
| DB ドライバ | `tokio-postgres` / `sqlx` |
| 非同期ランタイム | Tokio |
| ロギング | `tracing` + `tracing-subscriber` |
| 設定 | TOML（`serde` + `toml`） |
| ドライバ IPC | gRPC（`tonic`）/ Named Pipe（Windows） |
| 永続化 | TOML ファイル（タグ定義・ドライバ設定） |

## 用語

本プロジェクトでは、タグ管理の正規階層を次で統一する。

```text
DriverKind（postgres / slmp / joywatcher）
  └ Connection（接続先定義）
      └ ScanGroup（読出し周期・取得単位）
          └ Tag（共通項目 + driver_spec）
```

| 用語 | 定義 | 設定上の対応 |
| ------ | ------ | ------ |
| **DriverKind** | ドライバの種類。例: `postgres`, `slmp`, `joywatcher`。 | `drivers.toml` の `driver_type` |
| **Connection** | 実際の接続先定義（ホスト、ポート、認証情報など）。 | `drivers.toml` の `[[driver]]` （`id` で識別） |
| **ScanGroup** | 取得単位と周期を表すグループ。PostgreSQL では主にテーブル単位。 | `tags.toml` の `[[scan_group]]` |
| **Tag** | 本体が統一的に扱うデータ定義。接続先・グループに紐づく。 | `tags.toml` の `[[tag]]` |
| **driver_spec** | ドライバ固有設定（列名、アドレス、タグパス等）。 | `tag.driver_spec` |
| **Tag Registration Tool** | ドライバ固有の登録フロー・探索 UI を持つ別ウィンドウ/別プロセス。 | `docs/ui-registration.md` |
| **Publisher** | タグ値を MQTT/OPC 等へ配信するコンポーネント。 | `publishers.toml` の `[[publisher]]` |

補足:

- `tag.driver` は DriverKind ではなく、**Connection ID（`driver.id`）** を参照する。
- `tag.scan_group` は `scan_group.id` を参照する。
- `scan_group.scan_rate_ms` が読出し周期の正本であり、タグ単位周期は持たない。

## 非機能要件

| 項目 | 方針 |
| ------ | ------ |
| パフォーマンス | タグ数 1万 / 100ms 周期 を目標。`broadcast` の容量設定で背圧管理。 |
| 可用性 | ドライバ単位での障害分離。1つのドライバ停止で全体は止めない。 |
| ロギング | `tracing` で構造化ログ。ファイルローテーション。 |
| セキュリティ | 当面はオフライン環境での使用を想定。MQTT は TLS 不要、認証なし。DB パスワード等の秘匿情報も TOML に平文で記載して問題なし。 |
| 設定変更 | 実行時にドライバ追加・削除可能（ホットリロード）。 |
| テスト | Driver trait のモック実装で結合テスト。CI で `cargo test` + UI 結合テスト。 |
