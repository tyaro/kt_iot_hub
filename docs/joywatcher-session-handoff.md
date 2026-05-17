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
- `apps/joywatcher/bridge-x86/src/dll_api.rs` を追加し、`LoadLibraryW` / `GetProcAddress` による DLL ローダを実装した
- `scripts/build-dev-joywatcher-bridge-x86.ps1` を追加し、x86 bridge の開発用ビルド / 配置を自動化した
- `apps/joywatcher/driver/src/joywatcher_bridge.rs` を追加し、runtime から x86 bridge を起動して `ping` / `connect` する最小統合を実装した
- `apps/joywatcher/ui/assets/app.js` を更新し、保存 JSON が `driverSpec.nativeTagId` を保持できるようにした
- `ConnectNet.htm` の `CDaoDatabase*` 記述と同梱サンプルを突き合わせ、DAO ハンドルはタグ解決の正規ルートとみなさず、当面は `JWGetTagIDS2` / `JWRead` を主経路とする方針を確認した
- `apps/joywatcher/ui/src/joywatcher_bridge_client.rs` を追加し、登録UI から x86 bridge を起動して単一タグの `JWGetTagIDS2` を呼ぶ最小導線を実装した
- `apps/joywatcher/bridge-x86/src/dll_api.rs` に `JWGetTagIDS2` 実装を追加し、dll モードの `resolveTags` が動くようにした
- `apps/joywatcher/bridge-x86/src/dll_api.rs` に `JWRead` 実装を追加し、dll モードの `read` が `TCOM_DATA1` を `bool | number | string` へ変換できるようにした
- `apps/joywatcher/driver/src/joywatcher_runtime.rs` を追加し、`driverSpec.nativeTagId` を driver definition から拾って bridge の `read` 結果を `TagValueMessage` へ変換する最小ポーリング計画を実装した
- `apps/joywatcher/driver/src/main.rs` / `joywatcher_bridge.rs` を更新し、bridge への接続設定保持・`read` 実行・gRPC 送信の最小導線を追加した
- `apps/joywatcher/driver/src/joywatcher_runtime.rs` / `main.rs` を更新し、scan group の `scan_rate_ms` に基づく継続ポーリングへ切り替えた
- `scripts/build-dev-joywatcher-ui.ps1` / `scripts/build-dev-joywatcher-runtime.ps1` / `scripts/build-dev-joywatcher-suite.ps1` を追加し、`driver-ui/joywatcher/` へ dev 成果物をまとめて配置できるようにした

## まだ未完了のこと

