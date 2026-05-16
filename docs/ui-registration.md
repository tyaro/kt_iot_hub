# UI・タグ登録設計

## 本体 UI

本体の画面構成は 3 ペイン管理画面とする。

```shell
┌────────────┬─────────────────────────────┬───────────────────┐
│ Left Pane  │ Center Pane                 │ Right Pane         │
│ (Nav Tree) │ (一覧/編集)                  │ (詳細/プロパティ)  │
│            │                              │                    │
│ - Drivers  │ ・選択カテゴリの一覧表示     │ ・選択行の詳細     │
│ - Tags     │ ・新規/編集/削除ボタン       │ ・編集フォーム     │
│ - Publish. │ ・フィルタ/検索              │ ・リアルタイム値    │
│ - System   │                              │                    │
└────────────┴─────────────────────────────┴───────────────────┘
```

- 左ペイン: 機能カテゴリのツリー
- 中央ペイン: カテゴリ内のエンティティ一覧（タグ一覧、ドライバ一覧 等）
- 右ペイン: 選択中エンティティの詳細・編集
- ドライバ固有のタグ登録 UI は別ウィンドウ/別プロセスで起動する
- 本体 UI はタグ定義の一覧・共通項目編集・インポート/エクスポートを担当する

## ページ分割

- `/dashboard` — 稼働状況サマリ
- `/drivers` — ドライバ管理
- `/tags` — タグ管理
- `/publishers` — MQTT/OPC 配信設定
- `/logs` — ログビューア
- `/settings` — 全体設定

## タグ登録フロー

タグ登録は通常運転時の値取得とは別フローとする。

1. 本体 UI で対象ドライバを選択し、「タグ登録」を起動。
2. 本体が対応するドライバ登録プロセスを別ウィンドウで起動。
3. ドライバ登録プロセスが接続先探索・候補生成を実施。
4. 登録プロセスがタグ定義候補（共通タグ項目 + `driver_spec`）を JSON で本体へ返却。
5. 本体がタグ ID 重複、タグ名重複、データ型、必須項目、ドライバ種別整合性を検証。
6. 本体が `config/tags.toml` に保存し、必要に応じてドライバへタグ定義を再配布する。

## PostgreSQL 登録フロー

```text
接続先PostgreSQL設定
  → テーブル一覧表示
  → テーブル選択
  → 時系列フィールド選択
  → タグに使用するフィールド選択
  → タグ名・型・単位・コメント確認
  → 複数タグを一括登録
```

PostgreSQL では、テーブルの最新値をタグ値として扱うため、時系列フィールド（例: `created_at`, `measured_at`）を必須選択とする。
読出し周期はタグ単位ではなく、原則としてテーブル/スキャン単位で設定する。

## ドライバ別登録 UI 方針

- **決定**: ドライバ固有のタグ登録 UI は、本体 UI に差し込まず、**別ウィンドウ/別プロセス**として起動する。
- **理由**:
  - PostgreSQL / SLMP / JoyWatcher では、接続先探索・アドレス指定・タグ候補生成の流れが大きく異なる。
  - 本体 UI にドライバ固有画面を埋め込むと、本体がドライバ固有知識を持ちすぎる。
  - JoyWatcher DLL 等の外部依存・クラッシュリスクを本体から隔離できる。
  - ドライバごとの開発・テスト・配布を独立させやすい。

## 本体の責務

- タグ定義の正本管理
- タグ共通項目（タグ名、データ型、単位、コメント、有効/無効、ドライバID）の検証・保存
- `config/tags.toml` への書き戻し
- 登録結果の取り込み時バリデーション

## ドライバ登録プロセスの責務

- 接続設定 UI
- 接続先探索（例: PostgreSQL のテーブル/カラム一覧、SLMP のアドレス範囲、JoyWatcher のタグ一覧）
- ドライバ固有設定（`driver_spec`）の生成
- タグ候補の一括生成

## 受け渡し方式

- 初期実装: 一時 JSON ファイル
- 将来: gRPC / Named Pipe に統一

## タグ管理の階層モデル

タグ管理の正規階層は以下とする。

```text
DriverKind（postgres / slmp / joywatcher）
  └ Connection（接続先定義）
      └ ScanGroup（読出し周期・取得単位）
          └ Tag（共通項目 + driver_spec）
```

- UI 表示上は「接続先起点」の表示を許可するが、内部モデルは上記階層を正本とする。
- `Tag.driver_id` は `Connection.id` を参照し、`Tag.scan_group_id` は `ScanGroup.id` を参照する。
- 読出し周期はタグ単位ではなく `ScanGroup` 単位で管理する。

## 本体-登録プロセス連携プロトコル（初期版）

一時 JSON ファイル連携では、以下のメタ情報を必須とする。

- `schemaVersion`: スキーマ互換判定用
- `requestId`: 登録セッション識別子（重複取込防止）
- `generatedAt`: 生成時刻（監査・再実行判断）
- `driverKind` / `driverId`: ドライバ整合性確認

本体側受信時の最低バリデーション:

1. `schemaVersion` が対応範囲内であること
2. `driverKind` と `driverSpec.kind` が一致すること
3. `scanGroups[].id` と `tags[].driverSpec.scanGroup` が整合すること
4. タグ ID/名称重複がないこと（既存定義との衝突含む）
5. 参照不能な接続先・スキャングループがないこと

## 異常系・ロールバック方針

