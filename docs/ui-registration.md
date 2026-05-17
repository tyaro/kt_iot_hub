# UI・タグ登録設計

> 実装手順・受け入れ条件・別セッション向けの再開導線は [`driver-development.md`](./driver-development.md) を参照。

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
- **初期状態では Driver / Publisher のサービス処理は起動しない**。ただしタグ登録用 gRPC は IPC 用途として常時起動してよい。

## ページ分割

- `/dashboard` — 稼働状況サマリ
- `/tags` — タグ管理（ツリー表示: ドライバ > スキャングループ > タグ）
- `/publishers` — MQTT/OPC 配信設定
- `/logs` — ログビューア
- `/settings` — 全体設定

※ **ドライバ管理画面の役割はタグ管理画面へ統合する**。接続先の新規追加・確認・ドライバUI起動は `/tags` で扱い、独立した `/drivers` 一覧画面は持たない。

### タグ管理画面 UI 構成（ツリー階層表示）

タグ管理ページの中央ペインは、ツリー表示またはフラット一覧の 2 モードを切り替え可能とする。

**ツリーモード（推奨）**:

```text
+ Driver: postgres-main
  + ScanGroup: line1_sensors_1000ms (table: line1_sensors)
    - Tag: tag-line1-temp (temperature, f32)
    - Tag: tag-line1-pressure (pressure, f64)
  + ScanGroup: line1_system_5000ms (table: line1_system)
    - Tag: tag-line1-status (status, string)
+ Driver: slmp-device-a
  + ScanGroup: modbus_500ms
    - Tag: tag-device-a-input (input_register, f32)
```

- ドライバ行をクリック → 当該ドライバ配下のスキャングループを展開/折畳
- ドライバ行には `driver_id` と `driver_type` を併記する
- ドライバ行を選択 → 右ペインに接続先詳細を表示する
- スキャングループ行をクリック → 当該グループ配下のタグを展開/折畳
- スキャングループ行を選択 → 右ペインに ScanGroup 情報を表示する
- タグ行をクリック → 右ペインで詳細表示・編集

### タグ管理画面に統合する接続先管理

- ツリービュー未選択状態でも、中央ペイン上部の **「＋ 新規ドライバ」** を押せる。
- 「＋ 新規ドライバ」押下時は **ドライバ種別選択ダイアログ** を表示し、選択した種別のドライバ専用 UI を **別プロセス / 別ウィンドウ** で起動する。
- ドライバ専用 UI では以下を一連のフローで行う。
  1. 接続先情報の入力
  2. ScanGroup の追加
  3. タグの追加 / 編集
  4. 「確定」で接続先定義 + ScanGroup 一覧 + タグ一覧を本体へ返却
- 本体は返却結果を検証後に一括反映し、ツリービューへ以下の順で表示する。
  - ルート直下に接続先ノード（アイコン + 接続先名）
  - 配下に ScanGroup ノード
  - 配下にタグノード
- 既存接続先を選択した状態でタグ追加や編集を行う場合も、同じドライバ専用 UI を起動する。
- 接続先編集はタグ管理画面右ペインで接続情報を確認し、**「編集」操作で当該接続先のドライバ専用 UI を再起動する**。

## ダッシュボードの起動停止操作

- ダッシュボードには **「サーバ起動」** / **「サーバ停止」** ボタンを配置する。
- 「サーバ起動」で以下をまとめて開始する。
  - DriverManager 配下の有効ドライバ
  - PublisherManager 配下の有効パブリッシャ
- 「サーバ停止」で上記をまとめて停止する。
- タグ登録用 gRPC サーバは **別プロセスUIとの IPC 用途** のため常時起動とし、ダッシュボードの起動停止対象には含めない。
- Driver / Publisher の起動失敗はダッシュボード上のエラーメッセージで表示する。
- 将来的には Windows サービス等によるサービス駆動へ移行するが、当面は本体 UI から手動制御とする。

**フラットモード**:

```text
| Driver | ScanGroup | Table | Tag | DataType | Enabled |
| postgres-main | line1_sensors_1000ms | line1_sensors | temperature | f32 | ✓ |
| postgres-main | line1_sensors_1000ms | line1_sensors | pressure | f64 | ✓ |
| postgres-main | line1_system_5000ms | line1_system | status | string | ✓ |
| slmp-device-a | modbus_500ms | - | input_register | f32 | ✓ |
```

- テーブル表示で全タグを一覧表示
- タグ行をクリック → 右ペインで詳細表示・編集

### タグ管理画面の操作仕様

**ツリーの右クリックメニュー**:

