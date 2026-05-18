# kt_iot_hub リファクタ計画

> 目的: 機能変更を一切伴わず、**保守性・可読性・テスト容易性**を底上げする。
> 想定読者: 本書を見て独立に着手する別 AI エージェント / 開発者。
> 前提: `.github/copilot-instructions.md` 記載の規約（1ファイル300行・責務分割・既存パターン踏襲）を遵守する。

---

## 0. 全体方針

### 0.1 ゴール（非ゴール）

- ✅ ファイル分割 / モジュール再編 / 重複削減 / 命名統一
- ✅ テスト追加（純粋関数化したロジックに対して）
- ❌ 機能追加・既存挙動の変更
- ❌ 依存追加（必要な場合は事前提案 → 承認）
- ❌ プロトコル（gRPC / 外部 JSON / Tauri コマンド名）の変更

### 0.2 守るべき不変条件

| 項目 | 不変であること |
| --- | --- |  |
| Tauri コマンド名 / 引数 / 戻り値 | 1 バイトも変えない（`main.rs` `invoke_handler!` も同じ） |
| `commands::dto` の serde 表現（`camelCase` / フィールド名） | 完全互換 |
| gRPC proto / メッセージ | 変更禁止（変えるなら別タスク） |
| `kt_driver_ui_protocol` の JSON 形 | 完全互換 |
| `config/*.toml` のキー | 完全互換 |
| ログメッセージ（grep キー） | 可能な限り維持。変更時は本書に追記 |
| **シリアライズ表現（FE 向け JSON、ドライバ UI 向け JSON、TOML）** | **完全互換**（共通化リファクタで意図せず崩さない。serde derive 整理時は `serde_json::to_value` を比較する snapshot テストの導入を推奨） |

### 0.3 進め方の原則

1. **1 タスク = 1 PR / 1 コミット粒度**。本書のタスク ID（例 `R-BE-01`）をコミット先頭に付ける。
2. タスク着手前に「対象ファイル一覧」と「移動先の構造」を提示してから手を動かす。
3. 完了時に必ず: `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test` / `npm run check` を通す。
4. 移動だけの変更とロジック変更は**コミットを分ける**（レビュー容易化のため）。
5. リネーム・移動は `git mv` 相当（IDE のリネーム）で履歴を残す。

### 0.4 ブランチ運用

- 推奨: `refactor/<task-id>-<short-name>` （例: `refactor/R-BE-01-split-dto`）
- 大タスクは小タスクへ分割し、PR を細かく出す。

---

## 1. 現状サマリ（2026-05 時点）

### 1.1 規約違反 / グレーゾーン（300 行超）

#### Rust（src-tauri）

| ファイル | 行数 | 主担務 | 状態 |
| --- | --- | --- | --- |  |
| `src-tauri/src/commands/driver/ui_launcher.rs` | 366 | UI 起動・結果待ち | 分割対象 |
| `src-tauri/src/commands/subscriber/monitor.rs` | 349 | MQTT モニタ + ツリー構築 | 分割対象 |
| `src-tauri/src/commands/driver/import.rs` | 337 | 結果取込 + バリデーション | 分割対象 |
| `src-tauri/src/commands/driver/crud.rs` | 325 | CRUD + ランタイム連携 | 分割対象 |

#### Rust（apps）

| ファイル | 行数 | 主担務 | 状態 |
| --- | --- | --- | --- |  |
| `apps/joywatcher/ui/src/joywatcher_bridge_client.rs` | 584 | bridge stdio クライアント | 分割対象 |
| `apps/joywatcher/bridge-x86/src/dll_api.rs` | 583 | DLL FFI ラッパ | 分割対象 |
| `apps/joywatcher/driver/src/joywatcher_bridge.rs` | 473 | ランタイム側 bridge クライアント | 分割対象 |

#### Svelte / TS

| ファイル | 行数 | 主担務 | 状態 |
| --- | --- | --- | --- |  |
| `src/lib/components/layout/three-pane/DashboardContent.svelte` | 778 | ダッシュボード全部 | 分割対象 |
| `src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte` | 627 | MQTT モニタ画面 | 分割対象 |
| `src/lib/components/tag/TagTree.svelte` | 559 | タグツリー + 文脈メニュー | 分割対象 |
| `src/lib/components/layout/ThreePane.svelte` | 465 | レイアウト + 多数 IPC オーケストレーション | 分割対象 |
| `src/lib/components/driver/PostgresRegistrationPanel.svelte` | 415 | postgres 登録 | 分割対象 |
| `src/lib/components/driver/DriverDetailPanel.svelte` | 404 | ドライバ詳細 | 分割対象 |
| `src/lib/components/layout/three-pane/LogsContent.svelte` | 397 | ログ画面 | 分割対象 |
| `src/lib/components/layout/three-pane/PublishersContent.svelte` | 335 | パブリッシャ画面 | 分割対象 |

#### ドライバ UI（apps/*/ui/assets/app.js）

| ファイル | 行数 | 状態 |
| --- | --- | --- |  |
| `apps/joywatcher/ui/assets/app.js` | 1105 | 分割対象（最大） |
| `apps/postgres/ui/assets/app.js` | 725 | 分割対象 |

### 1.2 ホットスポット（行数は OK だが責務多）

- `src-tauri/src/commands/dto.rs` (284 行): タグ/ドライバ/パブリッシャ/サブスクライバ/メトリクス DTO が同居 → **機能別分割推奨**。
- `src-tauri/src/app_state.rs` (158 行): 16 個の `Arc<RwLock<...>>` がフラットに並ぶ → グルーピング検討。
- `src/lib/ipc/index.ts` (10 行) は OK だが、`ipc/` 配下に MQTT モニタ / metrics 等のドメイン分離は既にできている。

