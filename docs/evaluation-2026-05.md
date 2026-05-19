# kt_iot_hub 設計・アーキテクチャ・コード評価レポート

- 評価日: 2026-05-20
- 評価対象リビジョン: ワークスペース現状（`main` 相当）
- 評価範囲: [docs/](./) の設計群、`core/src-tauri`（バックエンド）、`core/src`（Svelte 5 フロント）、`drivers/`（外部ドライバ群）、`packages/`（共有クレート）
- 評価観点: 設計妥当性 / アーキテクチャ整合性 / 規約（[.github/copilot-instructions.md](../.github/copilot-instructions.md)）遵守 / コード品質 / リファクタ計画（[docs/refactor-plan.md](./refactor-plan.md)）達成度

---

## 1. エグゼクティブサマリ

| 項目 | 評価 | 要点 |
| --- | --- | --- |
| 全体設計 | ◎ | 「Tauri 本体 + 外部ドライバプロセス + Tag Bus + MQTT Publisher」の責務分離が明確。設計書群（[docs/design.md](./design.md), [docs/architecture.md](./architecture.md), [docs/decisions.md](./decisions.md)）は判断理由まで含めて整っている。 |
| アーキテクチャ実装整合性 | ○ | gRPC 常時稼働・ランタイム制御の対象範囲、DriverProcessManager、Tag Bus、PublisherManager の構成は設計と一致。ドライバ UI 分離・JoyWatcher x86 bridge も設計通り。 |
| コード品質（バックエンド） | ○ | `commands/` の薄ラッパ + `core` / `drivers` 分離は守られている。1 ファイル 300 行規約はおおむね達成（後述の例外あり）。 |
| コード品質（フロント） | ◎ | リファクタ計画後の分割（`ThreePane`, `DashboardContent`, `TagTree`, `MqttMonitorWindow` 等）が反映され、現状 Svelte 側は全ファイル 300 行台以下に収束。 |
| リファクタ計画達成度 | ◎ | `R-BE-01〜08`, `R-FE-01〜09`, `R-RS-01〜03`, `R-DEDUP-01〜12`, `R-NEW-01` は完了。現状の規約残件は Svelte 2 ファイル（`MqttMonitorWindow.svelte` 406 行、`PostgresRegistrationPanel.svelte` 355 行）のみ。 |
| 既知の地雷 | ○ | 以前の巨大 `drivers/*/ui/assets/app.js` は解消済み。現時点の主な注意点は、ドキュメント内に古い行数・未完表記が残りやすい点。 |
| セキュリティ / 規約 | ○ | `unsafe_code` の使用は JoyWatcher x86 bridge の FFI 周辺に局所化。`tauri.conf.json` の CSP 維持・capabilities 最小化方針は継続中。秘匿情報のハードコードは未検出。 |

総評: **設計はオフライン IoT ハブとして非常に良くまとまっており、現状はリファクタ計画後半に位置する成熟段階**。残課題はフロントエンドのドライバ別静的資産（巨大 `app.js`）と少数の重複ヘルパに収束している。

---

## 2. 設計評価

### 2.1 強み

1. **責務分離が一貫している**
   - 本体: タグ定義の正本管理 / Tag Bus / DriverProcessManager / PublisherManager / gRPC サーバ。
   - ドライバ: 「登録 UI（Tauri 子プロセス）」と「通信ランタイム（gRPC client バイナリ）」を別プロセス化（[docs/decisions.md](./decisions.md) 「ドライバ通信処理の別プロセス化」）。
   - これにより DriverType 追加時の本体改修が最小化される設計意図が、実装（`DriverProcessManager::start_driver`, `TagRegistrationService`, `DriverRuntimeService`）にきちんと反映されている。
2. **不変条件の明文化**
   - [docs/refactor-plan.md](./refactor-plan.md) §0.2 で「Tauri コマンド名 / serde 表現 / proto / TOML キー / ログ grep キー」をバイト互換維持と宣言。`dto/serde_snapshot_tests.rs` の存在で表現崩れに対する回帰テストの足場もある。
