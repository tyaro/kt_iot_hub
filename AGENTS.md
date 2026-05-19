# AGENTS.md

このファイルは GitHub Copilot 以外のコーディングエージェント（Claude / Cursor / Codex / Aider など）が
本リポジトリで作業を開始する際に**最初に読むべき**入口ドキュメントです。

> **基本姿勢**: 推測でコードを増やさない。既存パターンを踏襲する。指摘事項は黙って修正せず、まず提示する。

---

## 1. 必読ドキュメント（優先順）

着手前に**必ず**以下を参照してください。`.github/copilot-instructions.md` がプロジェクトルールの**唯一の正本**です。

| # | ドキュメント | 用途 |
| --- | --- | --- |
| 1 | [.github/copilot-instructions.md](.github/copilot-instructions.md) | **プロジェクトルール正本**。ファイルサイズ規約・命名・エラー処理・セキュリティ・スコープ規律。**最低限ここだけは読む** |
| 2 | [docs/refactor-plan.md](docs/refactor-plan.md) | リファクタタスク一覧（`R-BE-*` / `R-FE-*` / `R-RS-*` / `R-DEDUP-*`）と進行ログ。**構造変更を伴う作業の前に必ず確認** |
| 3 | [docs/design.md](docs/design.md) | 全体設計書 |
| 4 | [docs/architecture.md](docs/architecture.md) | アーキ概要 |
| 5 | [docs/decisions.md](docs/decisions.md) | 設計判断（ADR 相当） |
| 6 | [docs/driver-manifest-discovery-design.md](docs/driver-manifest-discovery-design.md) | ドライバ候補を `driver-manifest.json` から自動発見する設計（埋め込み DriverType 縮退方針を含む） |

ドライバ実装に着手する場合は加えて以下:

- [docs/driver-development.md](docs/driver-development.md)
- [docs/driver-implementation-flow.md](docs/driver-implementation-flow.md)
- [docs/templates/driver-session-handoff-template.md](docs/templates/driver-session-handoff-template.md)

---

## 2. プロジェクト概要（30 秒サマリ）

- **目的**: オフライン環境で動作する IoT ハブ。各種ドライバから値を収集し MQTT で配信。
- **構成**: Tauri v2 デスクトップアプリ。バックエンド Rust（Tokio）、フロントエンド Svelte 5 Runes + SvelteKit SPA（adapter-static, ssr=false）。
- **Cargo workspace 構成**: `core/src-tauri/`（本体）/ `drivers/joywatcher/{bridge-x86,driver,ui}` / `drivers/postgres/{driver,ui}` / `packages/{protocol-rs,driver-ui-host}`。
- **通信**: Tag Bus（`tokio::sync::broadcast`）+ gRPC（`DriverRuntimeService`）+ `DriverProcessManager`。
- **MQTT ブローカーは内蔵しない**（外部 Mosquitto/EMQX 前提）。Publisher のみ実装。
- **OPC DA は対象外**（COM/DCOM 非対応のため）。

---

## 3. 厳守する不変条件（変更厳禁）

以下は機能改修・リファクタの**いずれにおいても**バイト互換を維持する:

- Tauri コマンド名 / 引数 DTO のフィールド名・JSON 形（`#[serde(rename_all = "camelCase")]` を含む既存表現）
- gRPC proto（`core/src-tauri/proto/*.proto`）
- ドライバ UI 連携 JSON（`packages/protocol-rs/` 正本）
- `ops/config/*.toml` のキー
- ログメッセージの grep キー（変更時は `docs/refactor-plan.md` に追記）

---

## 4. 必ず通すチェック

完了時、PR / コミット前に必ず以下を通す:

```powershell
# Rust
cd src-tauri
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd ..

# または
cd core/src-tauri
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd ../..

# Frontend
npm run check
```

---

## 5. やってはいけないこと（抜粋）

- 依頼スコープ外のリファクタ・整形・依存追加（**1 コミット = 1 関心事**）
- `unwrap()` / `expect()` の濫用（テスト・`main()` 初期化のみ許可）
- `static mut` / `std::sync::Mutex` を `.await` 越しで保持
- 機密値のハードコード（パスワード・API キー・接続文字列）
- 外部 CDN への動的読み込み（オフライン環境のため）
- ライセンス不明 / コピーレフトな依存の無断追加

詳細・全項目は [.github/copilot-instructions.md](.github/copilot-instructions.md) §1〜§6 を参照。

---

## 6. リファクタタスクを引き継ぐ場合

1. [docs/refactor-plan.md](docs/refactor-plan.md) §4 進行ログで未着手タスクを確認。
2. §3 該当タスクの **Depends / 対象 / 手順 / 受け入れ条件** を読み切ってから着手。
3. コミットメッセージ先頭にタスク ID を付ける（例: `refactor(R-DEDUP-01): write_tags_toml_atomic 重複排除`）。
4. 完了時に §4 進行ログ表を更新し、`cargo fmt` / `cargo clippy -D warnings` / `cargo test` / `npm run check` を通す。
5. 派生問題は §5 発見メモへ追記。

### 既知の地雷（着手前必読）

- **`write_tags_toml_atomic` が 2 箇所で重複定義**（`core/src-tauri/src/commands/driver/toml_io.rs:31` と `core/src-tauri/src/commands/tag.rs:215`）。tag CRUD を触る場合は **R-DEDUP-01** を先に終わらせるか、両方を同期させて編集する。
- 同様の重複箇所は `docs/refactor-plan.md` §1.3 にまとめてある。
- ドライバ候補は現状フロント埋め込み（`knownDriverTypes`）と実行ファイル探索の併用。**manifest 駆動へ移行中**のため、候補生成ロジックに変更を入れる際は `docs/driver-manifest-discovery-design.md` の Phase 計画（互換フォールバック維持）を必ず確認する。

---

## 7. 不明点があるとき

- 仕様が曖昧なら **実装前に質問**する（黙って解釈しない）。
- 構想・設計に矛盾を見つけたら **指摘**する（黙って修正しない）。
- 「実現困難」「セキュリティ懸念」「過剰実装」を察知したら必ず提示する。
