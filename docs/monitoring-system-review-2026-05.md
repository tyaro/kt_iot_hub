# kt_iot_hub 監視システム観点 改善候補レビュー

- 作成日: 2026-05-26
- 対象リビジョン: ワークスペース現状（`main` 相当）
- 観点: **オフライン監視システムとして本番運用に耐えるか**
  - 信頼性（落ちない / 落ちても回復する）
  - 通信相手への配慮（DB / OPC 系 / MQTT ブローカーへ過負荷をかけない）
  - セキュリティ（オフライン LAN 内でも妥当な最低限）
  - 観測性（異常が見える）
- 比較先: 既存の総合評価は [docs/evaluation-2026-05.md](./evaluation-2026-05.md) を参照。本レビューは「監視システムとして残っている改善ポイント」を抽出する目的に絞る。

> 本書は**改善候補の洗い出し**であり、優先度判断や着手は別途検討。実装は計画化（`docs/refactor-plan.md` 追補 or 個別 issue 化）してから行う想定。

---

## 0. 結論サマリ

| # | 改善候補 | 影響 | 重要度 |
| --- | --- | --- | --- |
| M-01 | ドライバプロセスの異常終了監視・自動再起動なし | データ収集が**無音で停止**し得る | 高 |
| M-02 | MQTT Publisher の **TLS / クライアント証明書未対応** | ブローカー認証情報が平文で流れる | 高（LAN でも） |
| M-03 | PostgreSQL ドライバの **`sslmode=disable` 既定 / `NoTls` 固定** | DB 認証情報が平文で流れる | 高 |
| M-04 | パスワード等**秘匿情報を `*.toml` に平文保存**（規約 §1.5 違反） | 設定ファイル流出時に即漏えい | 高 |
| M-05 | PostgreSQL クエリに **statement_timeout / 接続タイムアウトが無い** | DB 側に長時間ロック・コネクション滞留を誘発 | 中 |
| M-06 | 周期遅延検知はあるが **遅延時のバックオフ・取りこぼし通知が UI に出ない** | 過負荷時に通信先を更に叩き続ける | 中 |
| M-07 | MQTT Publisher の **publish キューが満杯時に丸ごとドロップしない実装** で、`broadcast` が lag した分は捨てる一方、`mpsc` 詰まりは送信側の `await` で遅延伝搬する | バックプレッシャ挙動が混在し読みにくい | 中 |
| M-08 | MQTT トピックに **タグ名（任意文字列）をそのまま使用**、`+` `#` 等 MQTT 予約・非 ASCII の正規化不足 | 衝突・購読不能 | 中 |
| M-09 | MQTT モニタが `#` / `$SYS/#` を購読でき、**メッセージ蓄積はメモリのみ・上限 200 件固定** | 監視先ブローカー / 自プロセスのメモリ負荷 | 中 |
| M-10 | gRPC サーバ（`127.0.0.1:55051`）に**認証が無い** | 同一ホスト上の他プロセスから接続可能 | 中 |
| M-11 | アプリ起動時 / 設定再読込時に **`tags.toml` をフルロード**して `TagRegistry` に積むが、**変更検知やバリデーション一括レポートが弱い** | 大量タグ環境で起動遅延 / 部分破損で `warn!` のみ | 中 |
| M-12 | ドライバ→本体 への `TagValueMessage.timestamp` は空文字許容で、本体が **受信時刻で補完**する（JoyWatcher は常に空） | 「いつ実機側で取得した値か」が失われる | 中 |
| M-13 | ログは `tracing` で出力されるが、**ローテーション / 上限管理が無い**（`AppLogMakeWriter` のサイズ制御要確認） | 長期運用でディスク逼迫 | 中 |
| M-14 | `commands/driver/crud/logic.rs` 等で **空文字を「既存値の維持」と解釈**するロジックを各所に分散保持 | 仕様変更時のバグ温床 | 低 |
| M-15 | MQTT publish の payload で **文字列値を JSON ではなく素の文字列**としてそのまま送る | 受信側のパース実装に揺れが出る | 低 |
| M-16 | `MqttPublisher` `event_loop` の再接続が **無限 warn ループ**（バックオフなし） | 短時間でログ膨張・CPU 浪費 | 中 |
| M-17 | `PostgresPoller` の I/O カウンタは **送信 SQL バイト長 / 受信値文字列長の総和**で、実 TCP バイトを計っていない | UI 表示の B/s が実態と乖離 | 低 |

