# 互換性維持コード削除計画

> **作成日**: 2026-05-20  
> **対象バージョン**: v0.5.0 以降  
> **前提**: 本ドキュメントは [`docs/refactor-plan.md`](refactor-plan.md) の全タスク（R-BE/R-FE/R-RS/R-DEDUP）完了後の、
> 後方互換性コードを段階的に削除するための計画書である。

---

## 0. 不変条件（削除計画にも適用）

以下は削除作業中も絶対に変更してはならない:

- Tauri コマンド名 / 引数 DTO の JSON 表現（`#[serde(rename_all = "camelCase")]` 含む）
- gRPC proto（`core/src-tauri/proto/*.proto`）
- `ops/config/*.toml` のキー名
- ログの grep キー（削除する場合は本ドキュメントの §4 に追記）

---

## 1. 概要と削除方針

現在のコードベースには、以下の状況でコメント付きで維持している後方互換コードが存在する:

| カテゴリ | 互換の対象バージョン | 削除可能になる条件 |
| --- | --- | --- |
| UIパス旧構成探索 | v0.3 以前 (`driver-ui/`) | 全運用環境で `ops/driver-ui/` 配置に移行済み |
| UIパス祖先ディレクトリ探索 | 開発時ワークツリー探索 | `driverUiBaseDir` 設定による明示的パス指定が定着 |
| manifest fallback（ランタイム） | manifest がない旧配布 | 全配布物に `driver-manifest.json` を同梱済み |
| JoyWatcher 設定 camelCase キー | v0.3 以前の `drivers.toml` | 全既存設定が `user_id` (snake_case) を使用 |
| ドライバタイプ ハードコード（FE） | manifest-driven discovery 未完成期 | manifest discovery が UI で完全に機能 |
| インプロセス UI 分岐（FE） | `postgres` のみインプロセス UI | manifest の `ui_mode` で動的判定可能になる |
| MQTT 配信モード旧エイリアス | v0.4.x の互換入力 | `on_change` のみを正式化し、旧別名を削除済み |
| MQTT topic 旧キー互換 | `topic_prefix` を使っていた設定 | `topic` のみへ統一済み |

manifest-driven discovery の移行計画（[`docs/driver-manifest-discovery-design.md §11`](driver-manifest-discovery-design.md)）との対応:

- **Phase 1 完了**（v0.4.0）: manifest 仕様 v1 定義・Discovery コマンド追加
- **Phase 2 完了**（v0.4.1）: ドライバ種別候補を Discovery 由来へ全面切替
- **Phase 3（v0.5.0）**: 埋め込み候補と legacy discovery を削除 → **本ドキュメントのスコープ**

---

## 2. 発見箇所一覧

### C-01: UIパス旧構成 `<root>/driver-ui/<type>/<file>` の探索削除

**対象ファイル**: `core/src-tauri/src/commands/driver/ui_launcher/paths.rs`

**削除対象①** — `find_default_driver_ui_path` 内の後方互換ブランチ:

```rust
// <root>/driver-ui/<type>/<file> (後方互換)
let app_root_style = root.join("driver-ui").join(driver_type).join(file_name);
if app_root_style.exists() {
    return Some(path_to_string(app_root_style));
}
```

> 新構成は `<root>/ops/driver-ui/<type>/` または `driverUiBaseDir` 設定値。

**削除対象②** — `manifest_candidates_for_root` 内の旧パス候補:

```rust
root.join("driver-ui").join(driver_type).join("driver-manifest.json"),
```

**前提条件**:

- `ops/driver-ui/` への全運用環境移行が完了していること
- `core/src-tauri/driver-ui/` (bundle staging) への同期スクリプトが引き続き動作すること

**影響範囲**: `find_default_driver_ui_path` / `manifest_candidates_for_root` の絞り込みのみ。外部コマンド API に変化なし。

**削除後の候補順序** (priority 1〜3 のみ残す):

1. `driverUiBaseDir` 設定値 / `DRIVER_BIN_DIR` 環境変数
2. exe 同居ディレクトリ（bundle 配布時）
3. ~~旧 `driver-ui/<type>/` （削除）~~
4. ~~祖先ディレクトリ探索（C-02 で削除）~~