- ドライバノード右クリック:
  - 「タグ追加」: 当該ドライバ UI を起動し、タグ追加モードで開く
  - 「全タグ編集」: 当該ドライバ UI を起動し、登録済みタグ一覧を編集
- タグノード右クリック:
  - 「編集」: 当該タグの接続先ドライバ UI を起動し、該当タグを選択した状態で開く
  - 「削除」: 確認ダイアログ後、本体側で削除（ドライバ UI 起動なし）
  - 「複製」: 同接続先内で複製（タグ ID は新規採番）

**右ペインのボタン**:

- 「編集」: 選択中タグの接続先ドライバ UI を起動
- 「削除」: 確認ダイアログ後、本体側で削除
- 「閉じる」: 詳細表示をクリア

**中央ペイン上部のツールバー**:

- 「＋ 新規ドライバ」: ドライバ種別選択ダイアログ → ドライバ UI 起動 → 新規接続先を作成
- 「＋ 新規タグ」: ドライバ（接続先）選択ダイアログを表示 → ドライバ UI 起動
- 「再読み込み」: タグ一覧を再取得

## タグ登録フロー

タグ登録は通常運転時の値取得とは別フローとする。

### 階層関係（重要）

```text
DriverType（postgres / slmp / joywatcher）
  └ Driver / Connection（接続先：例 postgres-server1, postgres-server2）
      └ ScanGroup（テーブル/読出単位）
          └ Tag（カラム/レジスタ等）
```

- 「ドライバ UI」は **接続先（Connection）単位** で起動する。
- 例：PostgreSQL の `postgres-server1` と `postgres-server2` は、それぞれ独立したドライバ UI セッションで管理される。
- ドライバ UI は、その接続先に紐づく **全タグの一覧管理** を担当する（個別タグ単位ではない）。

### 新規タグ追加フロー

1. 本体 UI で「＋ 新規タグ」ボタンをクリック。
2. **ドライバ（接続先）選択ダイアログ** を表示。
3. ユーザーが接続先を選択 → `launch_driver_ui(driver_id)` で対応するドライバ UI を **別プロセス・別ウィンドウ** で起動する。
4. ドライバ UI で接続設定を行い、接続先探索・候補生成を実施する。
5. ドライバ UI が登録済みタグも含めて一覧表示し、ユーザーがタグを追加・編集する。
6. ドライバ UI が「確定」時、**接続先の全タグを JSON で本体へ返却**する。
7. 本体がタグ ID 重複、タグ名重複、データ型、必須項目、ドライバ種別整合性を検証する。
8. 本体が `config/tags.toml` に **接続先単位で置き換え保存**する。

### 新規接続先追加フロー

1. 本体 UI のタグ管理画面で **「＋ 新規ドライバ」** をクリック。
2. ドライバ種別選択ダイアログを表示。
3. ユーザーが種別を選択 → 対応するドライバ UI を **別プロセス・別ウィンドウ** で起動する。
4. ドライバ UI で接続先情報を入力し、必要な ScanGroup とタグを追加する。
5. ドライバ UI が「確定」時、**接続先定義 + 全 ScanGroup + 全タグを JSON で本体へ返却**する。
6. 本体が `driver_id`、`driver_type`、接続設定、ScanGroup、タグ整合性を検証する。
7. 本体が `config/drivers.toml` と `config/tags.toml` を **同一トランザクション相当で更新**し、ツリービューを再構成する。

### 実装メモ（2026-05 時点）

- 本体 UI は「＋ 新規タグ」押下時に **ドライバ選択ダイアログ** を表示する。
- ドライバ選択後、`launch_driver_ui(driver_id)` で外部 UI を別プロセス起動し、
 返却された `output_json_path` を 1 秒間隔でポーリング監視する。
- 起動時に本体は **初期コンテキスト JSON** を生成し、ドライバUIへ `--input-json <path>` で渡す。
- JSON 生成検出後に `import_driver_ui_result` を実行し、成功時はタグ一覧を再読込する。
- 監視タイムアウト時は再実行導線を表示し、ページ遷移時は監視を中断する。

### 既存タグ編集フロー

1. 本体 UI のツリーでタグを選択（または右クリックメニュー）。
2. 右ペインの「編集」ボタン、またはツリーの右クリック → 「編集」。
3. 本体が対応する接続先のドライバ UI を別プロセス・別ウィンドウで起動（`driver_id` を渡す）。
4. ドライバ UI が当該接続先の接続設定・全タグを一覧表示する。
5. ユーザーがタグを編集する。
6. 以降は新規追加フローの 6〜8 と同様。

