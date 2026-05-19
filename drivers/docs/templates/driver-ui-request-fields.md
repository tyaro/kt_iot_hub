# driver-ui-request-template フィールド仕様（`--input-json`）

この文書は、外部ドライバUIが本体から受け取る `--input-json` の正式仕様です。  
対応サンプルは `./driver-ui-request-template.json` を参照してください。

- 方向: **本体 → ドライバUI**
- 目的: ドライバUI起動時に、既存接続先・ScanGroup・Tag の編集コンテキストを渡す

## ルートオブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `schemaVersion` | `number` | 必須 | スキーマバージョン。現在は `1` を想定。 |
| `requestId` | `string` | 必須 | セッション内の要求識別子。ログ相関に使用。 |
| `generatedAt` | `string` | 必須 | 生成時刻（ISO 8601 / RFC3339）。 |
| `direction` | `string` | 必須 | 方向識別。`host-to-driver` 固定。 |
| `session` | `object` | 必須 | セッション情報。 |
| `driver` | `object` | 必須 | 起動対象ドライバ情報。 |
| `context` | `object` | 必須 | 編集対象の ScanGroup / Tag コンテキスト。 |

## `session` オブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `sessionId` | `string` | 必須 | 本体が発行するセッションID。 |
| `mode` | `string` | 必須 | 起動モード。現行は `create-or-edit`。 |
| `outputJsonPath` | `string` | 必須 | ドライバUIが確定結果を書き出す先。 |

## `driver` オブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `driverType` | `string` | 必須 | ドライバ種別（例: `postgres`）。 |
| `driverId` | `string` | 任意 | 既存接続先編集時に設定。新規接続先作成時は省略可。 |

## `context` オブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `scanGroups` | `array<object>` | 必須 | 既存 ScanGroup 一覧。新規時は空配列可。 |
| `tags` | `array<object>` | 必須 | 既存 Tag 一覧。新規時は空配列可。 |

---

## `context.scanGroups[]` 要素

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | `string` | 必須 | ScanGroup ID。 |
| `driver` | `string` | 必須 | 所属接続先ID。 |
| `scanRateMs` | `number` | 必須 | 読出し周期（ms）。 |
| `schema` | `string` | 任意 | PostgreSQL スキーマ名など。 |
| `table` | `string` | 任意 | 読出し対象テーブル。 |
| `timestampColumn` | `string` | 任意 | 最新値判定に使う時系列列。 |
| `node` | `string` | 任意 | 非SQL系ドライバ向けノード情報。 |

## `context.tags[]` 要素

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | `string` | 必須 | タグ永続ID。 |
| `name` | `string` | 必須 | タグ表示名。 |
| `dataType` | `string` | 必須 | タグ型（`bool`,`i32`,`i64`,`f32`,`f64`,`string`）。 |
| `driverId` | `string` | 必須 | 所属接続先ID。 |
| `scanGroupId` | `string` | 必須 | 所属 ScanGroup ID。 |
| `enabled` | `boolean` | 必須 | 有効フラグ。 |
| `driverSpec` | `object` | 必須 | ドライバ固有設定。`scanGroup` を含むこと。 |
| `metadata` | `object` | 任意 | 付加情報（`unit`,`comment` など）。 |

---

## ドライバUI実装時の扱い

1. `outputJsonPath` に、確定結果を `driver-ui-response-template.json` 形式で書き出す。  
2. `requestId` は戻りJSONへ引き継ぐことを推奨。  
3. `driverId` が省略されている場合は、新規接続先作成として扱う。  
4. 不明フィールドは無視し、既知フィールドのみで処理する（前方互換）。
