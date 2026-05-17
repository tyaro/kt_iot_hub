# JoyWatcher ドライバ実装 引き継ぎメモ

## 基本情報

- 対象 `driver_type`: `joywatcher`
- 対象範囲:
  - [x] 登録UI
  - [x] 通信ランタイム
  - [ ] 本体取り込み
  - [x] ドキュメント更新
- 参照中の正本ドキュメント:
  - `docs/driver-development.md`
  - `docs/ui-registration.md`
  - `docs/config-spec.md`

## 今回完了したこと

- `apps/joywatcher/ui` を追加し、launch context 読込・手動 ScanGroup/Tag 編集・保存までの最小登録UIを作成した
- `apps/joywatcher/driver` を追加し、`driver-joywatcher` として起動し gRPC で `GetDriverDefinition` できる最小 runtime 雛形を作成した
- 参考資料から JoyWatcher API が x86 前提の可能性が高いことを確認し、runtime 側に注意ログを入れた
- `apps/joywatcher/driver/src/joywatcher_connection.rs` を追加し、`ConnectNet` / `DisconnectNet` / `DisconnectNetForce` を前提にした接続ライフサイクルラッパ雛形と単体テストを用意した
- `apps/joywatcher/driver/src/joywatcher_ffi.rs` を追加し、`TCOM_DATA1` / `JWRead` / `JWGetTagIDS2` の最小 FFI 契約と値デコード補助を定義した
- `apps/joywatcher/driver/src/joywatcher_artifacts.rs` を追加し、`JoyWaApi.dll` / `JoyWaApi.lib` / `Project2.dll` の探索と起動時警告を実装した
- `docs/joywatcher-x86-bridge-design.md` を追加し、x86 ブリッジ方式の責務分担・IPC・配置案を整理した
- `apps/joywatcher/bridge-x86` を追加し、JSON Lines ベースの `joywatcher-bridge-x86` mock 実装を作成した

## まだ未完了のこと

- JoyWatcher DLL / LIB 実体の配置方針確認（`参考/JoyWaApi.dll` と `C:\Windows\SysWOW64\JoyWaApi.dll` は確認済み）
- `JoyWApi.h` をベースにした FFI 設計、または 32bit 別プロセスブリッジ方式の確定
- `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` を実DLLに結び付ける FFI 実装
- `driver-joywatcher` から `joywatcher-bridge-x86` を起動して接続する導線
- 登録UI からの接続テスト / タグ一覧自動取得 / runtime からのタグ値送信実装

## 変更ファイル

- `Cargo.toml`
- `apps/joywatcher/ui/Cargo.toml`
- `apps/joywatcher/ui/build.rs`
- `apps/joywatcher/ui/tauri.conf.json`
- `apps/joywatcher/ui/src/main.rs`
- `apps/joywatcher/ui/assets/index.html`
- `apps/joywatcher/ui/assets/styles.css`
- `apps/joywatcher/ui/assets/app.js`
- `apps/joywatcher/driver/Cargo.toml`
- `apps/joywatcher/driver/build.rs`
- `apps/joywatcher/driver/src/main.rs`
- `apps/joywatcher/driver/src/grpc_client.rs`
- `apps/joywatcher/driver/src/joywatcher_connection.rs`
- `apps/joywatcher/driver/src/joywatcher_ffi.rs`
- `apps/joywatcher/driver/src/joywatcher_artifacts.rs`
- `apps/joywatcher/bridge-x86/Cargo.toml`
- `apps/joywatcher/bridge-x86/src/main.rs`
- `apps/joywatcher/bridge-x86/src/protocol.rs`
- `apps/joywatcher/bridge-x86/src/connection.rs`
- `apps/joywatcher/bridge-x86/src/mock_api.rs`
- `apps/joywatcher/bridge-x86/src/service.rs`
- `docs/joywatcher-x86-bridge-design.md`
- `docs/joywatcher-session-handoff.md`

## 手動確認済み

- [ ] 登録UI が起動する
- [x] `get_driver_ui_launch_context` が読める（ビルド済み / コード接続済み）
- [x] `save_driver_ui_output` が呼べる（コード接続済み）
- [ ] 新規モードで保存できる
- [ ] 編集モードで既存値が復元される
- [ ] 通信ランタイムが起動する
- [ ] gRPC 接続が成功する
- [ ] タグ値を 1 件以上送信できる
- [x] `joywatcher-bridge-x86` が起動し、標準入出力 JSON Lines で応答する

