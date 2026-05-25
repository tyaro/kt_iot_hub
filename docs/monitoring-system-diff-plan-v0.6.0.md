# kt_iot_hub 監視システム強化 実装差分計画（v0.6.0）

- 作成日: 2026-05-26
- 対象: `docs/monitoring-system-hardening-plan-v0.6.0.md` に基づく、実装時の具体的な差分計画
- 目的: **設定 DTO / TOML 読み書き / keyring / TLS / 再起動制御** を安全に切り替えるための、変更単位を明示した実施メモ

> 本書は「何を実装するか」ではなく、「**どこをどう変えるか**」を固定するための差分計画である。既存の JSON / TOML / gRPC / Tauri コマンドの互換性は維持する。

---

## 0. 変更方針

1. **互換性維持を最優先**
   - 既存 `password` / `ssl_mode` / `retain` / `topic` の意味を壊さない。
   - 旧設定が読めることを先に保証する。
2. **保存形式の拡張は段階的に行う**
   - まず読み込み対応。
   - 次に保存時の新形式出力。
   - 最後に UI から新項目を編集可能にする。
3. **秘匿情報は TOML から分離**
   - TOML は参照キーのみ保持。
   - 実値は OS キーリングへ保存。
4. **TLS は既定無効**
   - 接続失敗時は明示的なエラーを返す。
   - 自己署名 / 社内 CA / クライアント証明書を後から足せるようにする。

---

## 1. 差分対象一覧

### 1.1 Rust 本体

- `core/src-tauri/src/commands/driver/crud/logic.rs`
- `core/src-tauri/src/commands/publisher/crud.rs`
- `core/src-tauri/src/commands/driver/toml_io.rs`
- `core/src-tauri/src/commands/publisher/toml_io.rs`
- `core/src-tauri/src/publishers/mqtt.rs`
- `core/src-tauri/src/drivers/mod.rs`
- `drivers/postgres/driver/src/postgres_poller.rs`
- `core/src-tauri/src/grpc/driver_runtime.rs`
- `core/src-tauri/src/commands/dto/*`

### 1.2 設定・ドキュメント

- `docs/config-spec.md`
- `docs/monitoring-system-hardening-plan-v0.6.0.md`
- `docs/monitoring-system-review-2026-05.md`

### 1.3 UI

- 設定画面
- ドライバ状態表示
- MQTT モニタ

---

## 2. DTO 差分

### 2.1 `driver` DTO

#### 2.1.1 driver DTO 追加項目

- `password_key: Option<String>`
- `tls_enabled: bool`
- `tls_ca_path: Option<String>`
- `tls_client_cert_path: Option<String>`
- `tls_client_key_path: Option<String>`
- `connect_timeout_ms: Option<u64>`
- `statement_timeout_ms: Option<u64>`
- `auto_restart: bool`
- `max_restart_per_minute: Option<u32>`

#### 2.1.2 driver DTO 変更方針

- 既存 `password` は読み込み互換のため残す。
- 書き込み時は `password_key` がある場合にキーリング保存を優先する。
- `ssl_mode` は現状維持しつつ、`tls_enabled` と併存させる。

### 2.2 `publisher` DTO

#### 2.2.1 publisher DTO 追加項目

- `password_key: Option<String>`
- `tls_enabled: bool`
- `tls_ca_path: Option<String>`
- `tls_client_cert_path: Option<String>`
- `tls_client_key_path: Option<String>`
- `reconnect_backoff_ms: Option<u64>`
- `max_reconnect_backoff_ms: Option<u64>`

#### 2.2.2 publisher DTO 変更方針

- MQTT は既定の平文接続を維持しつつ、TLS 情報を追加する。
- `qos` / `retain` / `topic` の意味は変更しない。

### 2.3 共通エラー

- `ErrorResponse` は追加フィールドを増やさず、既存の `code` / `error` を維持する。
- 新設定の不正値は既存のエラー表現に載せる。

---

## 3. TOML 読み書き差分

### 3.1 drivers.toml

#### 3.1.1 drivers.toml 読み込み

- 既存 `password` を読めるようにする。
- `password_key` があれば keyring から取得する。
- `tls_enabled=false` の場合は TLS 設定を無視する。
- 未指定値は既存のデフォルトを使う。

