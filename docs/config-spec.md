# 設定ファイル仕様

設定は TOML ファイルを正本（Source of Truth）とする。

## `config/tags.toml` 例

```toml
[[tag]]
id = "tag-0001"
name = "line1.temperature"
data_type = "f32"
unit = "degC"
driver = "postgres-main"
enabled = true
comment = "ライン1温度センサ"

[tag.driver_spec]
kind = "postgres"
table = "sensors"
value_column = "temperature"
timestamp_column = "created_at"
scan_group = "sensors_1000ms"

[[tag]]
id = "tag-0002"
name = "plc.tank.level"
data_type = "i32"
unit = "L"
driver = "joywatcher-1"
enabled = true

[tag.driver_spec]
kind = "joywatcher"
node = "PLC1"
tag_path = "Line1/Tank/Level"
native_tag_id = 101

[[scan_group]]
id = "sensors_1000ms"
driver = "postgres-main"
table = "sensors"
timestamp_column = "created_at"
scan_rate_ms = 1000
```

### タグ管理の正規階層

`tags.toml` の意味モデルは以下を正本とする。

```text
DriverType（postgres / slmp / joywatcher）
  └ Connection（drivers.toml の driver.id）
      └ ScanGroup（scan_group）
          └ Tag（tag）
```

- `tag.driver` は接続先 ID（`driver.id`）を参照する。
- `tag.scan_group` は `scan_group.id` を参照する。
- 読出し周期は `tag` ではなく `scan_group.scan_rate_ms` で管理する。

### `tags.toml` プロファイル例

#### 最小構成（PoC）

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
driver_spec = { kind = "postgres", value_column = "temperature" }
enabled = true
```

#### 標準構成（推奨）

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
driver_spec = { kind = "postgres", value_column = "temperature" }
enabled = true
metadata = { unit = "degC", comment = "ライン1温度" }

[[tag]]
id = "tag-line1-pressure"
name = "Line1 Pressure"
data_type = "f64"
driver = "postgres-main"
scan_group = "line1_sensors_1000ms"
driver_spec = { kind = "postgres", value_column = "pressure" }
enabled = true
metadata = { unit = "kPa", comment = "ライン1圧力" }

[[tag]]
id = "tag-line1-status"
name = "Line1 Status"
data_type = "string"
driver = "postgres-main"
scan_group = "line1_system_5000ms"
driver_spec = { kind = "postgres", value_column = "status" }
enabled = true
metadata = { comment = "運転状態" }
```

#### 大量登録構成（命名規約前提）

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
driver_spec = { kind = "postgres", value_column = "temperature" }
enabled = true

[[tag]]
id = "plant-a-sensors-humidity"
name = "plant_a_humidity"
data_type = "f32"
driver = "postgres-main"
scan_group = "plant_a_sensors_1000ms"
driver_spec = { kind = "postgres", value_column = "humidity" }
enabled = true
```

### 命名規約（推奨）

- `scan_group.id`: `<site>_<table>_<rate>ms`
- `tag.id`: 不変な永続ID
- `tag.name`: UI 表示名（変更可）
- `driver`: 接続先ID（種別名ではない）

### 保存前チェックリスト（本体）

1. `scan_group.id` 重複なし
2. `tag.id` 重複なし
3. `tag.scan_group` が存在
4. `tag.driver` と `scan_group.driver` が一致
5. `driver_spec.kind` と接続先 `kind` が一致
6. PostgreSQL では `driver_spec.value_column` が必須

### `scan_group.scan_rate_ms` の更新ルール

`scan_group.scan_rate_ms` は `tags.toml` 上の正本値であり、既存 ScanGroup に対する周期変更は
本体アプリから以下の単位で実施できる。

- **単体更新**: `(driver, scan_group.id)` をキーに 1 件の `scan_rate_ms` を更新する
- **接続先一括更新**: `driver` をキーに、その接続先配下の全 `scan_group.scan_rate_ms` を同一値へ更新する

更新時の仕様:

1. 変更対象は `scan_rate_ms` のみとする
2. `scan_group.id` / `driver` / `table` / `timestamp_column` / `node` / `schema` は変更しない
3. `scan_group.id` はドライバ横断で重複し得るため、単体更新の識別子は **`driver + scan_group.id`** とする
4. 保存は既存のアトミックライト方式（一時ファイル → rename）で行う
5. メモリ上の `scan_groups` と `tags.toml` を同一トランザクション相当で同期する

バリデーション:

- `scan_rate_ms` は 100ms 以上の整数とする
- 単体更新時に対象 `(driver, scan_group.id)` が存在しない場合はエラーとする
- 一括更新時に対象 `driver` 配下の ScanGroup が 0 件の場合はエラーとする

## `config/drivers.toml` 例

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
ssl_mode = "disable"

[[driver]]
id = "joywatcher-1"
driver_type = "joywatcher"
enabled = true
endpoint = "localhost"
user_id = 0
password = ""
```

## `config/publishers.toml` 例

```toml
[[publisher]]
id = "mqtt-main"
publisher_type = "mqtt"
enabled = false
broker = "localhost"
port = 1883
username = ""
password = ""
client_id = "kt_iot_hub"
qos = 1
retain = false
topic = "plant"
```

### MQTT パブリッシャ設定補足

- `enabled`: アプリ起動時にこの MQTT パブリッシャを自動開始するか。`true` の場合、起動時にドライバと MQTT 配信を開始する
- `qos`: 0 / 1 / 2
- `retain`: retain フラグ（省略時 `false`）
- `topic`: MQTT トピックのベースパス（省略時は空文字）
- 実際の publish topic は `<topic>/<driver_id>/<scan_group_id>/<tag_name>`
- 旧形式は `<topic>/<driver_id>/tags/<scan_group_id>/<tag_name>` だったが、現在は `tags` セグメントを挟まない
- 例: `plant/postgresql/bte1w/w0400`
- `topic` が空文字の場合は `<driver_id>/<scan_group_id>/<tag_name>`
- payload はタグ値そのもののスカラー値を publish する
- ダッシュボードの開始ボタンでは `enabled` の状態に関わらず、設定済みのドライバ / パブリッシャを起動する

## 設計ルール

- 秘匿情報（パスワード・APIキー）は当面 TOML に平文で記載する。
- 将来、外部ネットワーク接続時は暗号化または OS キーリングを検討する。
- スキーマは Rust 側で `serde` 定義 + JSON Schema を自動生成し、UI のバリデーションと AI エージェント生成のガイドに使う。
- ドライバ固有の接続先情報は `driver_spec` に格納する。
- 本体は `driver`（接続先ID）と `scan_group` の参照整合を検証し、`driver_spec` の詳細解釈は各ドライバへ委譲する。
- PostgreSQL のように複数タグを同一テーブルから読む場合は、`scan_group` でテーブル単位の読出し周期を管理する。
- 本体 UI から変更できる ScanGroup 項目は当面 `scan_rate_ms` のみとし、構造変更はドライバ UI 側で扱う。
- 編集時はアトミックライト（一時ファイル → rename）で破損を防止する。
- バージョンフィールド（`schema_version`）をファイル先頭に持たせ、将来のマイグレーションに備える。
- 登録プロセス取込時は、全検証成功後に一括反映し、部分成功を許可しない。