---

### C-02: UIパス祖先ディレクトリ探索（優先順④）の削除

**対象ファイル**: `core/src-tauri/src/commands/driver/ui_launcher/paths.rs`

**削除対象** — `app_root_candidates` 内の優先順④ブロック（コメント「後方互換のための祖先探索」）:

```rust
// 優先順④: 後方互換のための祖先探索（current_dir / exe_dir）
if let Ok(current_dir) = std::env::current_dir() {
    let mut cursor = Some(current_dir.as_path());
    while let Some(path) = cursor {
        roots.push(path.to_path_buf());
        cursor = path.parent();
    }
}
// exe_dir 祖先も同様
```

> ルートまで全ディレクトリを候補に追加するロジック。パフォーマンス劣化・セキュリティリスクあり。

**前提条件**:

- C-01 が完了していること
- 全開発者が `driverUiBaseDir` を設定済みで、ワークツリー探索に依存しないこと

**セキュリティ上の観点**: この祖先探索は OS ルートまで探索するため、意図しない `driver-ui/` ディレクトリを読む可能性がある。削除はセキュリティ改善にもなる。

---

### C-03: ドライバランタイム manifest 無効時の legacy search fallback 削除（完了）

**対象ファイル**: `core/src-tauri/src/drivers/path_resolver.rs`

**削除対象** — `resolve_manifest_runtime_path_for_root` 内の legacy search フォールバックブロック:

```rust
// manifest 解析失敗時に legacy search へフォールバック
tracing::warn!(
    "Manifest runtime resolution fell back to legacy search: driver_type={} path={} reason={:?}",
    ...
);
// legacy search ロジック...
```

**削除後の挙動**:

- manifest が存在するが無効（parse error / schema error）の場合 → エラーとして上位に伝播
- manifest が存在しない場合 → 既存の同居配置候補探索へ継続（`ops/driver-ui/<type>/` → `driver-ui/<type>/` → `<type>/`）

**前提条件**:

- 全配布物（`ops/driver-ui/<type>/`）に有効な `driver-manifest.json` が同梱済みであること
- Discovery コマンド（`discover_driver_packages`）で事前に manifest 有効性を検証できること

---

### C-04: JoyWatcher 設定キー camelCase 互換の削除

**対象ファイル**: `drivers/joywatcher/driver/src/joywatcher_runtime.rs`

**削除対象①** — `CONNECTION_USER_ID_KEYS` から legacy キーを除去:

```rust
// 現状（削除前）
const CONNECTION_USER_ID_KEYS: &[&str] = &["user_id", "userId", "uid"];

// 削除後
const CONNECTION_USER_ID_KEYS: &[&str] = &["user_id"];
```

> `"userId"` (camelCase) と `"uid"` は v0.3 以前の設定フォーマット。

**削除対象②** — テスト関数の削除:

```rust
#[test]
fn accepts_legacy_camel_case_user_id_key() { ... }
```

**⚠ 破壊的変更の注意**:
これは**設定ファイルの互換性を壊す破壊的変更**。削除前に以下を必ず実施:

1. `ops/config/drivers.toml` 内の JoyWatcher 設定が `user_id` を使用していることを確認
2. 既存ユーザーへの移行ガイドを作成（または自動変換スクリプトを提供）
3. v0.4.x → v0.5.0 マイグレーションノートに記載

**確認コマンド**:

```powershell
# drivers.toml に "userId" または "uid" キーが残っていないことを確認
Select-String -Path ops/config/drivers.toml -Pattern 'userId|"uid"'
```

---

### C-05: フロントエンド ドライバタイプ ハードコード削除

**対象ファイル**: `core/src/lib/components/layout/three-pane/driverUiFlow.ts`

**削除対象** — `driverTypeLabel` / `driverTypeDescription` の hardcoded switch 文:

```typescript
function driverTypeLabel(driverType: string): string {
  switch (driverType) {
    case 'postgres':
      return 'PostgreSQL 接続先';  // ← ハードコード
    case 'joywatcher':
      return 'JoyWatcher 接続先';  // ← ハードコード
    default:
      return driverType;
  }
}

function driverTypeDescription(driverType: string): string {
  switch (driverType) {
    case 'postgres':
      return '接続先情報...';  // ← ハードコード
    case 'joywatcher':
      return 'TagSel2 ...';   // ← ハードコード
    default:
      return '専用UIで...';
  }
}
```

