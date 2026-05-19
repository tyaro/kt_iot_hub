# ワークスペース構造整理計画

## 背景

現状の `driver-ui/` / `src-tauri/driver-ui/` は二重配置になっており、また探索ロジックと配置責務が不明確なため、ドライバ追加時や次フェーズ（manifest 駆動ディスカバリ）への移行時にノイズが増える。

本計画は、**既存動作を壊さず、段階的に整理**することを目標とする。

## ゴール

- 「開発時正本」「バンドルステージング」「実行時探索」の責務を明確化
- ドライバ追加時に「どこに何を置くか」が1ページでわかる状態
- 将来の manifest 駆動ディスカバリへの移行を素直にサポート

## 現状構造の問題点

| 項目 | 問題 |
| --- | --- |
| 二重配置 | `driver-ui/` と `src-tauri/driver-ui/` に同種成果物が並列 |
| 責務曖昧 | 開発用正本 vs bundle staging の区別が明確でない |
| 探索ロジック | `knownDriverTypes`（埋め込み）と実ファイル探索が併存 |
| スクリプト | build/install/staging が混在、再利用時の意図不明 |
| ドキュメント | 配置先・探索順が散在（README.md / deployment.md / 各スクリプト） |

## 最終像

### ディレクトリ責務

- **`driver-ui/<type>/`**（開発時正本）  
  - `registration-ui.exe`
  - `driver-<type>.exe`
  - （将来）`driver-manifest.json`
  - この場所が「運用開始時の起点」

- **`src-tauri/driver-ui/<type>/`**（bundle staging）  
  - バンドル直前に同期されるステージング先
  - installer bundle に同梱される
  - 正本ではない

- **`target/release/`**（ビルド生成物）  
  - 中間成果物
  - scripts で run-time に正本へコピー

### スクリプト責務

- `build-*`: ビルドのみ（`target/` へ出力）
- `install-*`: 正本 (`driver-ui/`) へのコピー
- `stage-*`: 正本から bundle staging (`src-tauri/driver-ui/`) へのコピー

---

## 実行計画（段階別）

### Phase 0: 現状固定と可視化（安全策）

**目的**: 運用事故防止。既存動作は変えず、ドキュメント整備のみ。

#### Task 0.1: 現行探索ロジック・配置先をドキュメント化

- **対象**: `docs/driver-development.md` / `docs/deployment.md`
- **作業**:
  1. 実行時探索の優先順位を明記  
     - 例: base_dir → DRIVER_BIN_DIR → app dir → PATH
  2. 現行の配置先2つ（`driver-ui/` / `src-tauri/driver-ui/`）を明記
  3. 各スクリプトの役割を記載
  4. ドライバ追加時「どこに何を置くか」の手順を明確化
- **受け入れ条件**: ドキュメント読了者が迷わずドライバを配置・追加できる

#### Task 0.2: README の配置説明を1箇所に集約

- **対象**: `README.md`
- **作業**:
  1. 散在している配置説明（複数箇所）を1セクション「Driver and Registration UI Deployment」に集約
  2. 二重配置の存在理由を簡潔に説明
- **受け入れ条件**: 新規ユーザが README だけで配置構成が理解できる

#### Task 0.3: スクリプト先頭コメント統一

- **対象**:
  - `scripts/build-release-driver-suite.ps1`
  - `scripts/install-driver-ui.ps1`
  - `scripts/install-driver-runtime.ps1`
- **作業**:
  1. 各スクリプトの冒頭コメントで責務を明記
     - どこから読み込むか（正本 or staging）
     - どこへ出力するか
  2. 例: "Installs artifacts from target/release/ to driver-ui/ (development source of truth)"
- **受け入れ条件**: スクリプトを開いた時点で、何をするもので、何を前提としているかが読める

---

### Phase 1: 正本ディレクトリの宣言と片方向化

**目的**: 「`driver-ui/` = 開発時正本」「`src-tauri/driver-ui/` = staging」を明文化し、スクリプト動作を統一。

#### Task 1.1: `driver-ui/` をリード・ミー配置として整備

- **対象**: `driver-ui/`（物理ディレクトリ）
- **作業**:
  1. `driver-ui/.deploymentinfo` 作成（例）

     ```text
     # Driver UI Deployment Structure
     This is the DEVELOPMENT SOURCE OF TRUTH for driver-ui resources.
     
     Layout:
     - driver-ui/postgres/
       - registration-ui.exe (運用開始時の起点)
       - driver-postgres.exe
     - driver-ui/joywatcher/
       - registration-ui.exe
       - driver-joywatcher.exe
       - joywatcher-bridge-x86.exe
     
     See docs/driver-development.md for add-new-driver procedure.
     ```

  2. `.gitignore` を見直し、exe ファイルは git 管理対象か or build-time 配置か明確化
- **受け入れ条件**: 誰が見ても「ここが正本」と理解できる

#### Task 1.2: スクリプト動作を「正本→staging 一方向」に統一

- **対象**: `scripts/build-release-driver-suite.ps1`
- **作業**:
  1. 既存動作は保持（互換性維持）
  2. コメント・ログを「正本から staging へコピー」と明確化
  3. 逆方向コピー（staging → 正本）がないことを確認
- **受け入れ条件**: 実行ログで「正本から staging へ」の一方向が視認できる

#### Task 1.3: `src-tauri/driver-ui/` に README 置く

- **対象**: `src-tauri/driver-ui/`（ディレクトリ）
- **作業**:
  1. `.staginginfo` ファイルを追加

     ```text
     # Bundle Staging Directory
     This is NOT the source of truth. 
     Synced from ../driver-ui/ before bundle.
     Do not edit files here directly.
     ```

- **受け入れ条件**: staging ディレクトリへの編集を防止（トラブル予防）

---

### Phase 2: スクリプト責務の分離と新規化

**目的**: 既存スクリプトは互換性維持、責務ごとに新規スクリプトを用意。

#### Task 2.1: `stage-*` スクリプト系を新規作成

- **対象**: `scripts/`
- **作業**:
  1. `scripts/stage-driver-artifacts.ps1` 作成  
     → 正本 (`driver-ui/`) から staging (`src-tauri/driver-ui/`) へコピーのみ
  2. `scripts/stage-driver-suite.ps1` 作成  
     → 全ドライバの staging を一括実行
- **受け入れ条件**: bundle 前ステップとして独立実行可能

#### Task 2.2: 既存 build-release スクリプトはラップ化検討

- **対象**: `scripts/build-release-driver-suite.ps1`
- **作業**:
  1. 当面は既存を維持（互換性）
  2. 内部的に段階的に新 stage-* へ委譲する準備
  3. 将来：deprecated path を明確化
- **受け入れ条件**: 既存利用者に影響なし

---

### Phase 3: 探索ロジックの単純化と明記

**目的**: 実行時探索の優先順位を固定。deprecated パターンは期限付きで縮退。

#### Task 3.1: 探索優先順位をハードコード

- **対象**: `src-tauri/src/commands/driver/ui_launcher/paths.rs`
- **作業**:
  1. 関数 `app_root_candidates()` のロジックを見直し、優先順を明確化
  2. コメント追加：「優先順① base_dir → ② DRIVER_BIN_DIR → ③ app/resources → ④ ancestors」
  3. 不要な candidate は削除 or deprecated 化
- **受け入れ条件**: コード + コメントで探索順が1読で理解できる

#### Task 3.2: 実行時エラーメッセージを改善

- **対象**: `src-tauri/src/commands/driver/ui_launcher/session.rs` など
- **作業**:
  1. 「見つからない理由」を具体的に（例: "searched in: base_dir/driver-ui/DriverType, DRIVER_BIN_DIR, ..."）
  2. ユーザーが対応しやすいメッセージ化
- **受け入れ条件**: エラーが出たときドキュメント＋エラー文から問題箇所が特定できる

---

### Phase 4: Manifest 駆動ディスカバリへの接続準備

**目的**: manifest 実装時にスムーズに移行できるよう、事前整備。

#### Task 4.1: `driver-manifest.json` 配置規約をドキュメント化

- **対象**: `docs/driver-manifest-discovery-design.md`
- **作業**:
  1. 現段階では「Phase 0 で整理した正本場所」の中に manifest も置く設計
  2. 例: `driver-ui/<type>/driver-manifest.json`
- **受け入れ条件**: ドライバ開発者が「manifest をどこに置くか」で迷わない

#### Task 4.2: `knownDriverTypes` 削除時期の決定

- **対象**: `docs/decisions.md` / `docs/roadmap.md`
- **作業**:
  1. manifest 全面移行時期を決定（例: v0.5.0 以降）
  2. 事前告知（v0.4.0 で deprecated 化など）
- **受け入れ条件**: 次フェーズ計画への道筋が明確

---

### Phase 5: ディレクトリ構造の簡潔化・グループ化

**目的**: ワークスペースルートの混在を解消。ドライバの個別開発を容易化。

**最終像（案A: 軽量グループ化）:**