---

## 1. 信頼性（落ちない / 回復する）

### M-01: ドライバ子プロセスの **死活監視と自動再起動がない**

- 該当: [core/src-tauri/src/drivers/mod.rs](../core/src-tauri/src/drivers/mod.rs)
  - `DriverProcessManager` は `spawn` 後、`tokio::process::Child` を `HashMap` に格納するだけ。`wait()` を回す監視タスクが無い。
  - 子プロセスが異常終了しても、本体は `is_running` を `true` のまま返し、UI 上「動いている」ように見える。
- ドライバ側 [drivers/postgres/driver/src/main.rs](../drivers/postgres/driver/src/main.rs) は内部に再接続ループを持つが、**プロセス自体が落ちたケース**（DLL 例外、JoyWatcher x86 bridge ハング → 親プロセス側だけ生存等）はカバーされない。
- **改善候補**:
  1. 起動毎に `tokio::spawn(async move { child.wait().await })` を持たせ、終了コードを `tracing::error!` + `RuntimeStatusState.last_error` へ反映。
  2. 自動再起動ポリシー（指数バックオフ＋上限回数）。`drivers.toml` に `auto_restart`/`max_restart_per_minute` を追加。
  3. UI 側にドライバ別「最終異常終了」「再起動回数」表示。
  4. JoyWatcher x86 bridge は更に下層なので、`joywatcher_bridge.rs` 側の bridge プロセス監視も同時に検討。

### M-16: `MqttPublisher` の event loop に **再接続バックオフが無い**