### 1.3 重複・共通化候補（バグ温床）

調査で判明した重複・冗長コードを以下にまとめる。詳細タスクは §3.4 を参照。

| ID | 内容 | 場所 | 重大度 |
| --- | --- | --- | --- |
| D-01 | `write_tags_toml_atomic` が **2 箇所で重複定義**（書き出し挙動が将来分岐するリスク） | `src-tauri/src/commands/driver/toml_io.rs:31` と `src-tauri/src/commands/tag.rs:215`（`TagsTomlFile`・`resolve_config_dir` も二重） | **高** |
| D-02 | `resolve_config_dir` / `replace_file_atomically` / TOML アトミック書き出し雛形が driver / publisher で別実装 | `commands/driver/toml_io.rs`, `commands/publisher/toml_io.rs` | 中 |
| D-03 | `ErrorResponse { error: ..., code: "XXX".to_string() }` 構築が 20+ 箇所、`map_err(\|e\| ErrorResponse { ... })` も 16+ 箇所 | `commands/**` 全域 | 中 |
| D-04 | `Arc<RwLock<...>>` 直書きが 16 箇所（`app_state.rs` 15 + `core/mod.rs` 1） | `src-tauri/src/app_state.rs`, `src-tauri/src/core/mod.rs` | 低 |
| D-05 | `normalize_optional_string` 同名関数が 3 ファイルに重複定義 | `commands/driver/ui_paths.rs:117`, `commands/driver/crud.rs:136`, `commands/runtime.rs:199` | 中 |
| D-06 | `BRIDGE_EXE_NAME` / `DLL_FILE_NAME` / `push_unique` / 候補パス探索が 4 ファイルに散在 | `apps/joywatcher/{driver,ui,bridge-x86}` 各種 | 中 |
| D-07 | `apps/joywatcher/driver/src/grpc_client.rs` と `apps/postgres/driver/src/grpc_client.rs` が `#[allow(dead_code)]` 1 行差でほぼ同一 | 同上 | 中 |
| D-08 | `tauriInvoke` ラッパ / `formatError` / `normalizeId` / `clearMessages` がドライバ UI 静的資産間で重複 | `apps/joywatcher/ui/assets/app.js`, `apps/postgres/ui/assets/app.js` | 中 |
| D-09 | `formatBytes` / `formatPercent` / `formatByteRate` 等の表示フォーマッタが `DashboardContent.svelte` に直書き、他箇所での再利用余地あり | `src/lib/components/layout/three-pane/DashboardContent.svelte` | 低 |
| D-10 | `await reloadDrivers(); await reloadScanGroups(); await reloadTags();` の三連リロードが 3 箇所で重複 | `ThreePane.svelte`, `DriverDetailPanel.svelte`, `driverUiActions.ts` | 低 |
| D-11 | `src/lib/ipc/*.ts` の薄ラッパが 20+ 関数 ＝ ほぼ `invoke<T>('cmd', args)` のみ。共通エラー整形/通知フック導入余地 | `src/lib/ipc/**/*.ts` | 低 |

---

## 2. ターゲットアーキテクチャ（移動先）

### 2.1 Rust: `src-tauri/src/commands/`

```text
commands/
  mod.rs
  dto/                          ← R-BE-01 で新設（dto.rs を分割）
    mod.rs                      ← pub use 集約のみ
    tag.rs
    driver.rs
    publisher.rs
    subscriber.rs               ← mqtt-monitor 系
    metrics.rs
    driver_ui.rs                ← Launch / Import / CheckResult 系
    runtime.rs
    common.rs                   ← ErrorResponse 等
  driver/
    mod.rs
    crud.rs                     ← R-BE-04 で薄くする
    ui_launcher/                ← R-BE-02
      mod.rs
      command.rs                ← #[tauri::command] 定義のみ
      session.rs                ← セッション解決・多重起動防止
      paths.rs                  ← 既存 ui_paths.rs を取り込み
      tempfile.rs               ← 入出力 json パス生成
    import/                     ← R-BE-03
      mod.rs
      command.rs
      validate.rs               ← validate_import_request / validate_and_convert_payload
      apply.rs                  ← driver_configs / scan_groups / tags / registry への反映
    runtime_sync.rs             ← 既存維持
    toml_io.rs                  ← 既存維持
  publisher/                    ← 既存（行数 OK、現状維持）
  subscriber/
    mod.rs
    monitor/                    ← R-BE-05
      mod.rs
      command.rs                ← #[tauri::command]
      tree.rs                   ← MutableTopicNode と組み立て
      window.rs                 ← open_mqtt_monitor_window
  metrics/                      ← R-BE-06（任意。現状 295 行で許容範囲だが platform 別を分けると見通し向上）
    mod.rs
    command.rs
    windows.rs                  ← #[cfg(windows)] 実装
    fallback.rs                 ← それ以外
  runtime.rs                    ← 既存維持
  logs.rs                       ← 既存維持
  tag.rs                        ← 既存維持
```

### 2.2 Rust: `src-tauri/src/app_state.rs`

- **方針**: 構造体の場所は変えない（互換を最大化）。フィールドを「ドメイン別サブ構造体」に**まとめるだけ**を別タスクで提案。
- まずは `state_types.rs` を新設し、`DriverUiSessionState` / `MqttMonitor*State` / `*RuntimeMetric*State` 等の補助型のみ外出しする（R-BE-07）。

### 2.3 Frontend: `src/lib/components/`

