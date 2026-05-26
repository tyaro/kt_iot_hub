# タグ設定インポート / エクスポート仕様

本書は、外部アプリケーションが `kt_iot_hub` へ取り込むためのタグ管理設定 JSON を生成する際の仕様を定義する。

- 対象コマンド:
  - `export_tag_management_settings`
  - `import_tag_management_settings`
- 実装基準:
  - `core/src-tauri/src/commands/driver/transfer.rs`
  - `core/src-tauri/src/commands/driver/transfer_impl.rs`

## 1. 目的とスコープ

この import/export は、以下 3 種の設定を一括で扱う。

1. `drivers`
2. `scanGroups`
3. `tags`

非対象:

- `publishers.toml`（MQTT パブリッシャ設定）
- `runtime.toml`（ランタイム自動起動設定など）

## 2. API 仕様（Tauri command）

### 2.1 エクスポート

- コマンド名: `export_tag_management_settings`
- リクエスト:
  - `path: string`（必須、空文字不可）
- レスポンス:
  - `path: string`
  - `driver_count: number`
  - `scan_group_count: number`
  - `tag_count: number`

備考:

- 親ディレクトリが存在しない場合は自動作成する。
- 出力 JSON は pretty format（整形）で保存される。

### 2.2 インポート

- コマンド名: `import_tag_management_settings`
- リクエスト:
  - `path: string`（必須、空文字不可）
- レスポンス:
  - `path: string`
  - `driver_count: number`
  - `scan_group_count: number`
  - `tag_count: number`

## 3. JSON ルート構造

```json
{
  "schemaVersion": 1,
  "exportedAt": "2026-05-26T00:00:00Z",
  "drivers": [],
  "scanGroups": [],
  "tags": []
}
```

### 3.1 ルート項目

- `schemaVersion`（number, 必須）
  - 現在は `1` 固定
  - 1 以外は import 拒否
- `exportedAt`（string, 必須）
  - RFC3339 を推奨（エクスポート時は RFC3339）
  - 現状 import で厳密バリデーションはしない
- `drivers`（array, 必須）
- `scanGroups`（array, 必須）
- `tags`（array, 必須）

## 4. 各配列要素の形式

## 4.1 drivers[]

`DriverConfig` 相当。固定キー + 可変キー（ドライバ設定）を同一オブジェクトに持つ。

必須/主要キー:

- `id: string`
- `driver_type: string`
- `enabled: boolean | null`（未指定も可）

可変キー（例）:

- PostgreSQL: `host`, `port`, `database`, `username`, `password_key`, `tls_enabled` など
- JoyWatcher: `endpoint`, `user_id`, `password_key`, `auto_restart` など

注意:

- import 時点では `driver_type` ごとの詳細必須キーは検証しない。
- ただし不足キーがあると、後続のドライバ起動/通信で失敗する可能性がある。

### 4.2 scanGroups[]

`ScanGroupConfig` 相当。

キー:

- `id: string`（必須）
- `driver: string`（必須。`drivers[].id` を参照）
- `scan_rate_ms: number`（必須）
- `schema: string | null`
- `table: string | null`
- `timestamp_column: string | null`
- `node: string | null`

### 4.3 tags[]

`TagConfig` 相当。

キー:

- `id: string`（必須）
- `name: string`（必須）
- `data_type: string`（必須）
- `driver: string`（必須。`drivers[].id` を参照）
- `scan_group: string`（必須。`scanGroups[].id` を参照）
- `driver_spec: object`（必須）
- `enabled: boolean | null`
- `metadata: object | null`

`data_type` は本体の `DataType` として解釈可能な値である必要がある。

## 5. インポート時バリデーション

`import_tag_management_settings` では、少なくとも以下を検証する。

1. `schemaVersion == 1`
2. `drivers[].id` 重複なし
3. `scanGroups[].id` 重複なし
4. `tags[].id` 重複なし
5. 各 `scanGroups[].driver` が既知 driver を参照
6. 各 `tags[].driver` が既知 driver を参照
7. 各 `tags[].scan_group` が既知 scanGroup を参照
8. 各 `tags[].data_type` が有効値
9. 各 tag について `tag.driver == scanGroup.driver`

いずれか違反時は import 全体を失敗として扱う（部分適用しない）。

## 6. 適用時の動作

インポート成功時の適用順序:

1. （必要なら）起動中ドライバを停止
2. `drivers.toml` と `tags.toml` をアトミック書き込み
3. メモリ上の driver / scanGroup 状態を置換
4. レジストリ上の tag を全置換
5. 停止していたドライバを再起動（有効化済みのみ）

備考:

- import 対象は drivers/scanGroups/tags のみ。
- `publishers` / `runtime` は変更しない。

## 7. 命名・互換ルール（外部生成ツール向け）

外部生成時は以下を推奨する。

- ルートは `schemaVersion` / `exportedAt` / `scanGroups`（camelCase）
- 内部要素キーは現行設定定義に合わせる（例: `driver_type`, `scan_group`, `driver_spec`）
- 不要なキーは付与しない（将来互換性のため）
- unknown キーを使う場合はドライバ固有設定として扱われることを理解したうえで使用する

## 8. 最小サンプル

```json
{
  "schemaVersion": 1,
  "exportedAt": "2026-05-26T00:00:00Z",
  "drivers": [
    {
      "id": "postgres-main",
      "driver_type": "postgres",
      "enabled": true,
      "host": "127.0.0.1",
      "port": 5432,
      "database": "iot_hub",
      "username": "iot_user",
      "password_key": "driver/postgres-main/password"
    }
  ],
  "scanGroups": [
    {
      "id": "sensors_1000ms",
      "driver": "postgres-main",
      "scan_rate_ms": 1000,
      "schema": "public",
      "table": "sensors",
      "timestamp_column": "created_at",
      "node": null
    }
  ],
  "tags": [
    {
      "id": "tag-0001",
      "name": "line1.temperature",
      "data_type": "f64",
      "driver": "postgres-main",
      "scan_group": "sensors_1000ms",
      "driver_spec": {
        "kind": "postgres",
        "valueColumn": "temperature"
      },
      "enabled": true,
      "metadata": null
    }
  ]
}
```

## 9. エラーハンドリング指針（外部アプリ向け）

外部アプリが import 用 JSON を生成する場合、事前に次を実施すると安全。

1. ID 重複チェック（driver / scanGroup / tag）
2. 参照整合チェック（tag→driver, tag→scanGroup, scanGroup→driver）
3. `data_type` 妥当性チェック
4. `schemaVersion` の固定化（`1`）

これにより、本体側 import 失敗率を大幅に下げられる。