#### 3.1.2 drivers.toml 書き込み

- `password` ではなく `password_key` を保存する。
- TLS 関連の空項目は省略可能にするが、UI で編集可能にするため DTO では保持する。
- `auto_restart` / `max_restart_per_minute` を保存する。

### 3.2 publishers.toml

#### 3.2.1 publishers.toml 読み込み

- `password` と `password_key` の両方を受け入れる。
- TLS 設定は未指定時に無効扱いとする。

#### 3.2.2 publishers.toml 書き込み

- `password_key` を優先保存する。
- `tls_enabled` / `tls_ca_path` / `tls_client_cert_path` / `tls_client_key_path` を保存する。
- 再接続バックオフ値を保存する。

### 3.3 移行ルール

1. 旧 TOML を読めること。
2. 保存し直した時に新形式へ寄ること。
3. 新形式で保存した TOML を旧版が読めない場合は、**旧版互換が必要な範囲を別途明示**する。

---

## 4. keyring 差分

### 4.1 保存キー命名

- driver: `driver/<driver_id>/password`
- publisher: `publisher/<publisher_id>/password`

### 4.2 差分方針

- 既存の保存処理に「キーリング保存」を追加する。
- TOML は参照キーのみを残す。
- 空文字入力は「既存維持」ルールとして従来互換を保つ。

### 4.3 フォールバック

- keyring 取得失敗時は明示エラーとする。
- 既存平文 `password` が残っている場合は、移行期間のみ読み取り可とする。

---

## 5. TLS 差分

### 5.1 MQTT

- `MqttOptions` の接続設定に TLS 追加。
- `tls_enabled=true` のときのみ `Transport::Tls(...)` を使う。
- CA / クライアント証明書の指定が無い場合は、エラーまたは明示的な警告にする。
- `event_loop` の再接続時にバックオフを入れる。

### 5.2 PostgreSQL

- `tokio_postgres::connect(..., NoTls)` の固定をやめる。
- `sslmode` と `tls_enabled` の関係を明示する。
- `connect_timeout_ms` / `statement_timeout_ms` を反映する。
- `statement_timeout` は接続直後に `SET` する方針を基本とする。

---

## 6. ドライバ死活監視差分

### 6.1 `DriverProcessManager`

- `spawn` 後に `wait()` を監視するタスクを追加する。
- 終了コードを runtime 状態へ反映する。
- `auto_restart=true` の場合のみ再起動する。

### 6.2 再起動ポリシー

- `max_restart_per_minute` を超えたら停止する。
- バックオフは指数増加とする。
- UI には「最終終了理由」「再起動回数」を出せるようにする。

---

## 7. UI 差分

### 7.1 設定画面

- TLS の有効/無効切替を追加する。
- keyring 保存であることを明示する。
- タイムアウト値と再起動回数を編集可能にする。

### 7.2 エラー表示

- keyring 未保存、証明書未設定、TLS 接続失敗を区別して表示する。
- 過負荷時のバックオフ動作が分かる表示を追加する。

---

## 8. テスト差分

### 8.1 追加したいテスト

- `password` / `password_key` の相互読解テスト
- TLS 無効時の既存互換テスト
- TLS 有効時の設定解釈テスト
- keyring 保存・復元の単体テスト
- ドライバ自動再起動回数上限のテスト
- `statement_timeout` / `connect_timeout` の反映テスト

### 8.2 変更しないこと

- JSON / TOML の既存表現を壊さない
- gRPC proto を変更しない
- Tauri コマンド名を変更しない

---

## 9. 実装順序

1. DTO に新項目を追加
2. 読み込み側を先に対応
3. keyring 保存を追加
4. 書き込みを新形式へ寄せる
5. TLS 対応を MQTT / PostgreSQL に実装
6. ドライバ監視と再起動制御を追加
7. UI とエラー表示を更新
8. テストと文書を整える

---

## 10. 受け入れ条件

- 既存設定ファイルが壊れない。
- 旧形式 TOML を読み込める。
- TLS はオプションで有効化できる。
- 秘匿情報は TOML 平文に残らない。
- ドライバ異常終了を本体が検知できる。
- UI から新設定を操作できる。