```text
components/
  layout/
    ThreePane.svelte            ← R-FE-01 で薄くする（200行未満目標）
    three-pane/
      panes/                    ← R-FE-02
        DashboardContent.svelte ← 残骸（< 200 行）
        dashboard/              ← 新設
          MetricCard.svelte
          RuntimeStatusCard.svelte
          DriverMetricsTable.svelte
          ScanCycleHealthCard.svelte
          formatters.ts         ← formatBytes/Percent/ByteRate/cpuLevel/ioLevel
        LogsContent.svelte
        logs/                   ← R-FE-03
          LogFilterBar.svelte
          LogTable.svelte
          logFilters.ts
        PublishersContent.svelte
        publishers/             ← R-FE-04
          PublisherList.svelte
          PublisherEditor.svelte
  tag/
    TagTree.svelte              ← R-FE-05 で薄くする
    tag-tree/                   ← 新設
      DriverNode.svelte
      ScanGroupNode.svelte
      TagNode.svelte
      ContextMenu.svelte
      treeBuilder.ts            ← groupedTree の組立て
  driver/
    DriverDetailPanel.svelte    ← R-FE-06 で薄く
    driver-detail/
      ConnectionForm.svelte
      MetricsSection.svelte
    PostgresRegistrationPanel.svelte ← R-FE-07
    postgres-registration/
      ConnectionFields.svelte
      TableSelector.svelte
      ColumnMapping.svelte
  mqtt-monitor/
    MqttMonitorWindow.svelte    ← R-FE-08 で薄く
    monitor/                    ← 新設
      ControlBar.svelte
      TopicTreePanel.svelte
      DetailPanel.svelte
      monitorPolling.ts
```

### 2.4 ドライバ UI 静的資産: `apps/<name>/ui/assets/`

- `app.js` を機能別 ESM に分割（`<script type="module">` 読込）。
  - 共通: `apps/<name>/ui/assets/lib/tauri.js`（invoke ラッパ + formatError）
  - 例 (joywatcher): `connection.js` / `tag-browser.js` / `type-probe.js` / `scan-groups.js` / `import-builder.js`
- 既存 `index.html` の参照を `<script type="module" src="./app.js">` のままにし、`app.js` を**エントリ点だけ**に縮小する。

---

## 3. タスク一覧（独立実行可能・着手順）

各タスクは「目的 / 対象 / 手順 / 受け入れ条件 / 検証」を持つ。`Depends` がないものは並行着手可。

### R-BE-01: `commands/dto.rs` を機能別に分割

- **Depends**: なし
- **対象**: `src-tauri/src/commands/dto.rs`
- **手順**:
  1. `commands/dto/` ディレクトリを作る。
  2. 既存型を以下に分配:
     - `tag.rs`: `TagPayload` / `TagDto` / `CreateTagRequest` / `ScanGroupDto` / `TagValueDto`
     - `driver.rs`: `DriverDto` / `SaveDriverRequest`
     - `publisher.rs`: `PublisherDto` / `SavePublisherRequest`
     - `subscriber.rs`: `MqttMonitor*Dto` / `Start*Request` / `Get*Request` 等
     - `metrics.rs`: `AppMetricsDto` / `DriverMetricsDto` / `RuntimeStatusDto`
     - `driver_ui.rs`: `LaunchDriverUi*` / `CheckDriverUiResult*` / `ImportDriverUi*`
     - `common.rs`: `ErrorResponse`
  3. `dto/mod.rs` で `pub use` 集約し、既存 `use crate::commands::dto::*;` を一切変更しないで済むようにする。
  4. 元 `dto.rs` を削除。
- **受け入れ条件**:
  - 全ファイル 200 行以下。
  - `cargo check` / `cargo test` / `cargo clippy -D warnings` 通過。
  - `commands/dto::*` の外向き API（pub use されるシンボル）に変更なし。
- **検証**: `rg "use crate::commands::dto" src-tauri` の差分 0 件。

### R-BE-02: `commands/driver/ui_launcher.rs` 分割

- **Depends**: なし（R-BE-01 と並行可）
- **対象**: `src-tauri/src/commands/driver/ui_launcher.rs` (366) と `ui_paths.rs`
- **手順**:
  1. `commands/driver/ui_launcher/` を作成。
  2. `command.rs` に `#[tauri::command]` (`launch_driver_ui` / `check_driver_ui_available` / `check_driver_ui_result`) のみ残す（薄い orchestration）。
  3. セッション解決（要求 → driver_id / driver_type / exe_path 解決、多重起動防止）を `session.rs` に純粋関数として切り出す。
  4. 入出力 json パス生成を `tempfile.rs` に切り出す（`std::env::temp_dir()` 依存だけ）。
  5. `ui_paths.rs` を `paths.rs` として配下に移動。`super::ui_paths::*` の参照箇所を全置換。
- **受け入れ条件**: 各ファイル 200 行以下、`main.rs` の `invoke_handler!` 変更なし。
- **検証**: `npm run tauri dev` でドライバ UI 起動が従前どおり動く（手動）/ `cargo test` 通過。

### R-BE-03: `commands/driver/import.rs` 分割

- **Depends**: R-BE-01（DTO 参照のため。並行も可だが衝突注意）
- **対象**: `src-tauri/src/commands/driver/import.rs` (337)
- **手順**:
  1. `commands/driver/import/` を作成。
  2. `command.rs`: `#[tauri::command] import_driver_ui_result` のみ。
  3. `validate.rs`: `validate_import_request` / `validate_and_convert_payload`（純粋関数化）。
  4. `apply.rs`: `driver_configs` / `scan_groups` / `tags` / `registry` への適用 + `sync_driver_runtime` 呼び出し。
  5. **副作用順序**: 既存どおり「(a) toml 更新 → (b) registry/state 更新 → (c) `sync_driver_runtime`」を**変更しない**。順序変更は別タスク（`R-BE-FIX-01` 候補、本リファクタの範囲外）。