- 保存処理は「検証成功後に一括反映」とし、途中失敗時は反映しない。
- `config/tags.toml` 書込時は `tags.toml.tmp` に出力後、アトミック rename で置換する。
- 取込失敗時はエラー一覧（行/タグID/理由）を UI に返し、部分成功を作らない。
- ドライバ登録プロセス異常終了時は、セッション破棄と再実行導線を表示する。

## UX 要件（タグ登録）

- ウィザード各ステップで「戻る」「進む」「キャンセル」を統一配置する。
- 最終ステップで「確定前レビュー（追加件数/重複件数/警告）」を必須表示する。
- 長時間処理（接続探索・列取得）は進捗表示とタイムアウト再試行を提供する。
- 大量登録に備え、列フィルタ・全選択（時系列列除外）・命名ルール一括適用を提供する。

## タグ登録結果の概念例

```json
{
  "schemaVersion": 1,
  "driverKind": "postgres",
  "driverId": "postgres-main",
  "tags": [
    {
      "id": "tag-0001",
      "name": "temperature",
      "dataType": "f32",
      "unit": "degC",
      "comment": "温度",
      "enabled": true,
      "driverSpec": {
        "kind": "postgres",
        "table": "sensors",
        "valueColumn": "temperature",
        "timestampColumn": "created_at",
        "scanGroup": "sensors_1000ms"
      }
    }
  ],
  "scanGroups": [
    {
      "id": "sensors_1000ms",
      "table": "sensors",
      "timestampColumn": "created_at",
      "scanRateMs": 1000
    }
  ]
}
```

## `config/tags.toml` スキーマ例（正式運用案）

以下は本体へ取り込んだ後の永続化イメージである。

### 1) 最小構成（PoC/検証用）

```toml
[[scan_group]]
id = "sensors_1000ms"
driver = "postgres-main"
table = "sensors"
timestamp_column = "created_at"
scan_rate_ms = 1000

[[tag]]
id = "tag-0001"
name = "temperature"
data_type = "f32"
driver = "postgres-main"
scan_group = "sensors_1000ms"
driver_spec = { value_column = "temperature" }
enabled = true
```

### 2) 標準構成（推奨）

```toml
[[scan_group]]
id = "line1_sensors_1000ms"
driver = "postgres-main"
table = "line1_sensors"
timestamp_column = "measured_at"
scan_rate_ms = 1000

[[scan_group]]
id = "line1_system_5000ms"
driver = "postgres-main"
table = "line1_system"
timestamp_column = "updated_at"
scan_rate_ms = 5000

[[tag]]
id = "tag-line1-temp"
name = "Line1 Temperature"
data_type = "f32"
driver = "postgres-main"
scan_group = "line1_sensors_1000ms"
driver_spec = { value_column = "temperature" }
enabled = true
metadata = { unit = "degC", comment = "ライン1温度" }

[[tag]]
id = "tag-line1-pressure"
name = "Line1 Pressure"
data_type = "f64"
driver = "postgres-main"
scan_group = "line1_sensors_1000ms"
driver_spec = { value_column = "pressure" }
enabled = true
metadata = { unit = "kPa", comment = "ライン1圧力" }

[[tag]]
id = "tag-line1-status"
name = "Line1 Status"
data_type = "string"
driver = "postgres-main"
scan_group = "line1_system_5000ms"
driver_spec = { value_column = "status" }
enabled = true
metadata = { comment = "運転状態" }
```

### 3) 大量登録構成（命名規約前提）

```toml
[[scan_group]]
id = "plant_a_sensors_1000ms"
driver = "postgres-main"
table = "plant_a_sensors"
timestamp_column = "recorded_at"
scan_rate_ms = 1000

[[tag]]
id = "plant-a-sensors-temperature"
name = "plant_a_temperature"
data_type = "f32"
driver = "postgres-main"
scan_group = "plant_a_sensors_1000ms"
driver_spec = { value_column = "temperature" }
enabled = true

[[tag]]
id = "plant-a-sensors-humidity"
name = "plant_a_humidity"
data_type = "f32"
driver = "postgres-main"
scan_group = "plant_a_sensors_1000ms"
driver_spec = { value_column = "humidity" }
enabled = true

[[tag]]
id = "plant-a-sensors-vibration"
name = "plant_a_vibration"
data_type = "f64"
driver = "postgres-main"
scan_group = "plant_a_sensors_1000ms"
driver_spec = { value_column = "vibration" }
enabled = true
```

## 命名規約（推奨）

- `scan_group.id`: `<site>_<table>_<rate>ms`
- `tag.id`: 永続IDとして不変（表示名変更の影響を受けない）
- `tag.name`: UI 表示向け（ユーザーが変更可能）
- `driver`: 接続先ID（`DriverKind` ではない）

## 保存前チェックリスト（本体側）

1. `scan_group.id` の重複がない
2. `tag.id` の重複がない
3. `tag.scan_group` が存在する
4. `tag.driver` と `scan_group.driver` が一致する
5. `driver_spec.value_column` が空でない

## SLMP 登録 UI（将来）

SLMP ドライバも同様に、接続先設定 → アドレス設定 → タグ登録とする。
アドレスは連続割付けできるようにする。

## JoyWatcher 登録 UI（将来）

JoyWatcher の DLL は JoyWatcher のネットワークミドルウェアに接続する想定。
JoyWatcher 側のタグ取得後、本体タグ定義へマッピングする登録 UI を提供する。