## 未確認 / 要確認

- 実 DLL は確認できたが、runtime からの実ロード / 呼出規約整合は未確認
- `JoyWaApi.dll` は `C:\Windows\SysWOW64` に配置され、PE Machine `0x14C` の x86 DLL と確認できた
- 本体から `driver-ui/joywatcher/registration-ui.exe` / `driver-joywatcher.exe` を探索させる導線は未配置

## 発生した問題とメモ

- `参考/JoyWaApi/DllApiSample.dsp` は `Win32 (x86)` / `/machine:I386` でビルドされている
- `参考/JoyWaApiHelp/Active X API.htm` にも 32bit / x86 を示す記述がある
- 現在の runtime 雛形は DLL をまだロードしない。先に本体契約と CLI / gRPC 入口だけを固定している
- ヘルプには「`ConnectNet` は複数回呼んでよいが、呼んだ回数ぶん `DisconnectNet` を実行する」とある
- 念のため異常系の退避導線として `DisconnectNetForce` も FFI 対象に含める前提で進める
- `JoyWApi.h` と `BC/JoyWApi.h` で `ConnectNet` / `DisconnectNet` の呼出規約表記に差がある（`_cdecl` と `_stdcall`）ため、実装時に実DLLの export を確認する必要がある
- `参考/JoyWaApi/BC/JoyWaApi.lib` は存在し、対応する `JoyWaApi.dll` 実体も `参考/JoyWaApi.dll` / `C:\Windows\SysWOW64\JoyWaApi.dll` で確認できた
- `参考/JoyWaApi/BC/JoyWaApi.lib` は `__IMPORT_DESCRIPTOR_JoyWaApi` / `__NULL_IMPORT_DESCRIPTOR` / `__imp_` を含み、`JoyWaApi.dll` を参照する import lib とみてよい
- `参考/JoyWaApi.dll` と `C:\Windows\SysWOW64\JoyWaApi.dll` は同サイズ・同更新日時で存在し、実 DLL は x86 (Machine `0x14C`) と確認できた
- DLL export には `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` / `GetTagCount` / `GetTagName` / `JWAsyncSelect` / `JWAsyncSelect2` が見えている
- `参考/JoyWaApi/BC/Project2.dll` は `JoyWApi.h` の API 本体ではなく、サンプル/ラッパ DLL の可能性が高い
- `apps/joywatcher/bridge-x86` は現時点で mock 実装。`ping` / `connect` / `disconnect` / `forceDisconnect` / `resolveTags` / `read` を JSON Lines で返す

## 次セッションで最初に見るファイル

1. `apps/joywatcher/bridge-x86/src/main.rs`
2. `apps/joywatcher/bridge-x86/src/service.rs`
3. `apps/joywatcher/driver/src/joywatcher_ffi.rs`

## 次の最小タスク

1. `joywatcher-bridge-x86` に DLL ロードと `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` を結び付ける
2. `driver-joywatcher` からブリッジ子プロセスを起動する
3. `JWGetTagIDS2` / `JWRead` を実 DLL 呼び出しへ差し替える

## 完了条件の見込み

- [ ] タグ管理画面から登録UIを起動できる
- [ ] 新規 / 編集の両モードで JSON を返却できる
- [ ] `driver-joywatcher` が起動できる
- [ ] gRPC 経由で値送信できる
- [x] 影響範囲の確認コマンドが通る

## 確認コマンド

- ルート: `npm run check`
- 影響クレート: `cargo test -p driver_ui_joywatcher`
- 影響クレート: `cargo test -p driver-joywatcher`
- 影響クレート: `cargo test -p joywatcher-bridge-x86`
- 追加確認: `cargo build --manifest-path apps/joywatcher/ui/Cargo.toml`
- 追加確認: `cargo build --manifest-path apps/joywatcher/driver/Cargo.toml`

## 補足

- 開発時ビルド成果物は workspace ルートの `target/` に出る
- 配置規約は `driver-ui/joywatcher/registration-ui.exe` と `driver-ui/joywatcher/driver-joywatcher.exe`
- 返却 JSON の正本は `packages/protocol-rs`
- DLL 探索は `JOYWATCHER_DLL_DIR` → カレントディレクトリ → 実行ファイル近傍 → `参考/` → `参考/JoyWaApi` → `参考/JoyWaApi/BC` → `C:\Windows\SysWOW64` の順で確認する
