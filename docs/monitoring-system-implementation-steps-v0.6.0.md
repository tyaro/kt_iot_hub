# kt_iot_hub 監視システム強化 実装手順書（v0.6.0）

- 作成日: 2026-05-26
- 目的: `docs/monitoring-system-diff-plan-v0.6.0.md` を前提に、**初手で触るファイル順**を短く固定する
- 方針: 変更は小さく、まず読み込み互換を確保し、その後に保存形式と UI を追従させる

> この手順書は「実装開始時の迷いを減らす」ことだけに絞る。詳細な差分は [実装差分計画](./monitoring-system-diff-plan-v0.6.0.md) を参照する。

---

## 1. 最初に触る順番

### 1.1 まず確認する

1. [docs/monitoring-system-hardening-plan-v0.6.0.md](./monitoring-system-hardening-plan-v0.6.0.md)
2. [docs/monitoring-system-diff-plan-v0.6.0.md](./monitoring-system-diff-plan-v0.6.0.md)
3. [docs/config-spec.md](./config-spec.md)

### 1.2 最初に編集するファイル順

1. `docs/config-spec.md`
   - 先に設定項目の名前と既定値を固定する。
   - `password_key` / `tls_enabled` / `connect_timeout_ms` / `auto_restart` の意味を明文化する。
2. `core/src-tauri/src/commands/dto/*`
   - `driver` / `publisher` DTO に新項目を追加する。
   - 既存フィールドは壊さない。
3. `core/src-tauri/src/commands/driver/toml_io.rs`
4. `core/src-tauri/src/commands/publisher/toml_io.rs`
   - 先に読み込み互換を入れる。
   - `password` と `password_key` の両方を読めるようにする。
5. `core/src-tauri/src/commands/driver/crud/logic.rs`
6. `core/src-tauri/src/commands/publisher/crud.rs`
   - 保存時に keyring 参照へ寄せる。
   - 空文字の既存維持ルールはここでまとめて扱う。
7. `core/src-tauri/src/publishers/mqtt.rs`
8. `drivers/postgres/driver/src/postgres_poller.rs`
   - TLS と timeout を実装する。
   - 既定無効のまま、オプションとして足す。
9. `core/src-tauri/src/drivers/mod.rs`
   - 子プロセス死活監視と自動再起動を入れる。
10. `core/src/lib/components/**` と `core/src/lib/ipc/**`
    - UI の設定項目とエラー表示を追従させる。

---

## 2. 実装の進め方

### 2.1 読み込みを先に通す

- 旧 TOML が読めることを最優先にする。
- 既存 `password` を落とさず、`password_key` を追加で扱う。
- TLS は未指定時に無効のままにする。

### 2.2 次に保存を書き換える

- 保存時は `password_key` を優先する。
- 旧形式を開いて保存し直したときに、新形式へ寄ることを確認する。

### 2.3 その後に通信まわり

- MQTT と PostgreSQL に TLS を追加する。
- PostgreSQL に connect / statement timeout を入れる。
- MQTT event loop とドライバ再起動にバックオフを入れる。

### 2.4 最後に UI と検証

- 設定画面に新項目を出す。
- エラー文言を keyring / TLS / timeout で分ける。
- `cargo fmt` / `cargo test` / `npm run check` を通す。

---

## 3. 途中で止める基準

- 旧 TOML が読めない場合はその時点で止める。
- `password` の扱いが壊れたら保存実装へ進まない。
- TLS の設定名が固まる前に UI を触らない。
- ドライバ再起動が無限ループになる実装は入れない。

---

## 4. 実装後に必ず見るもの

- 旧設定ファイルが壊れていないか
- `password` と `password_key` の両対応ができているか
- TLS はオプションとして動くか
- 自動再起動の上限が効くか
- UI から状態が読めるか
