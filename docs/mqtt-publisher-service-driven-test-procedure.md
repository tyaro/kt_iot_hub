# MQTTパブリッシャ サービス駆動 実機テスト手順書

## 1. 目的

`runtime` の起動/停止および自動起動設定に連動して、MQTTパブリッシャ（`PublisherManager` 管理）が期待どおりに動作することを実機で確認する。

対象機能:

- `start_runtime_services` 実行時に MQTT パブリッシャが起動すること
- `stop_runtime_services` 実行時に MQTT パブリッシャが停止すること
- `runtime.auto_start_runtime_services` 設定に従って起動時挙動が切り替わること

## 2. 事前条件

### 2.1 環境

- OS: Windows
- 本体: `kt_iot_hub`（対象ブランチの最新ビルド）
- 外部MQTTブローカー: Mosquitto または EMQX（ローカルで到達可能）

### 2.2 設定ファイル

`ops/config/publishers.toml`

- 少なくとも1件の `publisher_type = "mqtt"` が存在すること
- ブローカー接続先（`broker` / `port` / `username` / `password`）が実環境に合っていること

`ops/config/runtime.toml`

- `auto_start_runtime_services = false` から開始する（TC-03で使用）

### 2.3 観測手段

- kt_iot_hub のシステム状態表示（`publishers_running`）
- 本体ログ（以下の文字列を確認）
  - `Runtime start requested`
  - `Runtime started: drivers_running=... publishers_running=... grpc_running=...`
  - `Runtime stop requested`
  - `Runtime stopped: drivers_running=... publishers_running=... grpc_running=...`
  - `Failed to auto start runtime services`（異常系のみ）
- MQTT受信ツール（MQTT Explorer等）または `core/docs/mqtt-monitor.md` に沿ったモニタ画面

## 3. 事前準備

1. `ops/config/publishers.toml` をバックアップする。
2. テスト用 topic を固定する（例: `kt_iot_hub/e2e`）。
3. ブローカーが待受中であることを確認する。
4. 本体を起動し、ランタイムを停止状態にする。

## 4. テストケース

---

### TC-01: 手動起動で MQTTパブリッシャが開始する

目的:

- `start_runtime_services` により `publishers_running=true` になること
- MQTT publish が開始されること

手順:

1. 本体を起動（`runtime.auto_start_runtime_services=false`）。
2. ランタイム開始操作を実行する。
3. MQTT受信ツールで対象 topic を購読する。
4. 任意タグ値を変化させ、publish を発生させる。

期待結果:

- システム状態で `publishers_running=true`
- ログに `Runtime start requested` と `Runtime started: ... publishers_running=true ...`
- MQTT受信ツールでメッセージを受信できる

---

### TC-02: 手動停止で MQTTパブリッシャが停止する

目的:

- `stop_runtime_services` により publish が停止すること

手順:

1. TC-01 実施後の状態（publish中）で開始する。
2. ランタイム停止操作を実行する。
3. タグ値を変化させる（publishトリガを再発生）。
4. MQTT受信ツールの受信有無を確認する。

期待結果:

- システム状態で `publishers_running=false`
- ログに `Runtime stop requested` と `Runtime stopped: ... publishers_running=false ...`
- 停止後は新規メッセージが受信されない

---

### TC-03: 自動起動OFFでアプリ起動時にパブリッシャが起動しない

目的:

- `runtime.toml` の `auto_start_runtime_services=false` が有効であること

手順:

1. `ops/config/runtime.toml` を `auto_start_runtime_services = false` に設定する。
2. 本体を再起動する。
3. 起動直後のシステム状態とログを確認する。

期待結果:

- 起動直後は `publishers_running=false`
- `Runtime started: ...` が自動では出ない
- MQTT publish は手動でランタイム開始するまで発生しない

---

### TC-04: 自動起動ONでアプリ起動時にパブリッシャが起動する

目的:

- `runtime.toml` の `auto_start_runtime_services=true` が有効であること

手順:

1. `ops/config/runtime.toml` を `auto_start_runtime_services = true` に設定する。
2. 本体を再起動する。
3. 起動直後のシステム状態とログを確認する。
4. MQTT受信ツールでメッセージ受信可否を確認する。

期待結果:

- 起動後に `publishers_running=true` になる
- 自動起動後、タグ更新に応じて publish される
- 自動起動失敗時は `Failed to auto start runtime services` が出力される

---

### TC-05: publishers未設定時の安全動作

目的:

- `publishers.toml` が空または有効publisherなしでも異常終了しないこと

手順:

1. `publishers.toml` をバックアップのうえ、`[[publisher]]` を一時的に0件にする。
2. 本体起動後、ランタイム開始を実行する。
3. ログと状態表示を確認する。
4. テスト後に必ず元の `publishers.toml` を復元する。

期待結果:

- アプリは異常終了しない
- `publishers_running=false` のまま
- 例外的な panic / crash が発生しない

## 5. 異常系補足確認（推奨）

### TC-06: ブローカー停止時の起動失敗ハンドリング

目的:

- パブリッシャ起動失敗時に状態とログが妥当であること

手順:

1. ブローカーを停止する。
2. ランタイム開始を実行する。
3. ログ・状態表示を確認する。

期待結果:

- エラーログが記録される
- `publishers_running` が誤って `true` 固定にならない
- 失敗時は必要に応じてドライバ起動が巻き戻される（runtime実装のエラー分岐）

## 6. 判定基準

以下をすべて満たした場合、サービス駆動機能は「実機確認OK」とする。

1. 手動 start/stop と `publishers_running` の状態遷移が一致する
2. `auto_start_runtime_services` の ON/OFF で起動時挙動が切り替わる
3. start中のみ MQTT publish を確認できる
4. 異常系でクラッシュせず、ログで原因追跡できる

## 7. テスト後の戻し

1. `ops/config/runtime.toml` を運用値へ戻す。
2. `ops/config/publishers.toml` をバックアップから復元する。
3. ブローカー状態（サービス起動/停止）を運用状態へ戻す。

## 8. 記録テンプレート

- 実施日:
- 実施者:
- ブランチ/コミット:
- ブローカー種別/バージョン:
- TC-01: Pass / Fail（補足）
- TC-02: Pass / Fail（補足）
- TC-03: Pass / Fail（補足）
- TC-04: Pass / Fail（補足）
- TC-05: Pass / Fail（補足）
- TC-06: Pass / Fail（補足）
- 総合判定:
