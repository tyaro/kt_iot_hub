# ドキュメント配置再編案（ハイブリッド）

## 実施状況（2026-05-19）

### 完了済み

- `docs/INDEX.md` を追加（入口一本化）
- 領域別 README を追加
  - `core/docs/README.md`
  - `drivers/docs/README.md`
  - `ops/docs/README.md`
  - `drivers/joywatcher/docs/README.md`
- driver 系文書を `drivers/docs/` へ移動
  - `driver-development.md`
  - `driver-implementation-flow.md`
  - `templates/*`
- core / ops / JoyWatcher 固有文書を領域配下へ移動
  - `core/docs/mqtt-monitor.md`
  - `ops/docs/deployment.md`
  - `drivers/joywatcher/docs/{investigation.md, session-handoff.md, bridge-x86-design.md}`
- 主要導線リンクを更新
  - `README.md`
  - `AGENTS.md`
  - `.github/copilot-instructions.md`
  - 関連 docs 内部リンク

### セーブポイント（コミット）

- `b23eaca`: docs(layout): driver系ドキュメントをdrivers/docsへ移設し索引を追加
- `da8e291`: docs(layout): core/ops/joywatcher 文書を領域配下へ移設

### 残タスク（軽微）

- `docs/workspace-structure-cleanup-plan.md` 内の旧パス表記は、履歴記述として残置中（必要なら注釈付きで更新）
- Markdown Lint 既存指摘（`docs/driver-property-extensibility-design.md` の `MD036`）は本作業スコープ外

## 目的

- ルート `docs/` の可読性を維持しつつ、実装密着ドキュメントをコード近傍へ移す。
- 「どこが正本か」を明示して重複・矛盾を防ぐ。
- `core/`, `drivers/`, `ops/` の新構成に整合させる。

## 基本方針

### 1) ルート `docs/` に残す（横断・正本）

- プロジェクト全体の設計・意思決定・ロードマップ
- 複数領域にまたがる仕様（driver共通契約、manifest設計など）
- エージェント運用上の正本参照先

### 2) 各領域へ移す（実装密着）

- 実装手順・運用手順・引き継ぎメモ・調査ノート
- 特定ドライバ専用の設計/調査文書

### 3) リンク主導

- 物理移動後も `docs/INDEX.md`（新規）を入口にし、必ずリンクで到達可能にする。
- ルート側から「正本」と「補助資料」の関係を明示する。

---

## 移動先マッピング（提案）

| 現在 | 提案先 | 扱い |
| --- | --- | --- |
| `docs/design.md` | `docs/design.md` | 維持（正本） |
| `docs/architecture.md` | `docs/architecture.md` | 維持（正本） |
| `docs/decisions.md` | `docs/decisions.md` | 維持（正本） |
| `docs/roadmap.md` | `docs/roadmap.md` | 維持（正本） |
| `docs/overview.md` | `docs/overview.md` | 維持（入口） |
| `docs/refactor-plan.md` | `docs/refactor-plan.md` | 維持（正本） |
| `docs/workspace-structure-cleanup-plan.md` | `docs/workspace-structure-cleanup-plan.md` | 維持（横断計画） |
| `docs/config-spec.md` | `docs/config-spec.md` | 維持（契約） |
| `docs/driver-manifest-discovery-design.md` | `docs/driver-manifest-discovery-design.md` | 維持（横断設計） |
| `docs/driver-property-extensibility-design.md` | `docs/driver-property-extensibility-design.md` | 維持（横断設計） |
| `docs/ui-registration.md` | `docs/ui-registration.md` | 維持（横断UI契約） |
| `docs/driver-development.md` | `drivers/docs/driver-development.md` | 移動（実装手順） |
| `docs/driver-implementation-flow.md` | `drivers/docs/driver-implementation-flow.md` | 移動（実装手順） |
| `docs/mqtt-monitor.md` | `core/docs/mqtt-monitor.md` | 移動（本体機能） |
| `docs/deployment.md` | `ops/docs/deployment.md` | 移動（運用） |
| `docs/joywatcher-investigation.md` | `drivers/joywatcher/docs/investigation.md` | 移動（ドライバ固有） |
| `docs/joywatcher-session-handoff.md` | `drivers/joywatcher/docs/session-handoff.md` | 移動（ドライバ固有） |
| `docs/joywatcher-x86-bridge-design.md` | `drivers/joywatcher/docs/bridge-x86-design.md` | 移動（ドライバ固有） |
| `docs/templates/*` | `drivers/docs/templates/*` | 移動（実装テンプレ） |

---

## 最小追加ファイル（提案）

### `docs/INDEX.md`（新規）

- 目的: 入口一本化
- 内容:
  - ルート正本（design/decisions/refactor-plan など）
  - `core/docs` / `drivers/docs` / `ops/docs` へのリンク
  - 「正本/補助資料」の凡例

### 各領域 `README.md`（新規）

- `core/docs/README.md`
- `drivers/docs/README.md`
- `ops/docs/README.md`

役割: その領域内ドキュメントの目次と更新ルールを明示。

---

## 実施手順（安全順）

1. `docs/INDEX.md` を先に作成（リンク先は仮でも可）
2. 新ディレクトリを作成
   - `core/docs/`
   - `drivers/docs/`
   - `drivers/joywatcher/docs/`
   - `ops/docs/`
3. ドキュメントを **1カテゴリずつ** 移動（1コミット=1関心事）
4. 参照リンクを更新（README, AGENTS, `.github/copilot-instructions.md`）
5. 最後に `grep` で旧パス残存を検査

---

## ガード条件

- 既存の「正本」ドキュメントは移動対象にしない（まず維持）
- 1回で全部動かさず、カテゴリ単位で段階移行する
- 移動コミットと内容変更コミットを分離する
- `AGENTS.md` / `.github/copilot-instructions.md` の参照切れを必ず先に直す

---

## 判断メモ

このリポジトリでは、以下のバランスが最適:

- **検索性（入口一本化）**: ルート `docs/`
- **保守性（実装近接）**: `core/docs`, `drivers/docs`, `ops/docs`

よって、完全分散ではなくハイブリッドを推奨する。
