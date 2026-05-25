# kt_iot_hub 監視システム強化 実装メモ（v0.6.0）

- 作成日: 2026-05-26
- 目的: v0.6.0 実装中に見つかった、差分計画書に書ききれない論点を残す

---

## 実装中に見つかった論点

### N-01: keyring crate の `Entry::new` は `Result` を返す

- 実装時に `keyring` crate 3.6.3 を確認したところ、`Entry::new(...)` は `Result<Entry, Error>` を返した。
- そのため、単純な `Entry::new(...).get_password()` ではなく、`Result` を解いてから `get_password` / `set_password` / `delete_password` を呼ぶ必要があった。
- 影響ファイル:
  - `core/src-tauri/src/commands/secret_store.rs`

### N-02: MQTT publisher / monitor も同時に秘密参照を切り替える必要がある

- `drivers.toml` / `publishers.toml` の保存だけを先に変えても、実行時の MQTT publisher と MQTT monitor が旧 `password` を直接参照しているため、秘密分離が完了しない。
- そのため以下を同時に更新した。
  - `core/src-tauri/src/publishers/mqtt.rs`
  - `core/src-tauri/src/commands/subscriber/monitor/command.rs`
  - `core/src-tauri/src/commands/publisher/crud.rs`

### N-03: driver/publisher の保存時に secret 移送と TOML 更新の順序をそろえる必要がある

- 既存値の維持ルールと rename 変更が重なるため、`password_key` の新旧切替と keyring 保存/削除の順序を明示して扱う必要がある。
- 今回は、保存前に秘密値を解決し、保存先 key を決めたうえで TOML に `password_key` を書く流れに統一した。

### N-04: 既存 JSON 互換のため、新規 request DTO には snake_case alias が必要だった

- `SaveDriverRequest` / `SavePublisherRequest` の新規フィールドは `rename_all = "camelCase"` だけでは古い snake_case JSON を拾えず、`alias` を足して互換を維持する必要があった。
- これを入れないと、`password_key` や `tls_enabled` などの既存フィクスチャが `None` / `false` に落ちてしまい、回帰テストが失敗した。
- 影響ファイル:
  - `core/src-tauri/src/commands/dto/driver.rs`
  - `core/src-tauri/src/commands/dto/publisher.rs`
  - `core/src-tauri/src/commands/dto/serde_snapshot_tests.rs`

### N-05: PostgreSQL TLS の初期実装は CA ファイル優先で、クライアント証明書ペアは後続対応にした

- `native-tls` / `postgres-native-tls` 構成では、CA PEM の取り込みと TLS 有効化は実装できた。
- 一方、`tls_client_cert_path` / `tls_client_key_path` のような分離したクライアント証明書ペアはこの構成では直ちに使いづらいため、現段階では未接続とした。
- 将来的に mutual TLS を使う場合は、PKCS#12 変換か rustls ベースへの切り替えを検討する。
- 影響ファイル:
  - `packages/driver-ui-host/src/postgres.rs`
  - `drivers/postgres/driver/src/postgres_poller.rs`

### N-06: ドライバ再起動監視は、プロセス一覧と再起動履歴を manager 側に保持しないと安全に扱えなかった

- 自動再起動を子プロセスの監視タスクに持たせる場合、`start_driver` の呼び出し元だけでは stop 競合を判定しきれなかった。
- そのため、`DriverProcessManager` に `processes` と別に再起動履歴を持たせ、監視タスクから共有ハンドル経由で状態を確認する構成にした。
- 監視ループは、終了検知後もすぐに再起動せず、stop 要求でエントリが消えたかを再確認してから子プロセスを再生成する。
- 影響ファイル:
  - `core/src-tauri/src/drivers/mod.rs`
  - `core/src-tauri/src/commands/runtime.rs`
  - `core/src-tauri/src/commands/driver/runtime_sync.rs`
  - `core/src-tauri/src/commands/driver/transfer_impl.rs`