- **受け入れ条件**: 既存テスト全通過。手動: ドライバ UI 取込のハッピーパス確認。

### R-BE-04: `commands/driver/crud.rs` の薄化

- **Depends**: R-BE-01
- **対象**: `commands/driver/crud.rs` (325)
- **手順**:
  1. CRUD 内のビジネスロジック（id 正規化・重複検査・toml 反映準備など）を `crud/logic.rs`（純粋関数）に切り出す。
  2. `commands` 関数（`list_drivers` / `save_driver` / `delete_driver`）は `command.rs` に残し、`state` 操作 + `logic` 呼び出しのみに整理。
  3. `runtime_sync.rs` / `toml_io.rs` への呼び出しは現状維持。
- **受け入れ条件**: 各ファイル 200 行以下、API 互換、`cargo test` 通過。

### R-BE-05: `subscriber/monitor.rs` 分割

- **Depends**: R-BE-01
- **対象**: `src-tauri/src/commands/subscriber/monitor.rs` (349)
- **手順**:
  1. `commands/subscriber/monitor/` を作成。
  2. `command.rs`: `#[tauri::command]` 群（`start_mqtt_monitor` / `stop_mqtt_monitor` / `list_*` / `get_*` / `clear_*`）。
  3. `tree.rs`: `MutableTopicNode` と「メッセージ → ツリー組立」純粋ロジック（ユニットテスト追加機会）。
  4. `window.rs`: `open_mqtt_monitor_window`（`tauri::WebviewWindowBuilder` 周辺）。
- **受け入れ条件**: MQTT モニタウィンドウが従前どおり開閉 / メッセージ表示できる。ユニットテスト 1 件以上（ツリー組立）。

### R-BE-06: `commands/metrics.rs` の platform 分割（任意）

- **Depends**: R-BE-01
- **対象**: `commands/metrics.rs` (295)
- **手順**: `#[cfg(target_os = "windows")]` ブロックを `metrics/windows.rs` に隔離、フォールバックを `metrics/fallback.rs` に。`command.rs` から両者を呼ぶだけ。
- **受け入れ条件**: Windows / Linux/Mac で `cargo check` 通過。挙動互換。

### R-BE-07: `app_state.rs` 補助型の外出し

- **Depends**: なし
- **対象**: `src-tauri/src/app_state.rs` (158)
- **手順**:
  1. `app_state/` ディレクトリ作成、`mod.rs` に `AppState` 本体だけ残す。
  2. `driver_ui_session.rs`（`DriverUiSessionState`）、`mqtt_monitor.rs`（`MqttMonitor*State`）、`scan_metrics.rs`（`ScanGroupRuntimeMetricState` + `DriverIoSampleState` + `DriverIoTotalState`）、`app_cpu.rs`（`AppCpuSampleState` + `RuntimeMetricsCacheState`）に分配。
  3. `app_state::*` で全 re-export し、外部 use を変えない。
- **受け入れ条件**: `rg "use crate::app_state" src-tauri` の参照変更 0、`cargo test` 通過。

### R-FE-01: `ThreePane.svelte` の薄化

- **Depends**: なし（R-FE-02 とは独立だが、関連は強い）
- **対象**: `src/lib/components/layout/ThreePane.svelte` (465)
- **手順**:
  1. 既に `three-pane/` 配下に分割の素地あり。`ThreePane` 内の state を「ハンドラ群」「IPC オーケストレーション」「UI 結線」に分け、IPC オーケストレーションを `three-pane/orchestrators/` 配下の `.ts` に抽出する。
  2. `reloadTagManagementData` / `handleDriverSaved` / `ensureDriverUiNotBusy` 等は `orchestrators/tagManagement.ts` 等へ移し、`ThreePane.svelte` からは呼ぶだけにする。
  3. 既存 `controllerFactory.ts` の方針（クロージャ集合返却）を踏襲する。
- **受け入れ条件**: `ThreePane.svelte` 250 行以下、`npm run check` 通過、手動でタブ切替・ドライバ UI 起動・取込が動く。

### R-FE-02: `DashboardContent.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/layout/three-pane/DashboardContent.svelte` (778)
- **手順**:
  1. フォーマッタ群（`formatBytes` / `formatPercent` / `formatByteRate` / `cpuLevel` / `ioLevel`）を `dashboard/formatters.ts` に純粋関数として抽出（**ユニットテスト追加候補**）。
  2. UI セクションを `MetricCard.svelte` / `RuntimeStatusCard.svelte` / `DriverMetricsTable.svelte` / `ScanCycleHealthCard.svelte` に分割。
  3. `DashboardContent.svelte` は props 受け取り + サブコンポーネント結線のみ（< 200 行）。
- **受け入れ条件**: `npm run check` 通過、Dashboard 表示崩れなし（手動確認）、formatters ユニットテスト 1 ファイル以上。

### R-FE-03: `LogsContent.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/layout/three-pane/LogsContent.svelte` (397)
- **手順**:
  1. フィルタ UI を `logs/LogFilterBar.svelte`、表本体を `logs/LogTable.svelte` に分割。
  2. フィルタ条件の整形ロジックを `logs/logFilters.ts` に純粋関数化。
- **受け入れ条件**: 行数規約準拠、ログ画面挙動互換。

### R-FE-04: `PublishersContent.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/layout/three-pane/PublishersContent.svelte` (335)
- **手順**: 一覧 / 編集を `publishers/PublisherList.svelte` / `PublisherEditor.svelte` に分割。