- 該当: [core/src-tauri/src/publishers/mqtt.rs#L161-L168](../core/src-tauri/src/publishers/mqtt.rs)
  
  ```rust
  let event_loop_task = tokio::spawn(async move {
      loop {
          if let Err(e) = event_loop.poll().await {
              warn!("MQTT event loop error: {}", e);
          }
      }
  });
  ```

- broker 切断時に `poll()` が即座にエラーで戻り続けると、**毎ティック warn ログ**が出てログを埋める。CPU も無駄に回る。
- **改善候補**: エラー時 `tokio::time::sleep` を入れた指数バックオフ。`last_error` を `RuntimeStatusState` に集約。

### M-13: ログのローテーション / 容量管理

- `tracing_subscriber::fmt()` + `AppLogMakeWriter` 使用。`app_logs.rs` の Writer がローテーション機構を持っているか要確認。
- 監視システムは**長期常駐**前提なので、最低限「ファイルサイズ上限」「日次ローテーション」「世代数」を持たせる。`tracing-appender::rolling` を検討。

---

## 2. 通信相手への過負荷防止

### M-05: PostgreSQL 接続に **タイムアウトが無い**

- 該当: [drivers/postgres/driver/src/postgres_poller.rs](../drivers/postgres/driver/src/postgres_poller.rs)
  - DSN に `connect_timeout` / `statement_timeout` の付与なし。
  - `client.query_opt(&sql, &[]).await` に timeout ガードもない。
  - スキャン周期が短く、かつ DB 側が一時的に遅延すると、**前回クエリ完了を待つ間に次の周期がスキップされる**仕組み（`last_polled` で due 判定）はあるが、長クエリそのものを止める手段がない。
- **改善候補**:
  1. DSN に `connect_timeout=5` を既定で付与（ユーザ設定可能）。
  2. `tokio::time::timeout(scan_rate_ms * 0.8, client.query_opt(...))` で**スキャン周期の 80% を上限**にしてタイムアウト → quality=`bad` で publish。
  3. 接続 PG セッションで `SET statement_timeout = ...` を `connect` 直後に発行。
  4. 連続タイムアウト回数で **scan_rate を一時的に倍速**してバックオフするレート制限。

### M-06: スキャン周期遅延の検知はあるが **自衛行動がない**

- [core/src-tauri/src/grpc/driver_runtime.rs#update_scan_group_runtime_metric](../core/src-tauri/src/grpc/driver_runtime.rs)
  - `cycle_delta_ratio > 50% && consecutive >= 5` で `warn!` を出す。
  - しかし**ドライバ側へのフィードバック**（取得周期を下げる等）が無く、ログだけが流れる。
- **改善候補**:
  1. UI に「遅延スキャングループ」をハイライト表示し、運用者が `scan_rate_ms` を調整できる導線を強化（`bulk_update_driver_scan_group_rate` は既存）。
  2. 自動レートリミット（次バージョン以降の検討）。

### M-07: バックプレッシャ設計の見直し

- `MqttPublisher` の構造:
  - 入口: `TagBus`（`broadcast`）→ ラグ時は **skipped** で値を捨てる（`RecvError::Lagged`）
  - 中継: `mpsc::channel(32768)` → 満杯時は **送信側 `await`** で待つ → broadcast の取り出しが止まる → 更にラグが発生
- 結果: **同じ過負荷症状が `broadcast` lag と `mpsc` 待機の二段**で起きる。`warn` メッセージも 2 種類混在し、原因切り分けが難しい。
- **改善候補**:
  1. `mpsc::try_send` に切替えて満杯時はドロップ + `metric.dropped_count++`。
  2. ドロップ件数を `RuntimeStatusState` / UI に表示。
  3. ドキュメントに「過負荷時の挙動」を 1 章追記。

---

## 3. セキュリティ

### M-02 / M-03: TLS 未対応

- MQTT: [core/src-tauri/src/publishers/mqtt.rs](../core/src-tauri/src/publishers/mqtt.rs) は `MqttOptions` に TLS 設定なし。`rumqttc` の `set_transport(Transport::Tls(...))` 等を呼び出していない。
- PostgreSQL: [drivers/postgres/driver/src/postgres_poller.rs](../drivers/postgres/driver/src/postgres_poller.rs)

  ```rust
  tokio_postgres::connect(&self.dsn, NoTls).await
  ```

  `sslmode` 設定値を DSN に積みつつ、TLS コネクタは `NoTls` 固定なので **`sslmode=require` を設定しても TLS にならない**。これは仕様としてもっとも危険。
- **改善候補**:
  1. PostgreSQL: `postgres-native-tls` or `tokio-postgres-rustls` を導入し、`sslmode` の値に従って TLS を有効化。CA は OS ストア + 設定で追加可能に。
  2. MQTT: `MqttOptions::set_transport(Transport::Tls(ClientConfig))` を実装。CA バンドル指定、自己署名対応、クライアント証明書任意。
  3. オフライン環境前提でも **同一 LAN 上のスニファ・他社装置との混在**を想定し、TLS は「既定 off だが推奨設定」にする。

### M-04: 秘匿情報を `*.toml` 平文保存

- 規約 [.github/copilot-instructions.md](../.github/copilot-instructions.md) §1.5 で `keyring` crate へ分離が要求されている。
- 現状: `drivers.toml` の `settings.password`、`publishers.toml` の `settings.password` が平文保存。
- **改善候補**:
  1. `keyring` を導入し、保存時は `keyring://driver/<id>/password` のような参照キーだけを TOML に書く。
  2. `commands/driver/crud/logic.rs` / `commands/publisher/crud.rs` の **「空文字なら既存維持」ロジック**は keyring 対応時に共通化（M-14 もここで解消）。
  3. 当面の暫定として、`drivers.toml` 等を作成する直前に **OS ファイルパーミッション**（Windows ACL）を current user 限定に。

### M-10: gRPC サーバに認証なし

- [core/src-tauri/src/grpc/tag_registration.rs](../core/src-tauri/src/grpc/tag_registration.rs): `127.0.0.1:55051` で listen、tonic interceptor なし。
- ローカルバインドだが、**同一ホスト上の他プロセスから接続可能**。攻撃面としては低だが、認証ゼロは規約上もまずい。
- **改善候補**:
  1. 起動時に **共有トークン**を生成（Ephemeral）し、`DriverProcessManager` が子プロセスへ環境変数で渡す。tonic interceptor で `authorization` ヘッダを検証。
  2. Windows なら Named Pipe + ACL も選択肢（tonic 直対応は要 wrapper）。
  3. CSP（`tauri.conf.json`）は既に最小、これは継続維持。

---

## 4. データ品質・観測性

### M-12: タイムスタンプの出所が一貫しない

- 規約上 `TagValueMessage.timestamp` は RFC3339 文字列だが、JoyWatcher は **常に空文字を送り**、本体側で `chrono::Utc::now()` を充てている。
- PostgreSQL もクエリ実行直後の `Utc::now()` を使うため、**「DB の `timestamp_column` 値」ではない**。
- **改善候補**:
  1. ドライバ側で「データソースの時刻」が取れる場合は必ず付ける（postgres は `timestamp_column` の値を採用、JoyWatcher は bridge 側で取得時刻）。
  2. 取れない場合は空文字でなく**ドライバ側 `Utc::now()`** を入れる方が、通信遅延の影響を排せる。
  3. `quality` も同様に統一化（現状文字列 `good`/`uncertain`/`bad` のまま、enum 化候補）。

### M-11: 起動時の TOML 取り込みエラーが warn ログのみ

- `main.rs` の setup で `DataType::from_str` 失敗時に `tracing::warn!` してそのタグだけスキップ。
- **改善候補**: 起動時バリデーション結果を `RuntimeStatusState` に集約し、UI ダッシュボードに「読み込めなかったタグ N 件」を表示。

### M-09: MQTT モニタの保持件数とトピックフィルタ

- [core/src-tauri/src/subscribers/mqtt_monitor.rs](../core/src-tauri/src/subscribers/mqtt_monitor.rs)
  - `MAX_MONITOR_MESSAGES: usize = 200` ハードコード。
  - `include_sys=true` 時は `$SYS/#` まで購読 → 高頻度ブローカーで取りこぼし・UI ハング誘発の可能性。
- **改善候補**:
  1. 件数を設定値化（UI から変更可能）。
  2. `$SYS` 購読時は **「監視先ブローカーに継続的に負荷をかける」警告**を UI に常時表示。
  3. トピックフィルタはデフォルト **そのパブリッシャの `topic_prefix/#` に絞り**、`#` 直は確認ダイアログ。

### M-15: payload エンコード

- `build_payload`:

  ```rust
  serde_json::Value::String(v) => v.clone(),  // クォートなし
  ```

  → 受信側が「文字列 vs JSON 文字列」を区別できない。
- **改善候補**: 常に JSON で送る or `Content-Type` 相当の publish オプションをスキーマで決める（MQTT v5 properties が使えるならそちら）。

### M-17: I/O 計測の意味付け

- `PostgresPoller` / `JoyWatcherPollPlan` 共に **アプリ層のバイト長**を accumulate しているが、UI 上は「ドライバ I/O」「ネットワーク」と表示されている。
- ユーザに**「これはアプリ層のペイロード量で TCP/IP 実バイトではない」**ことを明示する。または `tokio` ソケット層から OS カウンタを取る。

---

## 5. 規約・設計整合性

### M-08: MQTT トピック生成の正規化不足

- [core/src-tauri/src/publishers/mqtt.rs#build_topic](../core/src-tauri/src/publishers/mqtt.rs)
  - `normalize_topic_segment` は前後 `/` を trim するだけで、`+`, `#`, 制御文字、空白を含むタグ名がそのまま topic に入る。
- **改善候補**:
  1. MQTT 予約文字（`+`, `#`, `/`, NUL）と非 ASCII を `_` 等へエスケープ。
  2. タグ登録時のバリデーションで弾く（規約化）。

### M-14: 「空文字 = 既存維持」の暗黙ルール分散

- `commands/driver/crud/logic.rs` / `commands/publisher/crud.rs` 双方に同じパターン。`R-DEDUP-*` 範囲で共通化候補。

### ドキュメント整合性

- 本書を踏まえ、[docs/refactor-plan.md](./refactor-plan.md) の **§3 タスク表に `R-OPS-*`（運用品質向上）または `R-SEC-*` を新設**し、上記 M-01〜M-17 をタスク ID 化することを推奨。

---

## 6. 優先度提案（参考）

| 優先度 | 推奨先着手 | 理由 |
| --- | --- | --- |
| 高 | M-01 / M-03 / M-04 / M-02 | 監視継続性 + 認証情報保護はオフラインでも守るべき最低ライン |
| 中 | M-05 / M-16 / M-13 / M-10 | 長期常駐前提なら短期に効く |
| 中 | M-06 / M-07 / M-09 / M-12 | 過負荷検知と通信先保護 |
| 低 | M-08 / M-11 / M-14 / M-15 / M-17 | 仕様 / 表示の整合性向上 |

---

## 7. 残課題・本書のスコープ外

- 性能・スケーリング（タグ数 100k 級など）の評価は別途必要。
- 物理層の冗長化（NIC・MQTT broker クラスタ）はアプリ責務外。
- 監視カバレッジ（メトリクスを別 Prometheus 等へエクスポートするか）は要件次第。
- 本書はコードレビューに基づく**改善候補**であり、各項目の採否・優先付け・実装計画は別途レビューミーティングで決定すること。
