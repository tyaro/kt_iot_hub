# GitHub Copilot / AI エージェント向けプロジェクトルール

このファイルは `kt_iot_hub`（Tauri v2 + Rust + Svelte）プロジェクトにおいて、
AI コーディングエージェント（Copilot Chat / Claude / Cursor 等）が遵守すべき規約を定義します。

> **基本姿勢**: 推測でコードを増やさない。既存パターンを踏襲する。指摘事項は黙って修正せず、まず提示する。

---

## 0. プロジェクト概要（コンテキスト）

- **目的**: オフライン環境で動作する IoT ハブ。各種ドライバから値を収集し MQTT で配信。
- **構成**: Tauri v2 デスクトップアプリ。バックエンド Rust、フロントエンド Svelte 5（SvelteKit SPA）。
- **設計書**: [docs/design.md](../docs/design.md) を必ず参照すること。
- **MQTT ブローカーは内蔵しない**（外部 Mosquitto/EMQX 前提）。Publisher のみ実装。
- **OPC DA は対象外**（COM/DCOM 非対応のため）。

---

## 1. 共通ルール

### 1.1 ファイルサイズ
- **1ファイル原則 300 行以内**。超える場合は責務分割を検討する。
- 機械的な分割ではなく、**責務（単一責任）単位**で分ける。
- 超える場合はファイル先頭コメントに理由を記載する。

### 1.2 疎結合
- モジュール間通信は **Tag Bus（`tokio::sync::broadcast`）** または **trait** を介して行う。
- ドライバ実装が他ドライバや UI の型に依存してはならない。
- Svelte コンポーネントは props と store でのみ通信し、`window` グローバル等を共有しない。

### 1.3 言語
- **コメント・ドキュメント・コミットメッセージは日本語**を基本とする。
- 識別子（関数名・変数名）は **英語 snake_case / camelCase**（言語慣習に従う）。
- ログメッセージは英語（grep しやすさのため）。

### 1.4 エラー処理
- `unwrap()` / `expect()` は **テストコードと `main()` の初期化のみ**許可。
- それ以外は `Result<_, anyhow::Error>` または `thiserror` で定義したエラー型を返す。
- フロントは `try/catch` で Tauri invoke を必ず包み、UI に通知する。

### 1.5 セキュリティ
- 秘匿情報（パスワード、APIキー、接続文字列）を**ソース・ログ・コミットに含めない**。
- 設定ファイルの秘匿項目は OS キーリング（`keyring` crate）に分離する。
- MQTT 接続はデフォルトで TLS を推奨する設定例を出す。
- `tauri.conf.json` の `app.security.csp` を有効に保つ。

### 1.6 依存追加
- 新規 crate / npm パッケージ追加時は**必ず理由を提示**し、ユーザの承認を得る。
- 同等機能の既存依存があれば再利用する。
- ライセンスは MIT / Apache-2.0 / BSD 系を優先（GPL 系は要確認）。

### 1.7 変更スコープ
- 依頼されていないリファクタリング・整形を**勝手にやらない**。
- 既存コードのスタイル（命名・分割粒度）を尊重する。
- 大規模変更を提案する前に、まず計画を提示する。

---

## 2. Rust（src-tauri）ルール

### 2.1 ツールチェイン・スタイル
- Edition: **2021** 以降。
- `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings` を通すこと。
- `#![deny(unsafe_code)]` をクレート root に設定する（必要箇所のみ `#[allow(unsafe_code)]` で局所許可）。

### 2.2 非同期
- ランタイムは **Tokio** に統一。`async-std` 等を混ぜない。
- ブロッキング処理（DB ドライバ等で同期 API しかない場合）は `tokio::task::spawn_blocking` で分離。
- 長時間ループには必ず `tokio::select!` でキャンセル（`CancellationToken`）を入れる。

### 2.3 モジュール構成
- `commands/` … Tauri コマンドは**薄く**。引数バリデーション + `core` 呼び出しのみ。ビジネスロジックを書かない。
- `core/` … ドメイン型と Tag Bus。外部 IO に依存しない。
- `drivers/<name>/` … 各ドライバは独立モジュール（将来 workspace のサブクレート化を見据えた構造）。
- `publishers/<name>/` … 同上。
- 新ドライバ追加時は **`Driver` trait のみ**に依存する。

### 2.4 型と命名
- ID 系は newtype（`pub struct TagId(String);`）で型安全を確保。生の `String` を引き回さない。
- `serde` 派生は明示。`#[serde(rename_all = "camelCase")]` をフロント連携 DTO に付ける。
- DTO（フロント連携用構造体）は `commands/dto.rs` 等に集約。ドメイン型と分離。

