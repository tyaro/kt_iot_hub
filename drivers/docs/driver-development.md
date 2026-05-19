<!-- このファイルは「実装手順 + UI/UX基準 + チェックリスト」を一体で参照できるようにするため 300 行を超える。 -->

# ドライバUI / 通信ドライバ 作成ガイド

> 本書は、`kt_iot_hub` に新しい `driver_type` を追加する際の実装手順をまとめたガイドです。
> 対象は **登録UI（`registration-ui`）** と **通信ランタイム（`driver-<type>`）** のセットです。

## このドキュメントの使い方

別セッションの AI エージェントや開発者が途中から作業を再開する場合は、以下の順で読む。

1. 本書の「セッション再開時の最小確認項目」
2. `docs/ui-registration.md`
3. `docs/config-spec.md`
4. 参考実装

- `drivers/postgres/ui/src/main.rs`
- `drivers/postgres/ui/assets/app.js`
- `drivers/postgres/driver/src/main.rs`
- `apps/common/driver_runtime_grpc_client.rs`

この順に読むことで、以下を短時間で判断できるようにする。

- 今回追加する DriverType の責務範囲
- どのファイルを編集すべきか
- どこまで実装できれば完了か
- 何を検証すれば既存仕様と整合するか

## セッション再開時の最小確認項目

新しいセッションで着手する際は、まず以下だけ確認する。

### 1. 追加対象の `driver_type`

- 例: `postgres`, `slmp`, `joywatcher`
- 本体設定・配置先・バイナリ名はこの値に依存する

### 2. 実装対象の範囲

- 登録UI だけか
- 通信ランタイムだけか
- 両方か
- 本体側の取り込みや起動導線まで含むか

### 3. 正本ドキュメント

- UI フロー正本: `docs/ui-registration.md`
- 返却 JSON / 型の正本: `packages/protocol-rs`
- 設定の正本: `docs/config-spec.md`
- 実装例の正本: `drivers/postgres/ui/`, `drivers/postgres/driver/`

### 4. 完了条件

最低限、以下が満たせること。

- タグ管理画面から登録UIを起動できる
- 新規 / 編集の両モードで JSON を返却できる
- 通信ランタイムが `driver-<type>` として起動できる
- gRPC 経由で本体へタグ値送信できる
- `npm run check` と `cargo test` が通る

## 先に見るべきファイル一覧

### 本体側

- `core/src-tauri/src/commands/bridge.rs`
  - 登録UI 起動コンテキストや受け渡しの入口
- `core/src-tauri/src/commands/driver_ui_protocol.rs`
  - 返却 JSON 型の再公開
- `core/src-tauri/src/commands/tag.rs`
  - タグ管理からの取り込みや反映導線の確認先
- `core/src-tauri/src/app_state.rs`
  - 実行時共有状態の確認先

### 登録UI 側の参考実装

- `drivers/postgres/ui/src/main.rs`
  - 最小 Tauri シェル
- `drivers/postgres/ui/assets/app.js`
  - 3 ステップ UI、接続テスト、候補生成、保存処理

### 通信ランタイム側の参考実装

- `drivers/postgres/driver/src/main.rs`
  - 起動引数、初期化、実行ループ
- `apps/common/driver_runtime_grpc_client.rs`
  - 本体との gRPC 通信

### 配置 / ビルド

- `ops/scripts/build-dev-driver-ui.ps1`
- `ops/scripts/install-driver-ui.ps1`

## 推奨実装順序

別セッションでも再開しやすくするため、以下の順で小さく進める。

1. `driver_type` と配置名を決める
2. `packages/protocol-rs` で不足型がないか確認する
3. 登録UI の最小起動を作る
4. launch context の読込と save output をつなぐ
5. 新規 / 編集モードの初期表示を作る
6. ドライバ固有の接続テストと候補取得を作る
7. 保存 JSON を本体が取り込める形にする
8. 通信ランタイムの CLI と gRPC 接続を作る
9. DriverDefinition 取得とタグ値送信をつなぐ
10. 配置スクリプトと README / docs を更新する

途中で止まっても再開しやすいように、各段階で「単独で成立する最小状態」を残す。

例:

- 登録UI は、最初はダミー JSON 保存だけでもよい
- 通信ランタイムは、最初は定義取得ログ出力だけでもよい

