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

[[scan_group]]
id = "sensors_1000ms"
driver = "postgres-main"
table = "sensors"
timestamp_column = "created_at"
scan_rate_ms = 1000
```

## `config/drivers.toml` 例

```toml
[[driver]]
id = "postgres-main"
kind = "postgres"
host = "127.0.0.1"
port = 5432
database = "plant"
user = "reader"
password = "reader_password"

[[driver]]
id = "joywatcher-1"
kind = "joywatcher"
endpoint = "localhost"
```

## `config/publishers.toml` 例

```toml
[[publisher]]
id = "mqtt-local"
kind = "mqtt"
broker = "tcp://127.0.0.1:1883"
client_id = "kt_iot_hub"
topic_template = "plant/{tag.name}"
qos = 1
```

## 設計ルール

- 秘匿情報（パスワード・APIキー）は当面 TOML に平文で記載する。
- 将来、外部ネットワーク接続時は暗号化または OS キーリングを検討する。
- スキーマは Rust 側で `serde` 定義 + JSON Schema を自動生成し、UI のバリデーションと AI エージェント生成のガイドに使う。
- ドライバ固有の接続先情報は `driver_spec` に格納する。
- 本体は `driver_spec.kind` と `driver` の整合性のみを検証し、詳細解釈は各ドライバへ委譲する。
- PostgreSQL のように複数タグを同一テーブルから読む場合は、`scan_group` でテーブル単位の読出し周期を管理する。
- 編集時はアトミックライト（一時ファイル → rename）で破損を防止する。
- バージョンフィールド（`schema_version`）をファイル先頭に持たせ、将来のマイグレーションに備える。