### 2.5 ロギング
- `tracing` を使用。`println!` / `eprintln!` を本体コードで使わない。
- スパンは関数単位で `#[tracing::instrument(skip(self))]`。
- ログレベルは: `error` (要対応), `warn` (異常だが継続), `info` (節目), `debug` (詳細), `trace` (フロー追跡)。

### 2.6 テスト
- ユニットテストは同ファイル内 `#[cfg(test)] mod tests`。
- ドライバには `MockDriver` を `drivers/mock.rs` に用意し結合テストで使用。
- 非同期テストは `#[tokio::test]`。

### 2.7 禁則
- `static mut` 禁止。可変グローバルは `OnceCell<Mutex<_>>` を使う。
- `std::sync::Mutex` を `.await` 越しで保持しない（→ `tokio::sync::Mutex`）。
- panic を期待する制御フロー禁止。

---

## 3. Svelte / Frontend ルール

### 3.1 バージョン・スタイル
- **Svelte 5** の Runes（`$state`, `$derived`, `$effect`, `$props`）を優先。
- TypeScript 必須（`<script lang="ts">`）。
- Prettier + ESLint（svelte plugin）に従う。
- SvelteKit を **SPA モード（adapter-static + `ssr = false`）** で使用。

### 3.2 コンポーネント
- 1 コンポーネント 1 ファイル、**200 行以内目安**。
- 配置:
  - 汎用 UI: `src/lib/components/ui/`
  - 機能別: `src/lib/components/<feature>/`
  - レイアウト: `src/lib/components/layout/`
- props は型を明示: `let { tag }: { tag: Tag } = $props();`
- 双方向バインドは最小限。基本は単方向データフロー。

### 3.3 状態管理
- ローカル状態: `$state`
- 派生: `$derived`
- 横断状態: `src/lib/stores/` に Svelte store または rune ベースのモジュールを置く。
- グローバル可変状態は最小化。Tauri からの push データは store に集約。

### 3.4 Tauri IPC
- `invoke` 呼び出しは **直接コンポーネントから呼ばない**。
- `src/lib/ipc/` 配下にラッパ関数を置き、型付き API として提供する。
- イベント購読（`listen`）は `onMount` で登録し、`onDestroy` で必ず解除する。

### 3.5 スタイル
- スタイルはコンポーネント `<style>` スコープ内に閉じる。
- グローバルは `src/app.css` のみ。CSS 変数でテーマ化。
- アクセシビリティ警告（a11y）を無視しない。

### 3.6 禁則
- `any` 型禁止（やむを得ない場合は `unknown` + 型ガード）。
- `eval` / `Function` コンストラクタ禁止。
- 外部 CDN への動的読み込み禁止（オフライン環境のため）。

---

## 4. Tauri v2 固有ルール

### 4.1 設定
- `tauri.conf.json` の `app.security.csp` は常時有効。`unsafe-inline` を追加しない。
- 権限は **capabilities** で最小権限に絞る。`allowlist` 全許可禁止。
- ファイル/シェル系プラグインは**本当に必要なもののみ**追加。

### 4.2 コマンド設計
```rust
#[tauri::command]
async fn create_tag(
    state: tauri::State<'_, AppState>,
    req: CreateTagRequest,
) -> Result<TagDto, AppError> {
    state.tag_service.create(req).await.map(Into::into)
}
```
- 引数は構造体（DTO）でまとめる。位置引数を増やさない。
- 戻り値は `Result<_, AppError>` で統一。`AppError` は `serde::Serialize` 実装。
- 状態は `tauri::State` 経由で注入。グローバル `static` を使わない。

### 4.3 イベント
- フロントへの push は `app.emit("tag.value.updated", payload)` のように **ドット区切りのイベント名規約**。
- ペイロードは型を Rust 側と共有（`ts-rs` 等で自動生成検討）。

---

## 5. ドライバ追加時のチェックリスト

新規ドライバ実装時、AI エージェントは以下を必ず確認・提示する:

- [ ] `Driver` trait を実装したか
- [ ] `registration_schema()` で JSON Schema を返しているか（UI 動的生成用）
- [ ] エラーは `DriverError` で表現し、panic していないか
- [ ] 取得値は `DriverCtx::publish` で Tag Bus に流しているか
- [ ] `start` / `stop` がべき等か（複数回呼ばれても安全か）
- [ ] 接続断時のリトライ戦略があるか（指数バックオフ推奨）
- [ ] ユニットテスト・モックを追加したか
- [ ] `DriverManager` への登録を追加したか
- [ ] README または docs にドライバ仕様を追記したか

---

## 6. AI エージェントの振る舞いルール