### R-FE-05: `TagTree.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/tag/TagTree.svelte` (559)
- **手順**:
  1. `groupedTree` 構築を `tag-tree/treeBuilder.ts` に純粋関数化（**テスト容易化**）。
  2. ノード描画を `DriverNode.svelte` / `ScanGroupNode.svelte` / `TagNode.svelte` に分割。
  3. 文脈メニューを `ContextMenu.svelte` に切り出す。
- **受け入れ条件**: 行数規約準拠、ツリー操作（展開・選択・右クリック）動作互換、`treeBuilder` ユニットテスト 1 件以上。

### R-FE-06: `DriverDetailPanel.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/driver/DriverDetailPanel.svelte` (404)
- **手順**: 接続フォームと監視メトリクスを別コンポーネント化。

### R-FE-07: `PostgresRegistrationPanel.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/driver/PostgresRegistrationPanel.svelte` (415)
- **手順**: 接続フィールド / テーブル選択 / カラムマッピングを分割。

### R-FE-08: `MqttMonitorWindow.svelte` 分割

- **Depends**: なし
- **対象**: `src/lib/components/mqtt-monitor/MqttMonitorWindow.svelte` (627)
- **手順**:
  1. ポーリング処理（`refreshSelectedDetail` 系・`highlight` 系・タイマ）を `monitor/monitorPolling.ts` に隔離。
  2. UI を `ControlBar.svelte` / `TopicTreePanel.svelte` / `DetailPanel.svelte` に分割。

### R-FE-09: ドライバ UI 静的資産分割

- **Depends**: なし
- **対象**:
  - `apps/joywatcher/ui/assets/app.js` (1105)
  - `apps/postgres/ui/assets/app.js` (725)
- **手順**:
  1. `<script type="module">` に変更し、機能別 ESM へ分割。
  2. 共通 invoke ラッパ・`formatError` を `assets/lib/` に抽出。
  3. **重要**: `index.html` の相対パス、Tauri の `frontendDist` 設定、`tauri.conf.json` の CSP（`script-src 'self'`）に**追加読込が違反しないか**事前確認。CDN は禁止。
  4. ビルド/インストールスクリプト（`scripts/build-dev-*-ui.ps1` / `install-driver-ui.ps1`）への影響確認。assets が追加されるだけならスクリプト改修不要。
- **受け入れ条件**: 各ファイル 300 行以下、ドライバ UI が従前どおり起動・登録できる（手動）。

### R-RS-01: `apps/joywatcher/bridge-x86/src/dll_api.rs` 分割

- **Depends**: なし
- **対象**: `dll_api.rs` (583)
- **手順**:
  1. FFI 型定義（型エイリアス・`JoyWatcherComData1` 等）→ `dll_ffi.rs`
  2. シンボル解決・呼び出しラッパ → `dll_symbols.rs`
  3. 高レベル API（`connect` / `disconnect` / `resolve` / `read` 等）→ `dll_api.rs` に残す
- **受け入れ条件**: 行数規約準拠、x86 bridge ビルド通過（`scripts/build-dev-joywatcher-bridge-x86.ps1`）。
- **注意**: `unsafe` ブロックの境界を変えない（FFI 周りはバグ温床）。

### R-RS-02: `apps/joywatcher/ui/src/joywatcher_bridge_client.rs` 分割

- **Depends**: なし
- **対象**: 584 行
- **手順**: `protocol.rs`（行指向プロトコル送受信）/ `commands.rs`（高レベル API: `resolve_single_tag` / `browse_tags` / `probe_tag_types`）/ `process.rs`（子プロセス起動・終了）に分割。
- **受け入れ条件**: 行数規約準拠、UI 側ビルド通過。

### R-RS-03: `apps/joywatcher/driver/src/joywatcher_bridge.rs` 分割

- **Depends**: なし
- **対象**: 473 行
- **手順**: R-RS-02 と同方針（送受信 / 高レベル / プロセス管理）。可能なら `protocol.rs` を ui 側と共有可能か検討（**別タスク** `R-RS-04` 候補、本書範囲外）。
- **受け入れ条件**: 行数規約準拠、ランタイムビルド通過。

### R-DOC-01: 本書の進行表更新

- 各タスク完了時に「セクション 4. 進行ログ」へ完了日とコミット ID を追記する。
- 派生タスクや前提崩れを発見した場合は「セクション 5. 発見メモ」に記録。

---

### 3.4 重複削減・共通化タスク（R-DEDUP-XX）

> **共通方針**
>
> - **挙動・シリアライズ表現を変えてはならない**（§0.2）。共通化前後で `cargo test` / `npm run check` / 手動 invoke の戻り JSON が同一であることを確認する。
> - 共通化は「明らかな重複の事実上の同一実装」のみを対象とする。似て非なるロジック（例: ドライバごとの探索順）は**触らない**。
> - 共通モジュールを新設する場合、まず**1 件の呼び出し元のみ**を移行 → ビルド通過確認 → 他呼び出し元を順次切り替え、の刻みで進める。

#### R-DEDUP-01: `write_tags_toml_atomic` 重複排除（最優先）

- **Priority**: 高（バグ温床）
- **Depends**: なし
- **対象**:
  - `src-tauri/src/commands/driver/toml_io.rs:31`（正本として残す）
  - `src-tauri/src/commands/tag.rs:215`（重複定義。`TagsTomlFile` / `resolve_config_dir` も同様に重複）
