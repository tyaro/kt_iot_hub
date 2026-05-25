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
password_key = "driver/postgres-main/password"
ssl_mode = "disable"
tls_enabled = false
tls_ca_path = "C:/certs/root-ca.pem"
tls_client_cert_path = ""
tls_client_key_path = ""
connect_timeout_ms = 5000
statement_timeout_ms = 10000
auto_restart = true
max_restart_per_minute = 3
```

### ドライバ設定（v0.6.0）

- `password_key`: OS キーリングへ保存した秘密値の参照キー。
- `ssl_mode`: 既存互換用（`disable` など）。段階移行中は読み取りを維持。
- `tls_enabled`: TLS 利用の有効/無効（既定: `false`）。
- `tls_ca_path`: CA 証明書のパス。自己署名証明書や社内 CA を想定。
- `tls_client_cert_path` / `tls_client_key_path`: クライアント証明書認証を使う場合のみ指定。
- `connect_timeout_ms`: 接続確立の上限時間（未指定ならドライバ既定値）。
- `statement_timeout_ms`: クエリ実行の上限時間（未指定ならドライバ既定値）。
- `auto_restart`: ドライバ子プロセスの自動再起動有無（既定: `true`）。
- `max_restart_per_minute`: 短時間リトライの上限回数（未指定で上限なし）。

### JoyWatcher 例

```toml
[[driver]]
id = "joywatcher-1"
driver_type = "joywatcher"
enabled = true
endpoint = "localhost"
user_id = 0
password_key = "driver/joywatcher-1/password"
auto_restart = true
max_restart_per_minute = 3
```

### 後方互換について

- 移行期間中は `password`（平文）も読めるが、保存時は `password_key` 利用を推奨する。
- 既存運用との互換維持のため `ssl_mode` は残す。新規設定は `tls_enabled` を基準にする。

## `config/publishers.toml` 例

```toml
[[publisher]]
id = "mqtt-main"
publisher_type = "mqtt"
enabled = false
broker = "localhost"
port = 8883
username = ""
password_key = "publisher/mqtt-main/password"
client_id = "kt_iot_hub"
qos = 1
retain = false
topic = "plant"
tls_enabled = true
tls_ca_path = "C:/certs/root-ca.pem"
tls_client_cert_path = ""
tls_client_key_path = ""
reconnect_backoff_ms = 500
max_reconnect_backoff_ms = 30000
```

### MQTT パブリッシャ設定（v0.6.0）

- `qos`: 0 / 1 / 2
- `retain`: retain フラグ（省略時 `false`）
- `topic`: MQTT トピックのベースパス（省略時は空文字）
- `password_key`: OS キーリング参照キー
- `tls_enabled`: TLS を使うかどうか（既定: `false`）
- `tls_ca_path`: ブローカー証明書検証用 CA
- `tls_client_cert_path` / `tls_client_key_path`: 必要時のみ指定
- `reconnect_backoff_ms` / `max_reconnect_backoff_ms`: MQTT 再接続バックオフ制御
- `publish_mode_default`: 既定配信モード（`scan_interval` / `on_change`）
- `publish_mode_by_driver` / `publish_mode_by_scan_group`: 上書き配信モード

### publish topic 仕様

- 実際の publish topic は `<topic>/<driver_id>/<scan_group_id>/<tag_name>`
- 例: `plant/postgresql/bte1w/w0400`
- `topic` が空文字の場合は `<driver_id>/<scan_group_id>/<tag_name>`
- payload はタグ値そのもののスカラー値を publish する
- 起動時の自動開始は `runtime.toml` の `auto_start_runtime_services` で一括制御する
- ダッシュボードの開始ボタンでは、設定済みのドライバ / パブリッシャを起動する

## 設計ルール

- 秘匿情報（パスワード・APIキー）は v0.6.0 以降、OS キーリングへ分離する。TOML には参照キーのみを保持する。
- 既存設定との互換性のため、移行期間は `password` の平文項目も読めるようにする。
- TLS は既定では無効とし、必要時のみ `tls_enabled = true` と CA / 証明書パスを設定する。
- スキーマは Rust 側で `serde` 定義 + JSON Schema を自動生成し、UI のバリデーションと AI エージェント生成のガイドに使う。
- ドライバ固有の接続先情報は `driver_spec` に格納する。
- 本体は `driver`（接続先ID）と `scan_group` の参照整合を検証し、`driver_spec` の詳細解釈は各ドライバへ委譲する。
- PostgreSQL のように複数タグを同一テーブルから読む場合は、`scan_group` でテーブル単位の読出し周期を管理する。
- 本体 UI から変更できる ScanGroup 項目は当面 `scan_rate_ms` のみとし、構造変更はドライバ UI 側で扱う。
- ドライバ子プロセスには `auto_restart` / `max_restart_per_minute` を設定できるようにし、異常終了時の自動復旧を制御する。
- 編集時はアトミックライト（一時ファイル → rename）で破損を防止する。
- バージョンフィールド（`schema_version`）をファイル先頭に持たせ、将来のマイグレーションに備える。
- 登録プロセス取込時は、全検証成功後に一括反映し、部分成功を許可しない。