## 最小実装雛形

最初の 1 コミット目は、以下の「起動するだけ」の骨組みから始めるとよい。

実ファイルとして使い回せるテンプレートは以下に置く。

- `drivers/docs/templates/driver-ui-main-template.rs`
- `drivers/docs/templates/driver-ui-app-template.js`
- `drivers/docs/templates/driver-runtime-main-template.rs`
- `drivers/docs/templates/driver-runtime-grpc-client-template.rs`

全体の確認順は `drivers/docs/driver-implementation-flow.md` を参照。

### 登録UI の最小 `src/main.rs`

```rust
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use kt_driver_ui_host::bridge;

#[tauri::command]
fn close_driver_ui_window(window: tauri::WebviewWindow) -> Result<(), String> {
  window
    .close()
    .map_err(|e| format!("failed to close driver ui window: {}", e))
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      close_driver_ui_window,
      bridge::get_driver_ui_launch_context,
      bridge::save_driver_ui_output,
    ])
    .run(tauri::generate_context!())
    .expect("error while running driver ui");
}
```

この段階の到達目標:

- Tauri ウィンドウが起動する
- launch context を取得できる
- save output の呼び出し口だけ先に作れる

### 通信ランタイムの最小 `src/main.rs`

```rust
use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tracing::{error, info, warn};

mod grpc_client;

#[derive(Debug, Parser)]
#[command(author, version, about = "Runtime driver process")]
struct Args {
  #[arg(long)]
  driver_id: String,

  #[arg(long)]
  driver_kind: String,

  #[arg(long, default_value = "127.0.0.1:55051")]
  grpc_addr: String,
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_target(true)
    .with_level(true)
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  if let Err(e) = run().await {
    error!("runtime driver terminated with error: {}", e);
    std::process::exit(1);
  }
}

async fn run() -> Result<()> {
  let args = Args::parse();
  info!(
    "runtime driver starting: driver_id={} kind={} grpc={}",
    args.driver_id, args.driver_kind, args.grpc_addr
  );

  loop {
    match grpc_client::connect(&args.grpc_addr, &args.driver_id, &args.driver_kind).await {
      Ok(_) => info!("gRPC connected"),
      Err(e) => warn!("gRPC connect failed: {}", e),
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
  }
}
```

この段階の到達目標:

- `driver-<type>` バイナリとして起動できる
- CLI 引数を受け取れる
- gRPC 接続のリトライループを先に確保できる

### 最初の段階で作るとよいファイル

```text
apps/foo/ui/
  Cargo.toml
  src/main.rs
  assets/index.html
  assets/app.js

apps/foo/driver/
  Cargo.toml
  src/main.rs
  src/grpc_client.rs
```

最初から機能を詰め込みすぎず、

1. 起動
2. 受け渡し
3. 保存
4. 値送信

の順で積み上げる。

## 対象と前提

新しいドライバは、原則として以下の 2 つを 1 セットで実装する。

1. **登録UI**
   - 接続先設定
   - ScanGroup 設定
   - Tag 設定
   - 本体へ JSON 返却
2. **通信ランタイム**
   - 本体からドライバ定義を取得
   - 外部機器 / DB / SDK と通信
   - タグ値を gRPC 経由で本体へ送信

本体側は Driver 固有知識を持ちすぎない方針であるため、以下を守る。

- 本体 UI に Driver 固有の編集画面を埋め込まない
- 登録UI は **別プロセス / 別ウィンドウ** とする
- 通信ランタイムは **別プロセス** とする
- 本体との契約は `packages/protocol-rs` と gRPC 定義に従う

## 全体構成

新しい `driver_type = "foo"` を追加する場合の推奨構成:

```text
drivers/
  foo/
    ui/
      Cargo.toml
      src/main.rs
      assets/
        index.html
        app.js
        styles.css
    driver/
      Cargo.toml
      src/main.rs
      src/<driver_logic>.rs
      src/grpc_client.rs

ops/driver-ui/
  foo/
    registration-ui.exe
    driver-foo.exe
```

補足:

- `drivers/<type>/ui/` は登録UI 実装
- `drivers/<type>/driver/` は通信ランタイム実装
- 配布時の配置は `ops/driver-ui/<type>/` に揃える

## 実装の責務分担

### 登録UI の責務