- **手順**:
  1. `commands/driver/toml_io.rs` の `write_tags_toml_atomic` / `TagsTomlFile` / `resolve_config_dir` / `replace_file_atomically` の可視性を `pub(super)` → `pub(crate)` に引き上げる（一時的）。
  2. `commands/tag.rs` から重複定義を削除し、`use crate::commands::driver::toml_io::write_tags_toml_atomic;` に置換。
  3. `cargo build` / `cargo test` 通過確認。
  4. R-DEDUP-02 で正式な共通モジュールへ移設するため、可視性昇格はそのまま維持。
- **受け入れ条件**: 重複定義が消滅し、`tags.toml` の書き出し結果が変わらない（同一の TOML テキスト）。
- **検証**: 既存 tag CRUD を 1 件実行し、`config/tags.toml` の diff が空であること。

#### R-DEDUP-02: `commands/config_io/` 新設（TOML I/O 共通化）

- **Priority**: 中
- **Depends**: R-DEDUP-01
- **対象**:
  - `src-tauri/src/commands/driver/toml_io.rs`
  - `src-tauri/src/commands/publisher/toml_io.rs`
- **手順**:
  1. `src-tauri/src/commands/config_io/mod.rs` を新設。`resolve_config_dir`, `replace_file_atomically`, `write_toml_atomic<T: Serialize>(file_name, value) -> Result<(), ErrorResponse>` を提供。
  2. `driver/toml_io.rs` の `write_tags_toml_atomic` / `write_drivers_toml_atomic` を `config_io::write_toml_atomic` を使う実装に書き換え（`TagsTomlFile` / `DriversTomlFile` 構造体は driver 側に残す）。
  3. `publisher/toml_io.rs` も同様に書き換え。
  4. 既存の `pub(super)` 可視性を最小に戻す。
- **受け入れ条件**: 行数削減（合計 -80 行目安）、3 種の TOML 書き出し結果が完全一致。
- **検証**: drivers/tags/publishers 各 CRUD を 1 件ずつ実行し、TOML diff が空。

#### R-DEDUP-03: `ErrorResponse` コンストラクタ + `error_code` 定数モジュール

- **Priority**: 中
- **Depends**: R-BE-01（dto 分割）
- **対象**: `src-tauri/src/commands/dto/common.rs`（R-BE-01 後の配置）と全コマンド
- **手順**:
  1. `dto/common.rs` に以下を追加（**既存 `ErrorResponse` のフィールド構造は変えない**）:

     ```rust
     impl ErrorResponse {
         pub fn new(code: impl Into<String>, error: impl Into<String>) -> Self {
             Self { error: error.into(), code: code.into() }
         }
         pub fn invalid_input(error: impl Into<String>) -> Self { Self::new(error_code::INVALID_INPUT, error) }
         pub fn not_found(error: impl Into<String>) -> Self { Self::new(error_code::NOT_FOUND, error) }
         pub fn io_error(error: impl Into<String>) -> Self { Self::new(error_code::IO_ERROR, error) }
         pub fn serialize_error(error: impl Into<String>) -> Self { Self::new(error_code::SERIALIZE_ERROR, error) }
         // 他は必要に応じて追加
     }

     pub mod error_code {
         pub const INVALID_INPUT: &str = "INVALID_INPUT";
         pub const NOT_FOUND: &str = "NOT_FOUND";
         pub const IO_ERROR: &str = "IO_ERROR";
         pub const SERIALIZE_ERROR: &str = "SERIALIZE_ERROR";
         // 既存コードを grep して全列挙
     }
     ```

  2. 既存の `ErrorResponse { error: ..., code: "XXX".to_string() }` を 1 ファイルずつコンストラクタ呼び出しに置換。
  3. **コード文字列値は絶対に変えない**（FE 側で `code` 判定している箇所があるため）。
- **受け入れ条件**: `grep -n 'code: "' src-tauri/src/commands/` がコンストラクタ実装 1 箇所のみ。
- **検証**: 単体テストで `ErrorResponse::invalid_input("x")` の JSON 出力が従来同等であること。

#### R-DEDUP-04: `Shared<T>` 型エイリアス導入

- **Priority**: 低
- **Depends**: R-BE-07
- **対象**: `src-tauri/src/app_state.rs`, `src-tauri/src/core/mod.rs`
- **手順**:
  1. `app_state.rs`（または R-BE-07 で分割したサブモジュール）冒頭に `pub type Shared<T> = std::sync::Arc<tokio::sync::RwLock<T>>;` を定義し、`fn shared<T>(v: T) -> Shared<T> { std::sync::Arc::new(tokio::sync::RwLock::new(v)) }` を併設。
  2. 16 箇所の `Arc::new(tokio::sync::RwLock::new(...))` を `shared(...)` に、型注釈 `Arc<RwLock<T>>` を `Shared<T>` に書き換え。
- **受け入れ条件**: 型エイリアスのみで、ランタイム挙動・スレッド安全性に変化なし。

#### R-DEDUP-05: DTO serde 規約統一（表現は不変）

- **Priority**: 低
- **Depends**: R-BE-01
- **対象**: `src-tauri/src/commands/dto/**`
- **手順**:
  1. FE 連携 DTO 全てに `#[serde(rename_all = "camelCase")]` を明示。**ただし `rename_all` を新規付与する際は、付与前後のフィールド名が同一であることを必ず確認**（既存命名が偶然 snake_case と camelCase で同形のケースを変化させないため、必要なら個別フィールドで `#[serde(rename = "…")]` を残す）。
  2. 各 DTO に `#[derive(Debug, Clone, Serialize, Deserialize)]` 一式を統一付与（不要 derive は付けない）。
  3. 念のため、代表的な DTO に `serde_json::to_value` の固定値を比較する snapshot 単体テストを追加。
- **受け入れ条件**: FE 受信 JSON が完全同一（手動でも diff 確認）。

#### R-DEDUP-06: Svelte `$lib/utils/format.ts` 集約