```text
kt_iot_hub/
├── core/                   ← メインアプリケーション（Tauri）
│   ├── src/                （フロント：Svelte）
│   └── src-tauri/          （バック：Rust）
├── drivers/                ← ドライバアプリケーション群
│   ├── postgres/
│   │   ├── driver/
│   │   └── ui/
│   └── joywatcher/
│       ├── driver/
│       ├── ui/
│       └── bridge-x86/
├── packages/               ← 共有ライブラリ（変更なし）
│   ├── protocol-rs/
│   └── driver-ui-host/
├── ops/                    ← 運用・設定・デプロイ
│   ├── config/
│   ├── scripts/
│   ├── driver-ui/          （開発用成果物）
│   └── sql/
├── docs/                   ← ドキュメント
├── Cargo.toml              ← root workspace
├── package.json
└── README.md
```

**利点**:

- ディレクトリ名で責務が一目瞭然
- ドライバの個別開発・CI/CD が容易
- 将来 submodule 化や split への段階
- Cargo workspace が構造を表現

#### Task 5.1: core/ へ移動（フロント・バック）

- **対象**: `src/`, `src-tauri/` ディレクトリ
- **作業**:
  1. `mkdir -p core`
  2. `mv src core/`
  3. `mv src-tauri core/`
  4. `ls core/` で確認：`src/`, `src-tauri/` が存在
- **受け入れ条件**: ファイル欠落なし、ビルド成功

#### Task 5.2: drivers/ へ移動（ドライバ群）

- **対象**: `apps/postgres/`, `apps/joywatcher/` ディレクトリ
- **作業**:
  1. `mkdir -p drivers`
  2. `mv apps/postgres drivers/`
  3. `mv apps/joywatcher drivers/`
  4. `rmdir apps/` （空ディレクトリ削除）
- **受け入れ条件**: ドライバファイル欠落なし

#### Task 5.3: ops/ へ移動（運用・設定）

- **対象**: `config/`, `scripts/`, `driver-ui/`, `sql/` ディレクトリ
- **作業**:
  1. `mkdir -p ops`
  2. `mv config ops/`
  3. `mv scripts ops/`
  4. `mv driver-ui ops/`
  5. `mv sql ops/`
  6. `ls ops/` で確認
- **受け入れ条件**: すべてのディレクトリが ops 配下に移動

#### Task 5.4: Cargo.toml パス修正

- **対象**:
  - `Cargo.toml` （root）
  - `core/src-tauri/Cargo.toml`
  - 各ドライバ `Cargo.toml`
- **作業**:
  1. root `Cargo.toml` の `members` 更新：

     ```toml
     [workspace]
     members = [
       "core/src-tauri",
       "drivers/postgres/driver",
       "drivers/postgres/ui",
       "drivers/joywatcher/driver",
       "drivers/joywatcher/ui",
       "drivers/joywatcher/bridge-x86",
       "packages/protocol-rs",
       "packages/driver-ui-host",
     ]
     ```

  2. 各 `Cargo.toml` の relative path 修正（例: `../config/` → `../../ops/config/`）
  3. `cargo build --workspace` 実行して検証
- **受け入れ条件**: `cargo build --workspace` と `cargo build -p <package>` 成功

#### Task 5.5: Scripts パス修正

- **対象**: `ops/scripts/*.ps1`
- **作業**:
  1. 各スクリプトの相対パス参照を修正
     - 例: `../config` → `../ops/config` （スクリプトが ops/ にあるため）
     - 例: `./src-tauri` → `./core/src-tauri`
  2. 実行テスト：
     - `ops/scripts/build-release-driver-suite.ps1` 実行
     - `ops/scripts/build-dev-joywatcher-ui.ps1` 実行
- **受け入れ条件**: すべてのスクリプトが相対パス修正完了 + 実行確認

#### Task 5.6: CI/CD・ドキュメント更新

- **対象**:
  - `.github/workflows/*.yml` （あれば）
  - `docs/deployment.md`
  - `docs/driver-development.md`
  - `README.md`
  - `AGENTS.md`
  - `.github/copilot-instructions.md`
- **作業**:
  1. CI/CD パス修正（src-tauri/ → core/src-tauri/ など）
  2. ドキュメント内の相対パス参照を更新
  3. 「ドライバの個別開発」セクションを追加
     - `cd drivers/postgres && cargo build` 手順
     - `cd drivers/postgres/ui && npm run build` 手順
- **受け入れ条件**: ドキュメント・スクリプト参照が正確

#### Task 5.7: .gitignore / .git パス再設定

- **対象**: `.gitignore`, `Cargo.lock`
- **作業**:
  1. `.gitignore` の相対パス修正（必要に応じ）
  2. git が新構造を認識：`git status` で差分が正しく見える
  3. `git add -A && git status` で確認
- **受け入れ条件**: `git status` に不要なファイルが表示されない