**削除後の代替**:

- `discovered.display_name` (manifest の `displayName`) をラベルとして使用
- 説明文は manifest の `version` / `vendor` / `capabilities` などのメタデータから組み立てる
- `discoveredPackages` が空の場合は `driverType` 文字列をそのまま表示（最小フォールバック）

**前提条件**:

- 全 `driver-manifest.json` に `displayName` と `description` が記載済みであること
- manifest discovery が UI で完全動作していること（`buildDriverTypeOptions` が `discoveredPackages` 優先で正しく動作）

**確認事項**: 現状の `buildDriverTypeOptions` はすでに `discovered?.display_name ?? driverTypeLabel(driverType)` という構造になっており、manifest がある場合は manifest 優先。manifest がない場合のフォールバックを削除する変更。

---

### C-06: `DriverUiHost.svelte` のハードコード `'postgres'` 分岐削除

**対象ファイル**: `core/src/lib/components/driver/DriverUiHost.svelte`

**削除対象** — `context.driverType === 'postgres'` のハードコード分岐:

```svelte
{#if context.driverType === 'postgres'}
  <PostgresRegistrationPanel {context} />
{:else}
  <div class="unsupported">...</div>
{/if}
```

**背景**: `DriverUiHost` は「インプロセスUI（PostgreSQL 設定パネル）」の分岐点。将来的にすべてのドライバが外部プロセス UI（`registration-ui.exe` 起動）に統一される場合、この分岐自体が不要になる。

**削除方針（2択）**:

| 方針 | 内容 | 前提 |
| --- | --- | --- |
| **A: インプロセスUI廃止** | `DriverUiHost` 自体を削除し、全ドライバを外部プロセス UI に統一 | `PostgresRegistrationPanel` の外部プロセス版を用意済み |
| **B: manifest `ui_mode` で動的切替** | manifest に `"ui_mode": "inprocess"` / `"external"` を追加し、動的に分岐 | manifest v2 仕様が確定後 |

> **方針 A を v0.5.0 で推奨**。`PostgresRegistrationPanel` はすでに Tauri IPC 経由で動作しており、外部プロセス化（`registration-ui.exe` 起動）に移行しやすい。

**前提条件**:

- Postgres ドライバが外部プロセス UI として動作する版を用意していること
- または、インプロセス UI のまま残す場合は `DriverUiHost` への `ui_mode` 追加（方針 B）を先に実装

---

### C-07: MQTT 配信モード旧エイリアス `"change_only"` / `"changed_only"` の削除

**対象ファイル**:

- `core/src-tauri/src/commands/publisher/crud.rs`（`normalize_publish_mode` 関数、line ~323）
- `core/src-tauri/src/publishers/mqtt.rs`（同名の重複定義）

**背景**: MQTT 配信モードを「SCAN周期に合わせて配信」と「変更があった場合のみ配信」の2モードに変更した際に、旧名称 `"change_only"` / `"changed_only"` → 正式名 `"on_change"` への正規化を互換維持のために追加した。

**削除対象** — `normalize_publish_mode` の legacy エイリアス:

```rust
// 現状（削除前）
fn normalize_publish_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "on_change" | "change_only" | "changed_only" => MQTT_PUBLISH_MODE_ON_CHANGE,
        _ => MQTT_PUBLISH_MODE_SCAN_INTERVAL,
    }
}

// 削除後
fn normalize_publish_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "on_change" => MQTT_PUBLISH_MODE_ON_CHANGE,
        _ => MQTT_PUBLISH_MODE_SCAN_INTERVAL,
    }
}
```

**⚠ 重複定義の問題（R-DEDUP 系）**: `normalize_publish_mode` は `crud.rs` と `mqtt.rs` の **2 箇所に同一実装が存在**する。削除と同時に `commands/publisher/crud.rs` を正本とし、`mqtt.rs` の重複を `pub(crate)` 参照に統合すること（または `publishers/mqtt.rs` が `crud.rs` に依存しない設計を維持するなら、両方から legacy ケースのみ削除でも可）。