- **Priority**: 低
- **Depends**: なし（R-FE-02 と並行可）
- **対象**: `src/lib/components/layout/three-pane/DashboardContent.svelte`
- **手順**:
  1. `src/lib/utils/format.ts` を新設し、`formatBytes` / `formatPercent` / `formatByteRate` / `cpuLevel` / `ioLevel` を移設（型を明示）。
  2. `DashboardContent.svelte` から import に切り替え、ロジック削除。
- **受け入れ条件**: ダッシュボード表示の文字列が完全同一。

#### R-DEDUP-07: ドライバ UI 静的資産の共通ライブラリ化

- **Priority**: 低
- **Depends**: なし（R-FE-09 と並行可）
- **対象**: `apps/joywatcher/ui/assets/app.js`, `apps/postgres/ui/assets/app.js`
- **手順**:
  1. 共通化方針を**先に決める**: (a) 各ドライバ UI 配下に `assets/lib/tauri.js` を置きビルド時に同期コピー、(b) `apps/_shared/ui-assets/` を新設し ESM `import` で参照、のいずれか（CSP `'self'` を守りつつ）。
  2. `tauriInvoke` ラッパ / `formatError` / `normalizeId` / `clearMessages` を抽出。
  3. 既存 `index.html` の `<script>` 構成・読み込み順を変えない範囲で適用。
- **受け入れ条件**: ドライバ UI のオフライン起動・登録動作が従来同等。
- **注意**: 外部 CDN 禁止（[copilot-instructions.md](../.github/copilot-instructions.md) §3.6）。

#### R-DEDUP-08: `normalize_optional_string` 重複排除

- **Priority**: 中
- **Depends**: なし
- **対象**:
  - `src-tauri/src/commands/driver/ui_paths.rs:117`（正本として残す候補）
  - `src-tauri/src/commands/driver/crud.rs:136`
  - `src-tauri/src/commands/runtime.rs:199`
- **手順**:
  1. `commands/util.rs` （無ければ新設）に `pub(crate) fn normalize_optional_string(value: Option<String>) -> Option<String>` を一本化。
  2. 既存 3 箇所を `use crate::commands::util::normalize_optional_string;` に置換。
- **受け入れ条件**: 関数定義が 1 箇所のみ。挙動完全互換。

#### R-DEDUP-09: JoyWatcher 探索ロジック共通化

- **Priority**: 中
- **Depends**: なし（apps 内クレート構成変更を伴う場合は R-RS-XX 系と相談）
- **対象**:
  - `apps/joywatcher/ui/src/joywatcher_bridge_client.rs`
  - `apps/joywatcher/driver/src/joywatcher_bridge.rs`
  - `apps/joywatcher/driver/src/joywatcher_artifacts.rs`
  - `apps/joywatcher/bridge-x86/src/dll_api.rs`（DLL 探索部分のみ）
- **手順**:
  1. **新規クレート不要案**: `apps/joywatcher/` 配下に `common/` モジュール（または `joywatcher-common` 内部クレート）を作り、`BRIDGE_EXE_NAME` / `DLL_FILE_NAME` / `push_unique` / 候補パス探索の純粋関数を集約。
  2. 各呼び出し元を順次置換。**探索順は現状の順序を厳守**（順序差で見つかる DLL が変わるため）。
  3. ユニットテストとして「ダミーディレクトリ群で期待する候補リストが返る」を 1 ケース追加。
- **受け入れ条件**: x86 ブリッジ / x64 ドライバ / UI 全てが従来と同じ DLL / EXE を解決する。
- **注意**: bridge-x86 は i686 ターゲット。共通モジュールが `cfg(target_arch)` 依存を持たないこと。

#### R-DEDUP-10: `apps/*/driver/src/grpc_client.rs` 統合

- **Priority**: 中
- **Depends**: R-DEDUP-09 の方針確定（共通モジュール配置場所）
- **対象**: `apps/joywatcher/driver/src/grpc_client.rs`, `apps/postgres/driver/src/grpc_client.rs`
- **手順**:
  1. 両ファイルが事実上同一（差分は `#[allow(dead_code)]` のみ）であることを再確認。
  2. `packages/protocol-rs/` 配下、または新設 `packages/driver-runtime-client/` に `DriverRuntimeClient` を集約（protocol-rs に置く場合は依存追加を最小化）。
  3. 各 driver の `main.rs` の use を差し替え、`grpc_client.rs` を削除。
- **受け入れ条件**: 両ドライバが従来通りランタイムに接続でき、`cargo build -p` で双方ビルド通過。
- **検証**: 統合後の `DriverRuntimeClient::connect` を双方の起動シナリオで 1 回実行。

#### R-DEDUP-11: ストア三連リロード共通化

- **Priority**: 低
- **Depends**: なし
- **対象**: `src/lib/components/layout/ThreePane.svelte:89`, `src/lib/components/driver/DriverDetailPanel.svelte:81`, `src/lib/components/layout/three-pane/driverUiActions.ts:108`
- **手順**:
  1. `src/lib/stores/index.ts` に `export async function reloadAllRegistry(): Promise<void>` を追加し、内部で `reloadDrivers` → `reloadScanGroups` → `reloadTags` を順次呼ぶ。
  2. 3 箇所を `await reloadAllRegistry();` に置換。
- **受け入れ条件**: 呼び出し順・例外伝播が完全同一。

#### R-DEDUP-12: `src/lib/ipc/_invoke.ts` 共通ラッパ（任意）