---

## タスク完了順序

**推奨順序**: 0 → 1 → 2 → 3 → 4

### 依存関係

```text
Phase 0 (ドキュメント)
  ↓ (理解深化)
Phase 1 (正本宣言)
  ↓ (基礎整備)
Phase 2 (スクリプト分離)
  ↓ (機械化)
Phase 3 (探索ロジック)
  ↓ (保守性向上)
Phase 4 (manifest 準備)
  ↓ (将来への接続)
Phase 5 (ディレクトリ構造化)
  ↓ (構造最適化)
```

### 各 Phase の推定時間

| Phase | タスク数 | 推定時間 |
| --- | --- | --- |
| 0 | 3 | 2-3h（ドキュメント中心） |
| 1 | 3 | 1-2h（設定ファイル追加） |
| 2 | 2 | 2-3h（スクリプト作成） |
| 3 | 2 | 2-3h（コード改修） |
| 4 | 2 | 1-2h（ドキュメント更新） |
| 5 | 7 | 4-6h（ディレクトリ移動・パス修正） |
| **合計** | **19** | **12-19h** |

---

## リスク・ガード条件

### リスク1: 既存スクリプト利用者への影響

- **ガード**: 既存スクリプト `build-release-driver-suite.ps1` は互換性維持
- **チェック**: Phase 2 完了後、`./scripts/build-release-driver-suite.ps1` が既存どおり動作

### リスク2: 二重配置時の同期漏れ

- **ガード**: `.staginginfo` / `.deploymentinfo` で「staging は read-only」を明記
- **チェック**: CI/CD に "validate staging == primary" チェック追加（Task 2 後）

### リスク3: Manifest 移行時の後方互換

- **ガード**: Phase 4 で期限付き deprecated 化
- **チェック**: v0.5.0 以降、`knownDriverTypes` 使用時に警告ログ

---

## 今すぐ実施できる小タスク（優先度高）

着手の障壁を最小化するため、以下から開始可能：

1. **Task 0.3** （スクリプトコメント統一）→ 30分以内
2. **Task 0.1** （ドキュメント更新）→ 1時間以内
3. **Task 1.1** （.deploymentinfo 作成）→ 15分以内

---

## Phase 5 の意義と実施条件

### 推奨実施タイミング

- Phase 0-4 すべて完了後
- manifest 移行が安定した後（v0.5.0 以降推奨）
- ドライバが 3 個以上に増えた段階

### Phase 5 実施のメリット

| メリット | 説明 |
| --- | --- |
| **スケーラビリティ** | ドライバ数が増えても構造が一貫 |
| **個別開発** | `cd drivers/postgres && cargo build` で単独コンパイル可能 |
| **CI/CD 最適化** | ドライバ単位でビルド・テスト・デプロイ可能 |
| **将来の submodule化** | 各ドライバが別リポジトリ化する前段階 |
| **チーム分割** | ドライバ開発チームが独立可能 |

### Phase 5 実施時のリスク

| リスク | 対策 |
| --- | --- |
| パス修正漏れ | Task 5.4-5.6 で入念に検証 |
| git history の複雑化 | 1 コミット = 1 タスクで粒度を保つ |
| CI/CD 一時的破損 | ローカル全ビルド成功後に CI/CD に進める |

### ガード条件（破損防止）

Phase 5 着手前に必ず確認：

- [ ] Phase 0-4 が完全に完了している
- [ ] `cargo build --workspace` が成功している
- [ ] すべてのスクリプトが動作確認済み
- [ ] manifest 駆動ディスカバリが運用安定している（v0.5.0+推奨）
- [ ] `git` 作業ツリーが clean（`git status` で何も表示されない）

---

## 段階的実施・ロールバック計画

### コミット粒度

各 Task を **1 コミット** で区切る：

```powershell
# Task 5.1: core/ へ移動
git add -A
git commit -m "refactor(P5-T5.1): Move src and src-tauri to core/"

# Task 5.2: drivers/ へ移動
git add -A
git commit -m "refactor(P5-T5.2): Move driver apps to drivers/"

# ... (Task 5.3-5.7 同様)
```

### ロールバック方法

問題が発生した場合、該当 Task のコミットに戻す：

```powershell
# 最後の Task（5.7）で問題があった場合
git log --oneline | head -20
# 5.6 のコミット ID を確認
git reset --hard <commit-id>
```

---

## 関連ドキュメント

- `docs/driver-development.md`
- `docs/deployment.md`
- `docs/driver-manifest-discovery-design.md`
- `docs/decisions.md`
- `README.md`
- `scripts/build-release-driver-suite.ps1`