**前提条件**:

- `ops/config/publishers.toml` に `"change_only"` / `"changed_only"` 値が存在しないこと

**確認コマンド**:

```powershell
Select-String -Path ops/config/publishers.toml -Pattern 'change_only|changed_only'
```

---

### C-08: MQTT 設定の旧キー `topic_prefix` フォールバック削除

**対象ファイル**:

- `core/src-tauri/src/publishers/mqtt.rs`（`get_topic_setting`、line ~76）
- `core/src-tauri/src/commands/publisher/crud.rs`（`list_publishers` の topic 読み取り、line ~61）

**削除対象** — `topic_prefix` への旧キー名フォールバック:

```rust
// 現状（削除前）
fn get_topic_setting(&self) -> String {
    self.config
        .settings
        .get("topic")
        .or_else(|| self.config.settings.get("topic_prefix"))  // ← 旧キー名互換
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// 削除後
fn get_topic_setting(&self) -> String {
    self.config
        .settings
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}
```

同様に `crud.rs` の `list_publishers` にも:

```rust
// 削除前（crud.rs の topic 読み取り）
.get("topic")
.or_else(|| cfg.settings.get("topic_prefix"))  // ← 削除対象
```

**前提条件**:

- `ops/config/publishers.toml` 内の全パブリッシャーが `topic` キーを使用していること（`topic_prefix` キーが存在しないこと）

**確認コマンド**:

```powershell
Select-String -Path ops/config/publishers.toml -Pattern 'topic_prefix'
```

---

## 3. 削除優先度と実施順序

| ID | 優先度 | 削除難易度 | 破壊的変更 | 推奨バージョン | Depends |
| --- | --- | --- | --- | --- | --- |
| **C-04** | **高** | 低 | **あり**（設定キー互換） | v0.5.0 | マイグレーション確認のみ |
| **C-05** | **高** | 低 | なし | v0.5.0 | manifest メタデータ優先へ移行済み |
| **C-01** | 中 | 中 | なし | v0.5.0 | 全運用環境移行確認後 |
| **C-02** | 中 | 低 | なし | v0.5.0 | C-01 完了後 |
| **C-03** | 中 | 低 | なし | v0.5.0 | 全配布物に manifest 同梱後 |
| **C-06** | 低 | 中 | なし（UI のみ） | v0.6.0 以降 | Postgres 外部プロセス UI 化後 |
| **C-07** | 高 | 低 | なし | v0.5.0 | **実装済み**（旧エイリアス削除） |
| **C-08** | 高 | 低 | なし | v0.5.0 | **実装済み**（topic_prefix 互換削除） |

---

## 4. v0.5.0 削除計画（詳細）

v0.5.0 での削除対象は **C-01〜C-05** とする。MQTT 関連の **C-07 / C-08 は先行して実装済み**。JoyWatcher の **C-04 も本対応で実装済み**。実施順序:

### Step 1: 設定ファイル確認（C-04 の前提）

```powershell
# drivers.toml に camelCase キーが残っていないことを確認
Select-String -Path ops/config/drivers.toml -Pattern '"userId"|"uid"'
```

問題があれば手動マイグレーション後に Step 2 へ進む。

### Step 2: C-04 — JoyWatcher camelCase キー削除

- `joywatcher_runtime.rs` の `CONNECTION_USER_ID_KEYS` から `"userId"` / `"uid"` を削除
- テスト `accepts_legacy_camel_case_user_id_key` を削除
- `cargo test -p driver-joywatcher` 通過確認

### Step 3: manifest 同梱確認（C-03 の前提）

```powershell
# 全ドライバの manifest 存在確認
Get-ChildItem ops/driver-ui -Recurse -Filter driver-manifest.json
```

### Step 4: C-01 — 旧 UIパス構成探索削除（完了）

- `paths.rs` の `find_default_driver_ui_path` から `app_root_style` ブランチ削除
- `manifest_candidates_for_root` の `root.join("driver-ui")` 候補削除
- 開発環境・ステージング環境でパス解決が正常であることを確認

### Step 5: C-02 — 祖先ディレクトリ探索削除（完了）