- JoyWatcher DLL / LIB 実体の配置方針確認（`参考/JoyWaApi.dll` と `C:\Windows\SysWOW64\JoyWaApi.dll` は確認済み）
- `JoyWApi.h` をベースにした FFI 設計、または 32bit 別プロセスブリッジ方式の確定
- 登録UI からの接続テスト / `JWGetTagIDS2` 実行 / `nativeTagId` 保存の実機確認
- `ConnectNet` / `DisconnectNet` の呼出規約差分の実機確認
- `endpoint` / `user_id` / `password` の設定キー名固定

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
- `apps/joywatcher/driver/src/joywatcher_bridge.rs`
- `apps/joywatcher/driver/src/joywatcher_runtime.rs`
- `apps/joywatcher/bridge-x86/Cargo.toml`
- `apps/joywatcher/bridge-x86/src/main.rs`
- `apps/joywatcher/bridge-x86/src/protocol.rs`
- `apps/joywatcher/bridge-x86/src/connection.rs`
- `apps/joywatcher/bridge-x86/src/mock_api.rs`
- `apps/joywatcher/bridge-x86/src/service.rs`
- `apps/joywatcher/bridge-x86/src/dll_api.rs`
- `scripts/build-dev-joywatcher-bridge-x86.ps1`
- `scripts/build-dev-joywatcher-ui.ps1`
- `scripts/build-dev-joywatcher-runtime.ps1`
- `scripts/build-dev-joywatcher-suite.ps1`
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
- [x] `joywatcher-bridge-x86 --mode dll` で DLL ローダが動作し、現環境では `os error 193` により x86 / x64 不一致が明示される
- [x] `cargo build -p joywatcher-bridge-x86 --target i686-pc-windows-msvc` が成功し、x86 ビルド済み EXE の `--mode dll` で `ping` / `connect` / `disconnect` が構造化応答を返す
- [x] `driver-joywatcher` が `nativeTagId` を bridge の `read` へ渡し、`TagValueMessage` へ変換できる形までビルド / テスト確認済み
- [x] `cargo test -p joywatcher-bridge-x86` が `JWGetTagIDS2` 実装追加後も成功する
- [x] `cargo test -p joywatcher-bridge-x86` が `JWRead` 実装追加後も成功する
- [x] `cargo test --manifest-path apps/joywatcher/ui/Cargo.toml` が成功する
- [x] `cargo test -p driver-joywatcher` が `read` / `nativeTagId` 連携追加後も成功する
- [x] `cargo test -p driver-joywatcher` が scan rate ベースの継続ポーリング追加後も成功する
- [x] `scripts/build-dev-joywatcher-runtime.ps1` が `driver-ui/joywatcher/driver-joywatcher.exe` を配置できる
- [x] `scripts/build-dev-joywatcher-ui.ps1` が `driver-ui/joywatcher/registration-ui.exe` を配置できる
- [x] `scripts/build-dev-joywatcher-suite.ps1` が UI / runtime / bridge をまとめて配置できる

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
- `ConnectNet.htm` には `CDaoDatabase*` 戻り値の記述があるが、ヘッダは `BOOL` / `long` を返しており資料間で揺れている
- `ConnectNet` の DAO ハンドルは「接続可能サーバ情報の DB」と読める一方、タグ解決は同梱サンプルでも `JWGetTagIDS2` を使っている
- `参考/JoyWaApi/BC/JoyWaApi.lib` は存在し、対応する `JoyWaApi.dll` 実体も `参考/JoyWaApi.dll` / `C:\Windows\SysWOW64\JoyWaApi.dll` で確認できた
- `参考/JoyWaApi/BC/JoyWaApi.lib` は `__IMPORT_DESCRIPTOR_JoyWaApi` / `__NULL_IMPORT_DESCRIPTOR` / `__imp_` を含み、`JoyWaApi.dll` を参照する import lib とみてよい
- `参考/JoyWaApi.dll` と `C:\Windows\SysWOW64\JoyWaApi.dll` は同サイズ・同更新日時で存在し、実 DLL は x86 (Machine `0x14C`) と確認できた
- DLL export には `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` / `GetTagCount` / `GetTagName` / `JWAsyncSelect` / `JWAsyncSelect2` が見えている
- `参考/JoyWaApi/BC/Project2.dll` は `JoyWApi.h` の API 本体ではなく、サンプル/ラッパ DLL の可能性が高い
- `apps/joywatcher/bridge-x86` は現時点で mock 実装。`ping` / `connect` / `disconnect` / `forceDisconnect` / `resolveTags` / `read` を JSON Lines で返す
- `apps/joywatcher/bridge-x86/src/dll_api.rs` で DLL ローダは追加済み。ただし現在の開発ビルドは x64 のため、x86 DLL ロード時に `os error 193` となる
- `i686-pc-windows-msvc` ターゲットを追加済みで、x86 ビルド済み `joywatcher-bridge-x86.exe --mode dll` は起動できる
- x86 ビルド済み bridge では `connect` / `disconnect` が少なくともクラッシュせず構造化応答を返す
- runtime 側は x86 bridge を優先探索するよう更新済み
- bridge ログが stdout に混ざると runtime 側の JSON 読取が壊れるため、bridge は stderr へログ出力し、runtime 側も JSON 行のみ採用するよう修正済み
- JoyWatcher の数値 tagId は本体タグ ID と別物なので、保存時は `driverSpec.nativeTagId` として分離する方針
- 登録UI から単一タグの `nativeTagId` を解決する Tauri コマンド `resolve_joywatcher_tag` を追加済み
- bridge の `connect` は `user_id` / `password` を保持し、`JWRead` 呼び出し時に再利用する実装へ更新済み
- `driver-joywatcher` は driver definition の `driver_spec_json` から `nativeTagId` を抜き出して gRPC 送信値へ変換する
- `driver-joywatcher` は `scan_group.scan_rate_ms` ごとに read を回し続ける継続ポーリング実装へ更新済み
- ただし UI 上の実機手動確認、設定キー名の固定、呼出規約の実機確認はまだ未実施

## 次セッションで最初に見るファイル

1. `apps/joywatcher/driver/src/joywatcher_runtime.rs`
2. `apps/joywatcher/driver/src/joywatcher_bridge.rs`
3. `scripts/build-dev-joywatcher-suite.ps1`

## 次の最小タスク

1. 登録UI の `resolve_joywatcher_tag` を実機で手動確認する
2. `ConnectNet` / `DisconnectNet` の呼出規約差分を実機で確定する
3. `endpoint` / `user_id` / `password` の設定キー名を UI / runtime / bridge 間で固定する
4. DAO ハンドル調査が必要になった場合は、Rust ではなく x86 / MFC C++ shim を別途切る
5. 必要なら bridge 再起動時の再定義取得 / 再接続戦略を調整する

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
- 追加確認: `cargo build -p joywatcher-bridge-x86 --target i686-pc-windows-msvc`
- 追加確認: `cargo test --manifest-path apps/joywatcher/ui/Cargo.toml`
- 追加確認: `$env:RUST_LOG='info'; .\target\debug\driver-joywatcher.exe -- --driver-id jw-test --driver-kind joywatcher --grpc-addr 127.0.0.1:59999`
- 追加確認: `cargo build --manifest-path apps/joywatcher/ui/Cargo.toml`
- 追加確認: `cargo build --manifest-path apps/joywatcher/driver/Cargo.toml`
- 追加確認: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-dev-joywatcher-runtime.ps1`
- 追加確認: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-dev-joywatcher-ui.ps1`
- 追加確認: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-dev-joywatcher-suite.ps1`

## 補足

- 開発時ビルド成果物は workspace ルートの `target/` に出る
- 配置規約は `driver-ui/joywatcher/registration-ui.exe` と `driver-ui/joywatcher/driver-joywatcher.exe`
- 返却 JSON の正本は `packages/protocol-rs`
- DLL 探索は `JOYWATCHER_DLL_DIR` → カレントディレクトリ → 実行ファイル近傍 → `参考/` → `参考/JoyWaApi` → `参考/JoyWaApi/BC` → `C:\Windows\SysWOW64` の順で確認する