3. **データフロー単純化**
   - `tokio::sync::broadcast` を中心とした Tag Bus 一本化により、ドライバ → 本体 → Publisher / UI の経路が単純で、増設に強い。
4. **設定の TOML 正本化**
   - AI 編集・Git 差分の双方に強い。アトミック書き出し（`write_toml_atomic`）が `commands/config_io` に集約済み。
5. **段階的ディスカバリ移行**
   - [docs/driver-manifest-discovery-design.md](./driver-manifest-discovery-design.md) で manifest 駆動への 3 段階移行（v0.4.0 deprecate → v0.5.0 削除）が決定済み。実装は `drivers/manifest.rs` + `drivers/manifest_discovery.rs` へ分割済み。

### 2.2 弱み・リスク

| # | 内容 | 影響 | 推奨対応 |
| --- | --- | --- | --- |
| W-1 | `MqttMonitorWindow.svelte` (406) / `PostgresRegistrationPanel.svelte` (355) が 300 行超 | 規約上の残件 | 機能安定性は維持されているため、次回の保守タイミングで段階分割（任意）。 |
| W-2 | ドキュメント内の古い行数・未完表記が残りやすい | 進捗判断の誤認 | `docs/refactor-plan.md` と本評価レポートを同日に同期更新する運用を固定化。 |
| W-3 | `import_driver_ui_result` 反映順序（sync 失敗時の見え方） | 運用時の混乱余地 | rollback もしくは反映順序整理を別チケットで検討。 |
| W-4 | `cargo clippy -D warnings` のベースライン管理 | CI 安定性 | `dead_code` 等の段階是正を継続タスク化。 |
| W-5 | `apps_state.rs` の `Shared<T>` が 16 並列フィールド（リファクタ計画では R-BE-07 で補助型外出しのみ実施済） | 後続で凝集度が上がりにくい | ドメイン別サブ構造体（`MqttMonitorState`, `DriverUiSessionStore` 等）への凝集を別タスクで段階導入。今回の不変条件には抵触しない範囲で。 |
| W-6 | `import_driver_ui_result` の反映順序: `sync_driver_runtime` 失敗時に `tags.toml` 更新済 / registry 未反映の不整合（リポジトリメモ既知） | 運用時の見え方の混乱 | 反映を「メモリ反映 → ランタイム sync → 失敗時に TOML を rollback」へ整理する設計タスクを起票推奨（互換維持のため別チケット）。 |
| W-7 | `cargo clippy --all-targets --all-features -- -D warnings` の既存ベースラインが `dead_code` 系で失敗（リポジトリメモ既知） | CI で「全体グリーン」の保証がない | 影響ファイルごとに `#[allow(dead_code)]` を局所付与 → 段階的に削除する別タスク化（リファクタ規律「機能変更を伴わない」と整合）。 |
| W-8 | 認証なし運用（[decisions.md](./decisions.md) で明示） | 単独 PC 前提なので現状は妥当 | 配布範囲が広がる前に「TOML / `keyring` 秘匿項目分離」「ローカル管理 PIN 等」を ADR 化。 |

### 2.3 設計と実装の差分チェック

- ✅ `docs/architecture.md` の「Driver Process Manager」「gRPC 常時稼働」「Tag Bus 中心」「Publisher trait」: いずれも実装側に対応物あり（`drivers/mod.rs`, `grpc/driver_runtime.rs`, `core/tag_bus.rs`, `publishers/mod.rs`）。
- ✅ `decisions.md` の「`start_runtime_services` は gRPC を停止しない」: `commands/runtime.rs` 実装と整合（gRPC shutdown は本体終了時のみ）。
- ✅ `mqtt-monitor.md` の「subscriber は本体内・モニタは別ウィンドウ」: `subscribers/mqtt_monitor.rs` + `commands/subscriber/monitor/` + Svelte `MqttMonitorWindow.svelte` で実装。
- ✅ `docs/refactor-plan.md` §1.1 / §4 は 2026-05-20 時点で更新済み。タスク進捗は「R-DEDUP-07/08/10 を含め完了」に同期済み。