### 6.1 着手前
1. 関連ファイルを読む（推測しない）。
2. 設計書（[docs/design.md](../docs/design.md)）の関連章を確認する。
3. 影響範囲を提示してから着手する（大規模変更の場合）。

### 6.2 実装中
- 既存パターンを真似る（独自スタイルを持ち込まない）。
- 1コミット = 1関心事を意識した粒度で変更する。
- 新規ファイル作成は最小限。既存ファイルへの追記で済むなら追記する。

### 6.3 完了時
- `cargo fmt` / `cargo clippy` / `npm run check` 相当を通したか確認する。
- テストを書いたか、走らせたかを報告する。
- 変更したファイル一覧と要点を簡潔に提示する。

### 6.4 不明点・違和感
- 仕様が曖昧な場合は**実装前に質問**する。
- 構想・設計に矛盾を見つけた場合は**黙って解釈せず指摘**する。
- 「実現困難」「セキュリティ懸念」「過剰実装」を察知したら必ず提示する。

### 6.5 やってはいけないこと
- 依頼スコープ外のリファクタ／整形／依存追加。
- テストの削除・スキップ（理由なしに `#[ignore]` を付ける等）。
- `unwrap()` の濫用、`unsafe` の無断使用。
- 機密値のハードコード。
- ライセンス不明 / コピーレフトな依存の無断追加。

---

## 7. 参考コマンド

```powershell
# Rust
cd src-tauri
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test

# Frontend
npm run check
npm run lint
npm run build

# Tauri 開発
npm run tauri dev
npm run tauri build
```

---

## 8. 進行中のリファクタリング引き継ぎ

> 本プロジェクトでは **保守性向上のためのリファクタを継続的に進行**している。
> 機能追加・修正タスクであっても、関連箇所がリファクタ対象に含まれる場合は本セクションを参照すること。

### 8.1 唯一の正本

- **[docs/refactor-plan.md](../docs/refactor-plan.md)** がリファクタ計画の正本。タスク ID（`R-BE-*` / `R-FE-*` / `R-RS-*` / `R-DEDUP-*` / `R-DOC-*`）単位で独立着手できる構成。
- タスク着手前に必ず以下を確認:
  1. `docs/refactor-plan.md` §0.2 **不変条件**（Tauri コマンド名・serde 表現・proto・JSON 形・TOML キー・ログ grep キーは不変）
  2. 同 §3 該当タスクの **Depends / 対象 / 手順 / 受け入れ条件**
  3. 同 §4 進行ログ表（既に他エージェントが着手済みでないか）

### 8.2 リファクタタスク着手の手順

1. 着手前: 進行ログ表の該当行を「着手中」に更新し、コミットを 1 件作る（`docs(refactor): R-XX-NN 着手` 等）。
2. 実装中: コミットメッセージの先頭にタスク ID を付ける（例: `refactor(R-BE-01): dto.rs を機能別に分割`）。
3. 完了時:
   - `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test` / `npm run check` を必ず通す。
   - 進行ログ表を「完了」に更新し、コミット ID を埋める。
   - 派生タスク・前提崩れを発見した場合は §5 発見メモへ追記。

### 8.3 既知の地雷（着手前必読）

- **`write_tags_toml_atomic` が 2 箇所で重複定義**（`src-tauri/src/commands/driver/toml_io.rs:31` と `src-tauri/src/commands/tag.rs:215`）。tag CRUD を触る場合は **R-DEDUP-01** を先に終わらせるか、両方を同期させて編集する。
- `normalize_optional_string` が 3 ファイルに同名で並存（R-DEDUP-08）。新規呼び出し追加時はどれを import すべきか **R-DEDUP-08** 完了後に統一される予定。
- `apps/joywatcher/{driver,ui}` 間でブリッジ探索ロジック / `BRIDGE_EXE_NAME` 等が重複（R-DEDUP-09）。片方だけ修正しない。
- driver-UI 静的資産（`apps/*/ui/assets/app.js`）の `tauriInvoke` ラッパ等が重複（R-DEDUP-07）。

### 8.4 リファクタ作業のスコープ規律（再掲・厳守）

- **シリアライズ表現を変えない**。Tauri コマンドの引数・戻り値 JSON、`config/*.toml` キー、ドライバ UI 連携 JSON、gRPC proto はバイト互換を維持。
- **依頼スコープ外のリファクタを混ぜない**。R-XX-NN 着手中であっても、計画外の整形・命名変更・依存追加は別タスクに切り出す。
- **共通化は「明らかな同一実装」のみ対象**。似て非なるロジック（例: ドライバ毎の探索順）は安易に統合しない。
- **1 タスク = 1 コミット粒度**。移動だけの変更とロジック変更はコミットを分ける。

### 8.5 関連ドキュメント早見表