- 接続先設定入力
- 接続テスト
- 外部機器 / DB / SDK からの候補一覧取得
- ScanGroup 構成の編集
- タグ候補の選択と命名
- 本体へ返却する JSON の生成

### 通信ランタイムの責務

- 起動引数受取（`--driver-id`, `--driver-kind`, `--grpc-addr`）
- 本体から DriverDefinition を取得
- 定義に基づく周期読出し
- タグ値のストリーミング送信
- 再接続 / 再試行 / 異常時ログ出力

### 本体の責務

- ドライバUI 起動
- 返却 JSON の検証
- `drivers.toml` / `tags.toml` 反映
- 通信ランタイム起動・停止

## 追加手順

### 1. `driver_type` と命名規約を決める

- `driver_type` は短く明確にする
  - 例: `postgres`, `slmp`, `joywatcher`
- 推奨バイナリ名:
  - 登録UI: `registration-ui.exe`
  - 通信ランタイム: `driver-<type>.exe`
- crate 名は衝突しないようにする
  - 例: `driver_ui_foo`, `driver-foo`

### 2. 登録UI を作成する

最小構成は PostgreSQL 実装を踏襲する。

#### Tauri 側

- `drivers/<type>/ui/src/main.rs` を作成
- 本体との橋渡しには `kt_driver_ui_host` を使う
- 最低限必要なコマンド:
  - `bridge::get_driver_ui_launch_context`
  - `bridge::save_driver_ui_output`
  - `close_driver_ui_window`
- Driver 固有の接続テスト / 一覧取得コマンドを追加する

#### フロント側

- オフライン前提のため、静的アセットで完結させる
- `assets/index.html`, `assets/app.js`, `assets/styles.css` を持つ
- 本体から渡された launch context を初期値へ反映する
- 最終的に `driver`, `scanGroups`, `tags` を含む JSON を返却する

### 3. 通信ランタイムを作成する

最小構成は `drivers/postgres/driver/` を踏襲する。

#### 起動引数

- `--driver-id`
- `--driver-kind`
- `--grpc-addr`

#### 必須フロー

1. gRPC 接続
2. DriverDefinition 取得
3. 定義から poller / client を組み立て
4. タグ値を本体へ送信（`TagValueMessage.io_rx_bytes_total` / `io_tx_bytes_total` にドライバ側累積I/Oを設定）
5. エラー時は待機後に再試行

#### 実装上の注意

- Tokio で統一する
- 長時間ループは再接続可能な構造にする
- panic ではなく `Result` で返す
- ログは `tracing` を使う

### 4. 共有プロトコルを使う

Driver UI が返却する JSON 型は、必ず `packages/protocol-rs` を正本とする。

使う主な型:

- `DriverUiImportPayload`
- `DriverUiDriverPayload`
- `DriverUiScanGroupPayload`
- `DriverUiTagPayload`

返却 JSON 仕様を独自に増やす場合は、まず `packages/protocol-rs` を更新する。

### 5. 本体から起動できる形に配置する

推奨配置:

```text
ops/driver-ui/<driver_type>/registration-ui.exe
ops/driver-ui/<driver_type>/driver-<driver_type>.exe
```

配置責務:

- `ops/driver-ui/<driver_type>/` は開発時正本（運用開始時の起点）
- `core/src-tauri/driver-ui/<driver_type>/` は bundle staging（同梱直前の同期先）
- staging は正本ではないため、直接編集しない

実行時探索（通信ランタイム）:

1. 設定された driver-ui ベースパス配下の同居配置
2. `DRIVER_BIN_DIR` 配下
3. 本体実行ファイルと同じディレクトリ
4. `PATH`

実行時探索（登録UI）:

- `driver_ui_base_dir` 指定時はその配下を優先
- 続いて resources / app directory などの実行環境由来候補を探索
- 実装詳細は `core/src-tauri/src/commands/driver/ui_launcher/paths.rs` を参照

スクリプト責務:

- `build-*`: ビルドのみ（`target/` 出力）
- `install-*`: `target/` から正本 `ops/driver-ui/` へ配置
- `stage-*`: 正本 `ops/driver-ui/` から `core/src-tauri/driver-ui/` へ同期

PowerShell スクリプトで配置できるようにする。

- 既存:
  - `ops/scripts/install-driver-ui.ps1`