---

## 3. アーキテクチャ評価

### 3.1 レイヤリング

```text
Svelte 5 (SPA)
  └─ src/lib/ipc/*.ts  ←  Tauri invoke のラッパ層（コンポーネントから直接 invoke 禁止規約と整合）
        │
        ▼
Tauri commands (薄い橋渡し)
  └─ commands/{tag,driver,publisher,subscriber,runtime,metrics,logs}/...
        │
        ▼
core (ドメイン): TagRegistry / TagBus
drivers (子プロセス管理 + manifest 探索)
publishers (MqttPublisher trait 実装)
subscribers (MqttMonitor)
grpc (DriverRuntimeService / TagRegistrationService)
        │
        ▼
外部プロセス: drivers/{postgres,joywatcher}/{ui, driver(, bridge-x86)}
共有: packages/{protocol-rs, driver-ui-host}
```

- ✅ 規約「`commands/` は薄く / ビジネスロジックは `core` 等に」を概ね順守。
- ✅ 共有プロトコルは `packages/protocol-rs` を正本にし、本体側 re-export（`commands/dto/driver_ui.rs` および `kt_driver_ui_host`）。
- ✅ JoyWatcher は x86 bridge プロセスにより、64bit 本体プロセスから FFI 非互換を切り離す方針が実装に反映（`drivers/joywatcher/{bridge-x86, driver, ui}`）。

### 3.2 通信境界

| 境界 | プロトコル | 評価 |
| --- | --- | --- |
| Svelte ↔ 本体 | Tauri invoke / event | `ipc/*.ts` ラッパで型付き集約、`event` は `ドット区切り` 規約 |
| 本体 ↔ ドライバ UI | 子プロセス + `--input-json` / `--output-json` ファイル | `packages/protocol-rs` で JSON 形を一元化、`kt_driver_ui_host` で取り扱い再利用 |
| 本体 ↔ ドライバランタイム | gRPC (`DriverRuntimeService`) | `TagValueMessage` に `io_rx_bytes_total/io_tx_bytes_total` を載せ、B/s 算出をドライバ報告基準に統一（メモあり） |
| JoyWatcher ランタイム ↔ x86 bridge | stdin/stdout JSON | `bridge_client/protocol.rs` で型と Shift_JIS 変換を集約 |

- ✅ いずれの境界でもデータ表現の正本ファイルが特定されており、変更影響範囲が読みやすい。
- ⚠️ `bridge_client/protocol.rs` (257 行) と `joywatcher_bridge/protocol.rs` (229 行) は別プロセスの両端で**互いに対称な型**を持つため、現状の手動同期にはドリフトリスクがある。将来的に `packages/` の共有クレート（例 `kt-joywatcher-protocol`）へ抽出する余地あり（緊急度低）。

### 3.3 並行性 / ライフサイクル

- ✅ Tokio 一本化、`Arc<RwLock<...>>` / `Arc<Mutex<...>>` の使い分けは妥当（`MqttMonitor` のみ `tokio::sync::Mutex`、他は `RwLock`）。
- ✅ ドライバ子プロセスは `kill_on_drop(true)` + 3 秒タイムアウト後の強制 kill で停止フローが揃っている（`drivers/mod.rs::stop_driver`）。
- ⚠️ `static SHUTDOWN_IN_PROGRESS: AtomicBool` を `main.rs` で使用。グローバル可変は許容範囲だが、`OnceCell<...>` ベースの shutdown coordinator に集約しておくと将来のテスト容易化に資する（任意）。

---

## 4. コード品質評価（実測ベース）