| 知りたいこと | 参照先 |
| --- | --- |
| アーキ全体・設計判断 | [docs/design.md](../docs/design.md), [docs/architecture.md](../docs/architecture.md), [docs/decisions.md](../docs/decisions.md) |
| ドライバ実装フロー | [docs/driver-development.md](../docs/driver-development.md), [docs/driver-implementation-flow.md](../docs/driver-implementation-flow.md) |
| 引き継ぎテンプレ | [docs/templates/driver-session-handoff-template.md](../docs/templates/driver-session-handoff-template.md) |
| 設定ファイル仕様 | [docs/config-spec.md](../docs/config-spec.md) |
| UI 登録仕様 | [docs/ui-registration.md](../docs/ui-registration.md) |
| MQTT モニタ仕様 | [docs/mqtt-monitor.md](../docs/mqtt-monitor.md) |
| **リファクタ計画（本セクションの正本）** | [docs/refactor-plan.md](../docs/refactor-plan.md) |

---

## 9. 機能追加・修正時の実行方針（AI エージェント運用ガイド）

> 本セクションは、リファクタ以外（機能追加・不具合修正・仕様調整）を実施する際の標準フローを定義する。

### 9.1 基本原則

1. **先に理解、後で実装**
  - まず再現条件・期待動作・既存挙動を文章化する。
  - 推測で実装を始めない。必ず関連コードと設計資料を読んでから着手する。
2. **互換性優先**
  - 既存 API / JSON / TOML / gRPC の表現を壊さない。
  - 破壊的変更が必要な場合は、必ず事前に提案し承認を得る。
3. **最小変更**
  - 要件達成に必要な最小差分を選ぶ。
  - 依頼スコープ外の整理・命名変更・整形混在は禁止。
4. **検証込みで完了**
  - 実装だけで終わらせず、必須コマンド実行・結果記録まで行う。

### 9.2 着手前チェック（必須）

- [ ] 依頼内容を「入力・処理・出力・失敗時挙動」で分解したか
- [ ] 影響範囲（Rust / Frontend / config / docs）を列挙したか
- [ ] 既存不変条件（§0.2, §8.1）に抵触しないことを確認したか
- [ ] 関連ドキュメント（最低: `docs/design.md`, 必要に応じ `docs/refactor-plan.md`）を確認したか
- [ ] 既存実装の再利用可否（同等ロジックの有無）を確認したか

### 9.3 実装手順（標準フロー）

1. **調査**: 参照箇所・呼び出し元・呼び出し先を確認する。
2. **設計メモ化**: 変更方針を 3〜7 行程度で明文化する（何を変え、何を変えないか）。
3. **小さく実装**: 1関心事ずつ変更し、途中でビルド不能状態を長く残さない。
4. **局所テスト**: 変更箇所に近いテスト・コマンドを先に実行する。
5. **全体ゲート**: §4 の必須チェックを通す。
6. **ドキュメント更新**: 仕様・運用影響がある場合は docs を更新する。

### 9.4 変更種別ごとの追加ルール

#### A) 機能追加
- 既存境界（`commands` は薄く、ドメインロジックは `core` / 専用モジュール）を維持する。
- UI 追加時は `ipc` ラッパを経由し、コンポーネントから直接 invoke しない。
- 新規設定値を追加する場合は、デフォルト値・未設定時挙動・後方互換を明記する。

#### B) 不具合修正
- まず再現手順を固定化し、「修正前に失敗する確認」を取る。
- 症状ではなく根本原因を修正する。
- 可能な限り回帰テスト（最低1件）を追加する。

#### C) 仕様変更
- 互換性影響を明記し、移行手順（必要なら段階移行）を提示する。
- 既存利用者への影響点（設定ファイル、UI 操作、ログキー）を列挙する。

### 9.5 品質ゲート（完了条件）

以下を満たして初めて「完了」とする。

- Rust: `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test`
- Frontend: `npm run check`（必要に応じ `npm run lint`, `npm run build`）
- 変更ファイル一覧と要点を報告
- 仕様影響がある場合、関連 docs 更新済み

### 9.6 コミット・報告ルール

- **1コミット = 1関心事**（移動とロジック変更は分離）。
- コミットメッセージは日本語で、何を・なぜ変更したかが読めること。
- 最終報告には最低限以下を含める:
  1. 変更ファイル
  2. 変更概要
  3. 検証結果（実行コマンドと pass/fail）
  4. 残課題（あれば）

### 9.7 禁止事項（再掲）

- 無承認の依存追加
- スコープ外変更の混入
- 既存プロトコル・シリアライズ表現の無断変更
- テストの無断削除・スキップ
- 機密情報のハードコード
