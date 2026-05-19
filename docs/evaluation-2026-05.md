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
| リファクタ計画達成度 | ○ | `R-BE-01/02/03/05/06/07`, `R-FE-01〜08`, `R-DEDUP-01`, `R-RS-01〜03` は概ね完了。**未完が `R-FE-09`（ドライバ UI 静的資産分割）と `R-DEDUP-08`（`normalize_optional_string` 3 重定義）**。 |
| 既知の地雷 | △ | ドライバ UI 静的資産が肥大化（`drivers/joywatcher/ui/assets/app.js` が **1,242 行**、計画時 1,105 行より増加）。`drivers/mod.rs` (431) / `drivers/manifest.rs` (439) が新たな 300 行超過候補。 |
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
   - [docs/driver-manifest-discovery-design.md](./driver-manifest-discovery-design.md) で manifest 駆動への 3 段階移行（v0.4.0 deprecate → v0.5.0 削除）が決定済み。`drivers/manifest.rs` (439 行) が Phase 1 の実体。

### 2.2 弱み・リスク

| # | 内容 | 影響 | 推奨対応 |
| --- | --- | --- | --- |
| W-1 | ドライバ UI 静的資産（`drivers/*/ui/assets/app.js`）が分割されておらず肥大化（joywatcher 1,242 行、postgres 709 行） | 規約違反 / 保守性低下 / 重複バグ温床 | `R-FE-09` を実施。`<script type="module">` 化 + 共通 `assets/lib/` に invoke ラッパ・`formatError`・`normalizeId` を集約。CSP / `frontendDist` 影響事前確認。 |
| W-2 | `drivers/mod.rs` 431 行 / `drivers/manifest.rs` 439 行 | 規約（300 行）違反 | manifest discovery 系を `manifest/{schema,discover,resolver}.rs` に責務分割。`drivers/mod.rs` は executable 解決ロジックを `executable_resolver.rs` に分離余地あり。 |
| W-3 | `normalize_optional_string` が `util.rs` / `crud/logic.rs` / `ui_launcher/paths.rs` の 3 箇所に並存 | 仕様分岐リスク | `R-DEDUP-08`: `commands/util.rs` を正本にし、他 2 箇所を `use crate::commands::util::normalize_optional_string;` に置換。 |
| W-4 | `commands/driver/transfer.rs` 333 行（タグ設定 JSON エクスポート/インポート） | 規約超過 | `transfer/{command.rs, export.rs, import.rs}` に責務分割を検討。 |
| W-5 | `apps_state.rs` の `Shared<T>` が 16 並列フィールド（リファクタ計画では R-BE-07 で補助型外出しのみ実施済） | 後続で凝集度が上がりにくい | ドメイン別サブ構造体（`MqttMonitorState`, `DriverUiSessionStore` 等）への凝集を別タスクで段階導入。今回の不変条件には抵触しない範囲で。 |
| W-6 | `import_driver_ui_result` の反映順序: `sync_driver_runtime` 失敗時に `tags.toml` 更新済 / registry 未反映の不整合（リポジトリメモ既知） | 運用時の見え方の混乱 | 反映を「メモリ反映 → ランタイム sync → 失敗時に TOML を rollback」へ整理する設計タスクを起票推奨（互換維持のため別チケット）。 |
| W-7 | `cargo clippy --all-targets --all-features -- -D warnings` の既存ベースラインが `dead_code` 系で失敗（リポジトリメモ既知） | CI で「全体グリーン」の保証がない | 影響ファイルごとに `#[allow(dead_code)]` を局所付与 → 段階的に削除する別タスク化（リファクタ規律「機能変更を伴わない」と整合）。 |
| W-8 | 認証なし運用（[decisions.md](./decisions.md) で明示） | 単独 PC 前提なので現状は妥当 | 配布範囲が広がる前に「TOML / `keyring` 秘匿項目分離」「ローカル管理 PIN 等」を ADR 化。 |

### 2.3 設計と実装の差分チェック