### 4.1 ファイルサイズ（300 行規約）

実測 Top（評価日時点）:

| ファイル | 行数 | 区分 | 評価 |
| --- | --- | --- | --- |
| [core/src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte](../core/src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte) | 406 | フロント | △ 300 超（R-FE-08 後の残分） |
| [core/src/lib/components/driver/PostgresRegistrationPanel.svelte](../core/src/lib/components/driver/PostgresRegistrationPanel.svelte) | 355 | フロント | △ 300 超（R-FE-07 後の残分） |
| [core/src-tauri/src/drivers/manifest.rs](../core/src-tauri/src/drivers/manifest.rs) | 290 | バックエンド | ✅ R-BE-08b で 300 行以下 |
| [core/src-tauri/src/drivers/mod.rs](../core/src-tauri/src/drivers/mod.rs) | 262 | バックエンド | ✅ R-BE-08 で 300 行以下 |
| [core/src-tauri/src/commands/driver/transfer.rs](../core/src-tauri/src/commands/driver/transfer.rs) | 188 | バックエンド | ✅ R-NEW-01 で 300 行以下 |
| 他（300 行未満） | — | — | ✅ |

逆に**プランで「分割対象」とされていたが既に達成済み**の主要例:

- `ThreePane.svelte` 465 → **4 行**（薄化完了。`three-pane/` 配下に移管）
- `DashboardContent.svelte` 778 → **164 行**（R-FE-02 完了）
- `TagTree.svelte` 559 → **298 行**（R-FE-05 完了）
- `LogsContent.svelte` 397 → **136 行**（R-FE-03 完了）
- `PublishersContent.svelte` 335 → **141 行**（R-FE-04 完了）
- `commands/subscriber/monitor.rs` 349 → 配下 5 ファイル分割（R-BE-05 完了、最大 `command.rs` 196）
- `commands/driver/ui_launcher.rs` 366 → 配下 6 ファイル分割（R-BE-02 完了）
- `commands/driver/import.rs` 337 → 配下 4 ファイル分割（R-BE-03 完了）
- `commands/driver/crud.rs` 325 → 配下 3 ファイル分割（R-BE-04 完了）
- `apps/joywatcher/bridge-x86/src/dll_api.rs` 583 → **221 行**（R-RS-01 完了）
- `apps/joywatcher/driver/src/joywatcher_bridge.rs` 473 → 配下分割（R-RS-03 完了。最大 `protocol.rs` 229）
- `apps/joywatcher/ui/src/joywatcher_bridge_client.rs` 584 → 配下分割（R-RS-02 完了。最大 `protocol.rs` 257）

### 4.2 重複・共通化

| ID | 状態 | 備考 |
| --- | --- | --- |
| D-01 (`write_tags_toml_atomic` 2 重定義) | ✅ 解消 | 現状 `commands/driver/toml_io.rs` の 1 箇所のみ。`tag.rs` 側は `super::driver::toml_io::write_tags_toml_atomic` を import |
| D-02 (`resolve_config_dir` 等の共通化) | ✅ 部分解消 | `commands/config_io/mod.rs` に統合済 |
| D-03 (`ErrorResponse` 構築の散在) | △ 残 | `ErrorResponse::invalid_input` ヘルパは追加されたが、`map_err` 直書きは依然多い |
| D-05 (`normalize_optional_string` 3 重) | ✅ 解消 | R-DEDUP-08 完了。`commands/util.rs` 正本へ統一 |
| D-06 / D-08 (ドライバ UI 系の重複) | ✅ 解消 | R-FE-09 + R-DEDUP-07 完了。ESM 分割と共通ライブラリ化を実施 |
| D-07 (driver `grpc_client.rs` の 1 行差重複) | ✅ 解消 | R-DEDUP-10 完了。`apps/common/driver_runtime_grpc_client.rs` へ統合 |

### 4.3 エラー処理 / 安全性