- **Priority**: 低（過剰実装にならない範囲で）
- **Depends**: なし
- **対象**: `src/lib/ipc/**/*.ts`
- **手順**:
  1. `src/lib/ipc/_invoke.ts` を新設し、`export const ipcInvoke = <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);` を定義（将来の trace/通知フック挿入点として用意）。
  2. 既存ラッパを 1 ファイルのみ移行し効果検証 → OK なら順次置換。**1 件あたり 1 コミット**。
  3. **挙動と例外型を変えてはならない**（既存 try/catch・`extractErrorMessage` の呼び出しに影響しないこと）。
- **受け入れ条件**: 全 invoke ラッパが `ipcInvoke` 経由になり、エラー時の挙動・引数構造が完全互換。
- **判断**: ラッパが本当に「単なる呼び出し」のみなら、無理に共通化しない選択肢も可（YAGNI）。**R-FE タスク群の進捗を見てから判断**。

---

## 4. 進行ログ（着手者が更新）

| タスク | 着手者 | 状態 | コミット / PR | 備考 |
| --- | --- | --- | --- | --- |  |
| R-BE-01 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/dto.rs` を `commands/dto/` へ分割 |
| R-BE-02 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/driver/ui_launcher.rs` を `ui_launcher/` へ分割 |
| R-BE-03 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/driver/import.rs` を `import/` へ分割 |
| R-BE-04 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/driver/crud.rs` を `crud/` へ分割 |
| R-BE-05 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/subscriber/monitor.rs` を `monitor/` へ分割（treeテスト追加） |
| R-BE-06 | - | 未着手 | - | 任意 |
| R-BE-07 | Copilot | 完了（ローカル） | - | 2026-05-18: `app_state.rs` を `app_state/` へ分割（補助型外出し） |
| R-FE-01 | - | 未着手 | - | - |
| R-FE-02 | - | 未着手 | - | - |
| R-FE-03 | - | 未着手 | - | - |
| R-FE-04 | - | 未着手 | - | - |
| R-FE-05 | - | 未着手 | - | - |
| R-FE-06 | - | 未着手 | - | - |
| R-FE-07 | - | 未着手 | - | - |
| R-FE-08 | - | 未着手 | - | - |
| R-FE-09 | - | 未着手 | - | - |
| R-RS-01 | - | 未着手 | - | - |
| R-RS-02 | - | 未着手 | - | - |
| R-RS-03 | - | 未着手 | - | - |
| R-DEDUP-01 | Copilot | 完了（ローカル） | - | 2026-05-18: `write_tags_toml_atomic` 重複定義を排除 |
| R-DEDUP-02 | Copilot | 完了（ローカル） | - | 2026-05-18: `commands/config_io` へ TOML I/O 共通化 |
| R-DEDUP-03 | Copilot | 完了（ローカル） | - | 2026-05-18: `ErrorResponse` コンストラクタ共通化と `commands` 側置換 |
| R-DEDUP-04 | - | 未着手 | - | R-BE-07 後 |
| R-DEDUP-05 | - | 未着手 | - | R-BE-01 後 |
| R-DEDUP-06 | - | 未着手 | - | R-FE-02 と並行可 |
| R-DEDUP-07 | - | 未着手 | - | R-FE-09 と並行可 |
| R-DEDUP-08 | - | 未着手 | - | - |
| R-DEDUP-09 | - | 未着手 | - | - |
| R-DEDUP-10 | - | 未着手 | - | R-DEDUP-09 後 |
| R-DEDUP-11 | - | 未着手 | - | - |
| R-DEDUP-12 | - | 未着手 | - | 任意（YAGNI 注意） |

---

## 5. 発見メモ（着手中に気付いた事項）

> 仕様の齟齬・隠れた依存・追加で潰すべき問題をここに記録する。
> 本リファクタの範囲外（=機能変更/挙動変更）と判断したものは **`OUT-OF-SCOPE`** ラベルを付け、別タスク化提案する。

- (空)

---

## 6. 受け入れ全体ゲート

すべてのタスク完了後に確認:

1. `npm run check` ／ `npm run lint` ／ `npm run build` （存在するもの）すべて通過。
2. `cd src-tauri && cargo fmt -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test`。
3. 行数規約: `rg -l '' --type rust src-tauri/src apps packages/*/src | xargs wc -l | awk '$1>300{print}'` で `mod.rs` / 例外明記済み以外が出ない。
4. Svelte / TS でも同等チェック。300 行超は先頭コメントで理由必須。
5. 手動シナリオ（最低限）:
   - アプリ起動 → ダッシュボード表示
   - ドライバ登録 UI 起動 → 取込
   - ランタイム start/stop
   - MQTT モニタウィンドウ開閉
   - タグツリー操作（展開・編集・削除）
6. `docs/refactor-plan.md` 進行ログがすべて「完了」になっていること。

---

## 7. アンチパターン（やってはいけないこと）

- 「ついでに」依存追加 / フォーマッタ変更 / 命名規則変更。
- Tauri コマンド名のリネーム（`invoke_handler!` を触る）。
- 機能改善・バグ修正の混入（見つけたら **発見メモ** に書いて別タスク化）。
- `pub use` で API を増やす（公開面は現状維持）。
- 純粋関数の振りをして `tauri::State` を取り回す（純粋化はあくまでロジック単位で）。
- 大量ファイル一括移動 PR（レビュー不能になる）。

---

## 8. 参考ポインタ

- 設計書: [docs/design.md](design.md)
- アーキテクチャ: [docs/architecture.md](architecture.md)
- ドライバ実装フロー: [docs/driver-implementation-flow.md](driver-implementation-flow.md)
- 既存テンプレ: [docs/templates/](templates/)
- リポジトリメモ: `/memories/repo/kt_iot_hub.md`（実装時参照すべき暗黙ルール）