- ✅ `docs/architecture.md` の「Driver Process Manager」「gRPC 常時稼働」「Tag Bus 中心」「Publisher trait」: いずれも実装側に対応物あり（`drivers/mod.rs`, `grpc/driver_runtime.rs`, `core/tag_bus.rs`, `publishers/mod.rs`）。
- ✅ `decisions.md` の「`start_runtime_services` は gRPC を停止しない」: `commands/runtime.rs` 実装と整合（gRPC shutdown は本体終了時のみ）。
- ✅ `mqtt-monitor.md` の「subscriber は本体内・モニタは別ウィンドウ」: `subscribers/mqtt_monitor.rs` + `commands/subscriber/monitor/` + Svelte `MqttMonitorWindow.svelte` で実装。
- ⚠️ `docs/refactor-plan.md` §1.1 の「分割対象一覧」は**既に大半が完了済**で表記が陳腐化。完了タスクと残課題（`R-FE-09`, `R-DEDUP-08`, 新規発生した 300 行超過 2 件）を §4 進行ログに反映し、表を最新化する必要あり。

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
| [drivers/joywatcher/ui/assets/app.js](../drivers/joywatcher/ui/assets/app.js) | 1242 | フロント（ドライバ UI） | ❌ 要分割（R-FE-09 未完）。計画時 1,105 → 増加 |
| [drivers/postgres/ui/assets/app.js](../drivers/postgres/ui/assets/app.js) | 709 | フロント（ドライバ UI） | ❌ 要分割（R-FE-09 未完） |
| [core/src-tauri/src/drivers/manifest.rs](../core/src-tauri/src/drivers/manifest.rs) | 439 | バックエンド | ❌ 新規 300 行超過 |
| [core/src-tauri/src/drivers/mod.rs](../core/src-tauri/src/drivers/mod.rs) | 431 | バックエンド | ❌ 新規 300 行超過 |
| [core/src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte](../core/src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte) | 406 | フロント | △ 300 超だが R-FE-08 で大幅縮減後（627→406）。残分割余地あり |
| [core/src/lib/components/driver/PostgresRegistrationPanel.svelte](../core/src/lib/components/driver/PostgresRegistrationPanel.svelte) | 355 | フロント | △ R-FE-07 部分達成（415→355） |
| [core/src-tauri/src/commands/driver/transfer.rs](../core/src-tauri/src/commands/driver/transfer.rs) | 333 | バックエンド | ❌ 計画未収録の 300 超過 |
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
| D-05 (`normalize_optional_string` 3 重) | ❌ **未解消**（R-DEDUP-08 残） | `util.rs` / `crud/logic.rs` / `ui_launcher/paths.rs` |
| D-06 / D-08 (ドライバ UI 系の重複) | ❌ 未解消 | R-FE-09 と連動。`tauriInvoke` ラッパ・`BRIDGE_EXE_NAME` 探索ロジックの共通化が宙ぶらりん |
| D-07 (driver `grpc_client.rs` の 1 行差重複) | ❌ 未解消（ファイルパス変更あり） | `drivers/{joywatcher,postgres}/driver/src/main.rs` 周辺へ移動済の可能性あり。要再調査 |

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
| **R-FE-09: ドライバ UI 静的資産分割** | ❌ **未着手 / 悪化中**（joywatcher 1,105→1,242） |
| R-RS-01/02/03: JoyWatcher 大ファイル分割 | ✅ 完了 |
| R-DEDUP-01: `write_tags_toml_atomic` 重複排除 | ✅ 完了 |
| **R-DEDUP-08: `normalize_optional_string` 統一** | ❌ 未完 |
| R-DEDUP-07/09: ドライバ UI / JoyWatcher 共通化 | △ 部分残 |

---

## 6. 推奨アクション（優先度順）

1. **R-FE-09 の再優先化（高）**  
   - `drivers/joywatcher/ui/assets/app.js` (1,242 行) を ESM 分割し、`drivers/{joywatcher,postgres}/ui/assets/lib/` 共通モジュール（invoke ラッパ・`formatError`・`normalizeId`）を導入。CSP（`script-src 'self'`）と `frontendDist` の参照を事前検証。
2. **`drivers/mod.rs` / `drivers/manifest.rs` の責務分割（中）**  
   - manifest 駆動ディスカバリは Phase 進行に伴いさらに増える見込みのため、`drivers/manifest/{schema,discover,resolver,fallback}.rs` 等への分割を計画化（[docs/refactor-plan.md](./refactor-plan.md) に新規 `R-BE-08` として追記推奨）。
3. **R-DEDUP-08（中）**  
   - `commands/util.rs` を正本に統一。差分はテストで保証。
4. **`commands/driver/transfer.rs` 分割（中）**  
   - JSON エクスポート / インポート / `driverUiBaseDir` 既定値解決を独立ファイルへ。
5. **refactor-plan の §1.1 / §4 進行ログ更新（中）**  
   - 完了済タスクの除去と、新規発生した 300 行超過 2 件・`app.js` 拡大の記録。
6. **`MqttMonitorWindow.svelte` の残り 100 行削減（低）**  
   - `monitor/monitorPolling.ts` 抽出が完了済かを確認し、残ロジックを `ControlBar.svelte` 等に分配。
7. **`import_driver_ui_result` の整合性改善（低・別チケット）**  
   - TOML rollback or 「メモリ反映先行」順序の設計レビュー（互換維持の範囲で）。
8. **clippy `-D warnings` グリーン化計画（低）**  
   - `dead_code` 系の局所 `#[allow]` を段階解消する独立タスクとして起票。

---

## 7. 結論

- 本プロジェクトは **設計書・ADR・リファクタ計画が三位一体で運用されている稀有な状態**にあり、AI / 人間いずれの実装者でも追従しやすい構造を達成している。
- リファクタ計画後半（フロント分割・JoyWatcher 大ファイル分割・DTO 分割・`write_tags_toml_atomic` 重複排除）は実装に反映済みで、**残課題はドライバ UI 静的資産と少数のヘルパ重複に限定**できている。
- 今後は (a) ドライバ UI 静的資産の ESM 化、(b) manifest discovery 関連の責務分割、(c) refactor-plan の最新化、の 3 点を優先することで、規約（300 行）達成率を再び 100% に近づけられる。
