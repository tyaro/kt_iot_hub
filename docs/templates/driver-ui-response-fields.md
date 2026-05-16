# driver-ui-response-template フィールド仕様（`--output-json`）

この文書は、外部ドライバUIが本体へ返却する `--output-json` の正式仕様です。  
対応サンプルは `./driver-ui-response-template.json` を参照してください。

- 方向: **ドライバUI → 本体**
- 目的: 接続先定義・ScanGroup・Tag の確定結果を本体へ一括反映する

## ルートオブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `schemaVersion` | `number` | 必須 | スキーマバージョン。現在は `1`。 |
| `requestId` | `string` | 推奨 | 本体から受け取った `requestId` の引き継ぎを推奨。 |
| `generatedAt` | `string` | 推奨 | 返却JSONの生成時刻（ISO 8601 / RFC3339）。 |
| `direction` | `string` | 任意 | `driver-to-host` を推奨。 |
| `driver` | `object` | 必須 | 接続先定義。`id` と `driverType` を含む。 |
| `scanGroups` | `array<object>` | 必須 | 対象接続先の ScanGroup 一覧。空配列可。 |
| `tags` | `array<object>` | 必須 | 対象接続先の Tag 一覧。空配列可。 |

---

## `driver` オブジェクト

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | `string` | 必須 | 接続先ID。 |
| `driverType` | `string` | 必須 | ドライバ種別（例: `postgres`）。 |
| `enabled` | `boolean` | 任意 | 接続先有効/無効。省略時は `true` 扱い。 |
| `settings` | `object` | 必須 | 接続設定。ドライバ固有キーを含む。 |

`settings` の例（postgres）:

- `host`: `string`
- `port`: `number`
- `database`: `string`
- `username`: `string`
- `password`: `string`
- `registration_ui_path`: `string`（任意）

---

## `scanGroups[]` 要素

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | `string` | 必須 | ScanGroup ID。 |
| `scanRateMs` | `number` | 推奨 | 省略時は本体側既定値（現行 `1000`）。 |
| `schema` | `string` | 任意 | PostgreSQL スキーマ名など。 |
| `table` | `string` | 任意 | 読出し対象テーブル。 |
| `timestampColumn` | `string` | 任意 | 最新値判定に使う時系列列。 |
| `node` | `string` | 任意 | 非SQL系ドライバ向けノード情報。 |

---

## `tags[]` 要素

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `id` | `string` | 必須 | タグ永続ID（システム全体で一意）。 |
| `name` | `string` | 必須 | タグ表示名。 |
| `dataType` | `string` | 必須 | タグ型（`bool`,`i32`,`i64`,`f32`,`f64`,`string`）。 |
| `enabled` | `boolean` | 任意 | 省略時は本体実装で既定処理。 |
| `unit` | `string` | 任意 | 単位。 |
| `comment` | `string` | 任意 | コメント。 |
| `driverSpec` | `object` | 必須 | ドライバ固有設定。 |

`driverSpec` には以下を含めること:

- `scanGroup`（`scanGroups[].id` と一致）
- `kind`（`driver.driverType` と整合）

---

## 本体側の受信バリデーション（要点）

1. `schemaVersion == 1` であること。  
2. `driver.id` と `driver.driverType` を含むこと。  
3. `driver.driverType` と `tags[].driverSpec.kind` が整合すること。  
4. `tags[].driverSpec.scanGroup` が `scanGroups[].id` に存在すること。  
5. タグID重複がないこと（既存定義との衝突含む）。  
6. 同一接続先・同一ScanGroup内でタグ名重複がないこと。

---

## ドライバUI実装時の扱い

1. 本体起動時に受け取った `outputJsonPath` へ最終結果を上書き保存する。  
2. 途中保存ファイルではなく、**確定時の最終状態のみ**を書き出す。  
3. `driver` ブロックは常に出力し、`id` / `driverType` / `settings` を含める。  
4. 不明フィールドがあっても本体は既知フィールドで処理する。