- `app_root_candidates` から優先順④ブロック削除
- `cargo clippy` でデッドコード警告がないことを確認

### Step 6: C-03 — manifest fallback削除

- `path_resolver.rs` の legacy search fallback ブロック削除
- manifest が無効なドライバで適切なエラーメッセージが出ることを確認

### Step 7: C-05 — フロントエンド ハードコード削除（完了）

- `driverUiFlow.ts` の `driverTypeLabel` / `driverTypeDescription` switch ケース削除
- `buildDriverTypeOptions` のフォールバック動作を確認（manifest なし時に `driverType` 文字列表示）
- `npm run check` / `npm run build` 通過確認

### Step 8: 品質ゲート

```powershell
cd core/src-tauri
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test

cd ../..
npm run check
npm run lint
npm run build
```

---

## 5. v0.6.0 以降の計画（C-06）

**C-06 は外部プロセス UI 化の収束タスク**として扱い、以下の条件が揃った時点で別タスクとして着手する:

- Postgres ドライバが `registration-ui.exe`（外部プロセス）として動作するバージョンが確認済み
- または manifest `ui_mode` 仕様が確定し、`DriverUiHost` の動的切替実装が完了

実装メモ（2026-05-20）:

- `DriverUiHost.svelte` は埋め込みパネルを持たない案内画面へ縮退済み
- `drivers/postgres/ui/` に外部プロセス登録UI 実装が存在し、`ops/driver-ui/postgres/registration-ui.exe` へ配置される
- したがって、Postgres 登録UI は外部プロセス前提へ移行済み

作業内容（将来タスク):

1. `DriverUiHost.svelte` を案内のみへ縮退し、埋め込みパネル依存をなくす
2. 外部プロセス UI の配置・起動経路（`ops/driver-ui/postgres/registration-ui.exe`）を維持する
3. `DriverUiHost` は将来的に廃止候補とし、必要なら完全削除する

---

## 6. マイグレーションノート（v0.4.x → v0.5.0）

v0.5.0 では以下の破壊的変更が含まれる（C-04）:

### JoyWatcher 設定キーの変更（C-04）

**変更内容**: `drivers.toml` の JoyWatcher 接続設定で `userId` / `uid` キーは非対応になる。

**対応方法**: 既存設定を `user_id` (snake_case) に変更する。

```toml
# 変更前（v0.4.x 以前）
[drivers.your_driver_id]
driver_type = "joywatcher"
userId = "your_user"   # ← NG（v0.5.0 以降）

# 変更後（v0.5.0）
[drivers.your_driver_id]
driver_type = "joywatcher"
user_id = "your_user"  # ← OK
```

---

## 7. 進行ログ

| ID | 着手者 | 状態 | バージョン | コミット | 備考 |
| --- | --- | --- | --- | --- | --- |
| C-01 | - | 完了 | v0.5.0 | - | 旧構成探索を削除済み |
| C-02 | - | 完了 | v0.5.0 | - | C-01 後 |
| C-03 | - | 完了 | v0.5.0 | - | manifest 無効時の fallback を削除済み |
| C-04 | - | 完了 | v0.5.0 | - | **破壊的変更・マイグレーション必須** |
| C-05 | - | 完了 | v0.5.0 | - | manifest 優先ラベル/説明へ移行済み |
| C-06 | - | 完了 | v0.6.0+ | - | 外部プロセス UI 前提へ移行済み |
| C-07 | - | 完了 | v0.5.0 | - | 旧エイリアス削除済み |
| C-08 | - | 完了 | v0.5.0 | - | topic_prefix 互換削除済み |

---

## 8. 関連ドキュメント

| ドキュメント | 参照理由 |
| --- | --- |
| [`docs/driver-manifest-discovery-design.md`](driver-manifest-discovery-design.md) | Phase 3 移行（本計画の前提） |
| [`docs/refactor-plan.md`](refactor-plan.md) | 完了済みリファクタ（本計画の前提） |
| [`docs/config-spec.md`](config-spec.md) | `drivers.toml` キー仕様（C-04 マイグレーション） |
| [`docs/decisions.md`](decisions.md) | 設計判断（ADR） |