### 同時起動ポリシー

- 同一接続先のドライバ UI は **1 ウィンドウのみ**（多重起動禁止）。
- 異なる接続先（例：`postgres-server1` と `postgres-server2`）は、別ウィンドウで並列起動可能。
- 既に起動中の場合は、該当ウィンドウを前面化する。

### 重複取込防止について

- ドライバ UI は **接続先の全タグを一括返却** する仕様のため、本体は受信したタグセットで既存定義を置き換える。
- 新規接続先作成時も同様に、受信した接続先定義・ScanGroup・タグのセットを一括反映する。
- そのため `requestId` による重複取込防止は **必須ではない**（任意のメタ情報として保持）。
- 同時編集を防ぐため、ドライバ UI 起動中は本体側で当該接続先の編集ロックをかけることが望ましい。

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

## ドライバUI配置規約

本体の `launch_driver_ui(driver_id)` コマンドは、対象ドライバの `driver_type` を使って
本体配下の決まった位置からドライバUI実行ファイルを探索し、別プロセス起動する。

表記ルール:

- JSON では `driverType`（camelCase）
- TOML では `driver_type`（snake_case）
- 意味は同じ **DriverType** を指す

- 既定探索パス: `<app-root>/driver-ui/<driver_type>/registration-ui(.exe)`
- 互換探索: `<app-root>/driver-ui/<driver_type>/driver-ui(.exe)` など

### PostgreSQL ドライバUI（monorepo 分離後）

- 実装本体: `apps/driver-ui-postgres/`
- 静的画面資産: `apps/postgres/ui/assets/`
- 開発用ビルド/配置: `npm run driver-ui:dev`
- 生成バイナリ: `target/debug/driver_ui_postgres.exe`（workspace ルート）
- 配置先: `driver-ui/postgres/registration-ui.exe`

補足:

- ルート `dist/` は本体アプリ (`kt_iot_hub`) 用のフロントエンド出力先であり、PostgreSQL 登録UIとは別物である。
- PostgreSQL 登録UI は `apps/postgres/ui/tauri.conf.json` の `frontendDist = "./assets"` を通じて、`apps/postgres/ui/assets/` の HTML / CSS / JavaScript を読み込む。

### 配置スクリプト

PowerShell スクリプト `scripts/install-driver-ui.ps1` を使用する。

- 例: `npm run driver-ui:install -- -DriverType postgres -SourcePath C:/tools/postgres-tag-ui/postgres-tag-ui.exe`

### `drivers.toml` 設定例

```toml
[[driver]]
id = "postgres-main"
driver_type = "postgres"
enabled = true
host = "localhost"
port = 5432
database = "iot_hub"
username = "iot_user"
password = "iot_password"
```

## 受け渡し方式

- 初期実装: 一時 JSON ファイル
- 将来: gRPC / Named Pipe に統一

## タグ管理の階層モデル

タグ管理の正規階層は以下とする。

```text
DriverType（postgres / slmp / joywatcher）
  └ Connection（接続先定義）
      └ ScanGroup（読出し周期・取得単位）
          └ Tag（共通項目 + driver_spec）
```

- UI 表示上は「接続先起点」の表示を許可するが、内部モデルは上記階層を正本とする。
- `Tag.driver_id` は `Connection.id` を参照し、`Tag.scan_group_id` は `ScanGroup.id` を参照する。
- 読出し周期はタグ単位ではなく `ScanGroup` 単位で管理する。

### MQTT Topic 構造

MQTT 配信時のトピックは以下の階層構造に従う。

```text
<driver_id>/<scan_group_id>/<tag_name>
```

**例**:

- `postgres-main/line1_sensors_1000ms/temperature`
- `postgres-main/line1_system_5000ms/status`
- `slmp-device-a/modbus_500ms/pressure`

### タグ名の重複ルール

タグ名とスキャングループ名の重複は**許可**される。ただし、以下の条件を満たす場合のみとする：

- **同一スキャングループ内ではタグ名重複禁止**: `(driver_id, scan_group_id, tag_name)` の組み合わせは一意
- **異なるドライバまたは異なるスキャングループなら重複許可**: 同じ `tag_name = "temperature"` でも、ドライバやスキャングループが異なれば別タグとしてカウント

**スキャングループ名の重複**:

- 異なるドライバなら、同じ `scan_group_id`（例: `sensors_1000ms`）を複数ドライバで使用可能
- 同じドライバ内では `scan_group_id` は一意

**検証ルール**（本体側保存前チェック）:

1. `tag_id` は全体で一意（システム内永続ID）
2. `(tag.driver_id, tag.scan_group_id, tag.name)` は一意（既存定義との重複確認）
3. 同じ `(driver_id, scan_group_id)` 内に同名タグが存在しないこと
4. 同じ `driver_id` 内に同名 `scan_group_id` が存在しないこと

## 本体-登録プロセス連携プロトコル（初期版）

### 相互受け渡し（本体 ⇄ ドライバ）

本体とドライバは、タグ登録セッション中に相互にタグ情報を受け渡す。

- **本体 → ドライバUI（初期コンテキスト）**
  - 既存の接続先設定
  - 既存の ScanGroup 一覧
  - 既存のタグ一覧（編集時の初期値）
- **ドライバUI → 本体（確定結果）**
  - 接続先定義（新規または更新）
  - ScanGroup 一覧
  - タグ一覧

正式テンプレート（サンプル）:

- `docs/templates/driver-ui-request-template.json`（本体 → ドライバ）
- `docs/templates/driver-ui-response-template.json`（ドライバ → 本体）
- `docs/templates/driver-ui-request-fields.md`（`--input-json` フィールド仕様: 必須/任意）
- `docs/templates/driver-ui-response-fields.md`（`--output-json` フィールド仕様: 必須/任意）

一時 JSON ファイル連携では、以下のメタ情報を必須とする。

- `schemaVersion`: スキーマ互換判定用
- `requestId`: 登録セッション識別子（重複取込防止）
- `generatedAt`: 生成時刻（監査・再実行判断）
- `driver`: 新規接続先時の接続定義（`id`, `driverType`, `enabled`, `settings` を含む）

本体側受信時の最低バリデーション:

1. `schemaVersion` が対応範囲内であること
2. 新規接続先時は `driver.id` と `driverType` が存在すること
3. `driver.driverType` と `driverSpec.kind` が一致すること
4. `scanGroups[].id` と `tags[].driverSpec.scanGroup` が整合すること
5. タグ ID/名称重複がないこと（既存定義との衝突含む）
6. 参照不能な接続先・スキャングループがないこと

### 返却JSON仕様のコード化（実装）

- Rust 側では以下の型で返却JSONを受け付ける。
  - `packages/protocol-rs/src/lib.rs`
    - `DriverUiImportPayload`
    - `DriverUiDriverPayload`
    - `DriverUiScanGroupPayload`
    - `DriverUiTagPayload`
- 本体側の `src-tauri/src/commands/driver_ui_protocol.rs` は互換用 re-export とし、実体定義は持たない。
- 返却JSONでは `driver` ブロックを必須とし、`id` / `driverType` / `settings` を常に出力する。

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
- PostgreSQL 登録UIは **3ステップ（接続先設定 → グループ設定 → 確認・保存）** を基本とする。
- PostgreSQL のグループ設定ステップは **2カラム** とし、左にテーブル一覧 / 登録済みグループ、右に選択中テーブルの編集フォームを配置する。
- 新規登録と既存接続先編集は同一UIで扱い、見出しと初期値でモードを区別する。

## タグ登録結果の概念例

```json
{
  "schemaVersion": 1,
  "driver": {
    "id": "postgres-main",
    "driverType": "postgres",
    "enabled": true,
    "settings": {
      "host": "localhost",
      "port": 5432,
      "database": "iot_hub",
      "username": "iot_user",
      "password": "******"
    },
    "scanGroups": [
      {
        "id": "sensors_1000ms",
        "table": "sensors",
        "timestampColumn": "created_at",
        "scanRateMs": 1000,
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
              "valueColumn": "temperature"
            }
          }
        ]
      }
    ]
  }
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
- `driver`: 接続先ID（`DriverType` ではない）

## 保存前チェックリスト（本体側）

1. `scan_group.id` の重複がない
2. `tag.id` の重複がない（全体）
3. `tag.scan_group` が存在する
4. `tag.driver` と `scan_group.driver` が一致する
5. `driver_spec.value_column` が空でない
6. 同一 `(driver_id, scan_group_id)` 内に同名タグ（`tag.name`）が存在しないこと（既存定義を含む）

## SLMP 登録 UI（将来）

SLMP ドライバも同様に、接続先設定 → アドレス設定 → タグ登録とする。
アドレスは連続割付けできるようにする。

## JoyWatcher 登録 UI（将来）

JoyWatcher の DLL は JoyWatcher のネットワークミドルウェアに接続する想定。
JoyWatcher 側のタグ取得後、本体タグ定義へマッピングする登録 UI を提供する。