- 必要に応じて通信ランタイム用 install script も同様の形式で揃える

### 6. ドキュメントとチェック項目を更新する

新しい DriverType を追加したら、少なくとも以下を更新する。

- `docs/ui-registration.md`
  - 登録フロー差分
- `docs/config-spec.md`
  - `driver_spec` 仕様
- `docs/decisions.md`
  - 新しい設計判断がある場合
- `README.md`
  - 構成または配置が増える場合

## 登録UI の UI/UX 基準

新しい Driver UI は自由に作ってよいが、ユーザー体験は揃える。

### 1. 新規登録と編集は同一 UI

- 新規登録専用画面と編集専用画面を分けない
- 見出し、初期値、補助文言でモード差分を表現する
- 既存接続先編集時は、保存済みの接続先・ScanGroup・タグを必ず復元する

### 2. ステップ構成は明確にする

推奨構成:

1. 接続先設定
2. グループ / タグ設定
3. 確認・保存

要件:

- ステッパまたは見出しで現在位置を明示する
- 「戻る」「進む」「キャンセル」を統一配置する
- 最終ステップでは保存前レビューを必須表示する

### 3. 一覧と編集の同時視認性を確保する

- 左に対象一覧、右に編集フォームの 2 カラムを基本とする
- 一覧で選んだ対象が、右側に即反映されること
- 選択中 / 保存済み / 未設定の状態が視覚的に分かること

### 4. 長時間処理には進捗と再試行を出す

対象:

- 接続テスト
- テーブル / アドレス一覧取得
- 列一覧取得
- 外部 SDK 呼び出し

要件:

- 処理中表示を出す
- 失敗時は原因を短く表示する
- 再試行操作を用意する

### 5. 大量タグ登録を前提にする

- 検索フィルタを置く
- 全選択 / 全解除を置く
- 時系列列や主キー列を除外しやすくする
- 命名規則がある場合は一括適用を用意する

### 6. 保存前レビューを省略しない

保存前レビューで最低限表示するもの:

- 接続先 ID
- 接続先情報（host / endpoint / station など）
- ScanGroup 数
- タグ数
- 警告（重複、未設定、既定値利用など）

### 7. エラーメッセージは操作可能にする

- 「何が悪いか」を一文で出す
- 可能なら入力箇所を特定する
- 技術詳細は必要最小限に留める

悪い例:

- `invalid payload`
- `unknown error`

良い例:

- `時系列フィールドを選択してください`
- `接続先IDが既存定義と重複しています: postgresql1`

### 8. オフライン前提を守る

- 外部 CDN を使わない
- ブラウザから直接ネット依存リソースを読み込まない
- 依存は同梱可能なものに限定する

## 通信ランタイムの品質基準

### 必須

- 起動直後に gRPC 接続できなくても再試行する
- 定義取得失敗でも再試行する
- 一時的な接続断から復帰できる
- 値送信ループが停止してもプロセスが即終了しない

### 推奨

- 読出し周期を `ScanGroup` 単位で扱う
- 外部ライブラリ例外を吸収して `warn` ログ化する
- 将来的な複数 ScanGroup 並列化を見据えた責務分離にする

## 実装チェックリスト

### 登録UI

- [ ] `get_driver_ui_launch_context` を利用している
- [ ] `save_driver_ui_output` で返却できる
- [ ] 新規 / 編集の両モードを扱える
- [ ] 保存前レビューがある
- [ ] エラー時に再試行可能

### 通信ランタイム

- [ ] `--driver-id` / `--driver-kind` / `--grpc-addr` を受け取る
- [ ] DriverDefinition を取得している
- [ ] タグ値を gRPC で stream している
- [ ] `TagValueMessage.io_rx_bytes_total` / `io_tx_bytes_total` を単調増加する累積値で送っている（再起動時リセットは許容）
- [ ] 再接続 / 再試行がある
- [ ] `tracing` でログを出している

### 本体連携

- [ ] `driver-ui/<type>/registration-ui.exe` に配置できる
- [ ] `driver-ui/<type>/driver-<type>.exe` に配置できる
- [ ] `drivers.toml` / `tags.toml` に反映できる
- [ ] タグ管理画面から起動できる

## 受け入れ条件

別セッションの AI エージェントが「完了」と判断できるよう、受け入れ条件を明示する。