- `unwrap()` / `expect()` の濫用は本体側コードで未検出（生成済みコードや `main()` 初期化を除く）。
- `unsafe` は JoyWatcher x86 bridge（`drivers/joywatcher/bridge-x86`）の FFI に局所化（Windows API による cp932 変換含む）。crate root の `#![deny(unsafe_code)]` 方針はバックエンド本体側で守られている見込み（ファイル別に再確認推奨）。
- `static mut` の使用は未検出。`SHUTDOWN_IN_PROGRESS` は `AtomicBool` で適切。

### 4.4 テスト

- `commands/dto/serde_snapshot_tests.rs` で DTO の JSON 表現スナップショットが取られており、不変条件（serde 表現バイト互換）の自動チェック基盤あり。
- ドライバ単位のユニットテスト網羅率は未計測。`drivers/joywatcher/bridge-x86`・`drivers/postgres/driver` に重点テストを置く余地あり。

---

## 5. リファクタ計画達成度マップ

| タスク群 | 状態 |
| --- | --- |
| R-BE-01: `dto.rs` の分割 | ✅ 完了（`commands/dto/{tag,driver,publisher,subscriber,metrics,runtime,driver_ui,common}.rs`） |
| R-BE-02: `ui_launcher` 分割 | ✅ 完了 |
| R-BE-03: `import` 分割 | ✅ 完了 |
| R-BE-04: `crud` 分割 | ✅ 完了（`crud/{command,logic,mod}.rs`） |
| R-BE-05: `subscriber/monitor` 分割 | ✅ 完了 |
| R-BE-06: `metrics` platform 分割 | ◯（`commands/metrics/` 化のみ。windows/fallback 分離は未確認、現状 1 ファイル 179 行で許容） |
| R-BE-07: `app_state` 補助型外出し | ✅ 完了（`app_state/{app_cpu,driver_ui_session,mqtt_monitor,scan_metrics}.rs`） |
| R-FE-01〜08 | ✅ ほぼ完了（`MqttMonitorWindow` のみ 406 行で部分残） |
| **R-FE-09: ドライバ UI 静的資産分割** | ✅ 完了（`app.js` エントリ薄化 + `_shared` 分割） |
| R-RS-01/02/03: JoyWatcher 大ファイル分割 | ✅ 完了 |
| R-DEDUP-01: `write_tags_toml_atomic` 重複排除 | ✅ 完了 |
| **R-DEDUP-08: `normalize_optional_string` 統一** | ✅ 完了 |
| R-DEDUP-07/09/10: ドライバ UI / JoyWatcher / gRPC client 共通化 | ✅ 完了 |

---

## 6. 推奨アクション（優先度順）

1. **ドキュメント同期運用の固定化（中）**  
   - `refactor-plan` / `evaluation` を同日更新し、未完表記の陳腐化を防止する。
2. **Svelte 300 行超 2 ファイルの扱い方針決定（中）**  
   - `MqttMonitorWindow.svelte` と `PostgresRegistrationPanel.svelte` を追加分割するか、例外理由を明記して据え置くかを決める。
3. **`import_driver_ui_result` の整合性改善（低・別チケット）**  
   - rollback もしくは反映順序整理の設計レビューを行う。
4. **clippy `-D warnings` ベースライン運用（低）**  
   - `dead_code` 系の段階是正タスクを継続する。

---

## 7. 結論

- 本プロジェクトは **設計書・ADR・リファクタ計画が三位一体で運用されている稀有な状態**にあり、AI / 人間いずれの実装者でも追従しやすい構造を達成している。
- リファクタ計画後半（フロント分割・JoyWatcher 大ファイル分割・DTO 分割・重複排除）は実装に反映済みで、**主要タスクは完了済み**。
- 今後は (a) ドキュメント同期運用、(b) Svelte 300 行超 2 件の扱い方針、(c) 低優先の品質改善（clippy/反映順序）を進める段階。