### 登録UI の受け入れ条件

- 新規接続先作成時に初期値なしで起動できる
- 既存接続先編集時に保存済み内容が復元される
- 接続テスト失敗時に再試行導線がある
- 保存時に `driver`, `scanGroups`, `tags` を含む JSON を返却する
- 保存前レビューに件数と警告が表示される

### 通信ランタイムの受け入れ条件

- `driver-<type>` バイナリとして起動できる
- `--driver-id`, `--driver-kind`, `--grpc-addr` を解釈できる
- DriverDefinition を取得できる
- 少なくとも 1 件のタグ値送信が成功する
- 一時的な接続失敗後に再試行する

### ワークスペース全体の受け入れ条件

- 追加したドキュメントへの参照が残る
- `npm run check` が成功する
- `cargo test` または影響範囲の Rust テストが成功する
- 変更ファイルと未実装事項を次セッションへ引き継げる状態になっている

## よくある詰まりどころ

### `packages/protocol-rs` と実装がずれる

- 症状: 保存 JSON は出るが本体が取り込めない
- 確認先: `packages/protocol-rs`, `src-tauri/src/commands/driver_ui_protocol.rs`

### 配置先は正しいが本体が実行ファイルを見つけられない

- 症状: タグ管理画面から UI 起動時に見つからない
- 確認先: `driver-ui/<type>/registration-ui.exe`, `driver-ui/<type>/driver-<type>.exe`
- 補足: 開発時ビルド成果物は workspace ルート `target/` に出る

### JSON 取込後に UI へ反映されない

- 症状: `tags.toml` は更新されたように見えるが画面反映されない
- 確認先: 本体取り込み後の registry 同期処理
- 補足: 反映順序を誤ると config と runtime 状態がずれる

### 新規モードと編集モードで UI が分岐しすぎる

- 症状: 一方の修正でもう一方が壊れる
- 対応: 画面は共通にし、見出し・初期値・補助文言だけ切り替える

## セッション引き継ぎメモの書き方

別セッションへ引き継ぐときは、少なくとも以下を残す。

- 定型フォーマットは `drivers/docs/templates/driver-session-handoff-template.md` を使う

- 今回対象の `driver_type`
- 完了した段階
- 未完了の段階
- 変更したファイル
- 手動確認した項目
- 未確認項目
- 次に開くべきファイル 3 つ

推奨テンプレート:

```text
対象 driver_type: foo
完了: 登録UI の起動 / launch context 読込 / ダミー保存
未完了: 接続テスト / 候補一覧 / runtime の値送信
変更ファイル:
- apps/foo/ui/src/main.rs
- apps/foo/ui/assets/app.js
- drivers/docs/driver-development.md
確認済み:
- UI 起動
- save_driver_ui_output
未確認:
- 本体取込
- driver-foo 起動
次に見るファイル:
- apps/foo/driver/src/main.rs
- packages/protocol-rs/src/lib.rs
- src-tauri/src/commands/bridge.rs
```

## PostgreSQL 実装を参考にする場所

- 登録UI 起動点:
  - `apps/postgres/ui/src/main.rs`
- 登録UI フロント実装:
  - `apps/postgres/ui/assets/app.js`
- 通信ランタイム起動点:
  - `apps/postgres/driver/src/main.rs`
- gRPC クライアント:
  - `apps/postgres/driver/src/grpc_client.rs`
- 配置スクリプト:
  - `scripts/build-dev-driver-ui.ps1`
  - `scripts/install-driver-ui.ps1`

  ## まずコピペして始めるときの参照順

  1. `drivers/docs/driver-implementation-flow.md`
  2. `drivers/docs/templates/driver-ui-main-template.rs`
  3. `drivers/docs/templates/driver-ui-app-template.js`
  4. `drivers/docs/templates/driver-runtime-main-template.rs`
  5. `drivers/docs/templates/driver-runtime-grpc-client-template.rs`
  6. `drivers/docs/templates/driver-session-handoff-template.md`

## 非推奨事項

- 本体 UI に Driver 固有フォームを直接追加する
- 登録UI と通信ランタイムを 1 プロセスに混ぜる
- `packages/protocol-rs` を通さず独自 JSON を返す
- panic / unwrap に依存した制御フロー
- 外部 CDN やネット接続を前提にした UI
