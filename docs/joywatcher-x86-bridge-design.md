# JoyWatcher x86 ブリッジ設計メモ

> 対象: `driver_type = joywatcher`
> 目的: 32bit 前提の可能性が高い JoyWatcher DLL を、本体 Rust ランタイムから安全に利用するための中継方式を定義する。

## 背景

現時点で確認できている事実:

- `参考/JoyWaApi/BC/JoyWaApi.lib` は存在する
- `JoyWaApi.lib` には `__IMPORT_DESCRIPTOR_JoyWaApi` / `__NULL_IMPORT_DESCRIPTOR` / `__imp_` を含む文字列が見つかっている
- `JoyWaApi.lib` 内には `JoyWaApi.dll` を参照する文字列が含まれている
- `JoyWaApi.lib` のサイズは約 10KB と小さく、多数 API の実装本体を含む静的ライブラリとしては不自然
- `参考/JoyWaApi.dll` と `C:\Windows\SysWOW64\JoyWaApi.dll` の存在を確認した
- `JoyWaApi.dll` の PE Machine は `0x14C` で、x86 (32bit) DLL と確認できた
- `参考/JoyWaApi/BC/Project2.dll` は API 本体ではなくサンプル DLL の可能性が高い
- `JoyWApi.h` / ヘルプは `_cdecl` ベース
- `BC/JoyWApi.h` は `ConnectNet` / `DisconnectNet` に `_stdcall` 表記がある
- 参考資料とプロジェクト設定は `Win32 (x86)` 色が強い

このため、64bit の `driver-joywatcher` から DLL を直接ロードする案は、ABI / 呼出規約 / 32bit 制約の観点で不確実性が高い。

また、`C:\Windows\SysWOW64\JoyWaApi.dll` に配置される点と PE Header の Machine 値から、実 DLL は 32bit 前提と判断してよい。

## 結論

JoyWatcher は **x86 専用ブリッジプロセス** を別途用意し、`driver-joywatcher` 本体とは IPC で接続する構成を第一候補とする。

### 採用理由

- 32bit DLL クラッシュを `driver-joywatcher` 本体から隔離できる
- Rust 本体は 64bit のまま維持できる
- 呼出規約差分や Borland 由来の ABI 問題をブリッジ側へ閉じ込められる
- 将来 DLL 本体の差し替えやバージョン差分吸収がしやすい

## 非採用案

### 1. 64bit Rust ランタイムから DLL を直接ロード

非採用理由:

- DLL が x86 の場合、同一プロセスでロードできない
- `ConnectNet` / `DisconnectNet` の呼出規約差分が未確定
- import lib があっても実 DLL が無いと実行不可

### 2. `JoyWaApi.lib` を静的ライブラリとして直接リンク

非採用理由:

- `JoyWaApi.lib` は import lib とみなしてよいだけの材料が揃っている
- Borland 由来 `.lib` は MSVC / GNU 系ツールチェインとの互換性が不明
- 32bit / 64bit 問題は解消しない

## `JoyWaApi.lib` 判定メモ

現時点の暫定結論は **「`JoyWaApi.lib` は静的ライブラリではなく `JoyWaApi.dll` 向け import lib」** である。

### 判定根拠

- COFF archive シグネチャ `!<arch>` を持つ
- `__IMPORT_DESCRIPTOR_JoyWaApi` を含む
- `__NULL_IMPORT_DESCRIPTOR` を含む
- `__imp_` プレフィックス付きシンボルを含む
- `JoyWaApi.dll` 文字列を含む
- ファイルサイズが約 10KB と小さい

## `JoyWaApi.dll` 実体確認メモ

現時点で **実 DLL は確保済み** とみなせる。

### 確認できたこと

- `参考/JoyWaApi.dll` が存在する
- `C:\Windows\SysWOW64\JoyWaApi.dll` も存在する
- 両者のサイズは 118,853 bytes で一致する
- 更新日時も一致している
- PE Machine は `0x14C` で x86 DLL である
- ヘルプ `ConnectNet.htm` には `CDaoDatabase * _cdecl ConnectNet(void);` という記述がある
- 同梱ヘッダ群は `BOOL` / `long` を返す宣言になっており、`ConnectNet` の戻り値型は資料間で揺れている

### エクスポートで見えた主要関数

- `ConnectNet`
- `DisconnectNet`
- `DisconnectNetForce`
- `GetTagCount`
- `GetTagName`
- `JWAsyncSelect`
- `JWAsyncSelect2`

少なくとも `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` が DLL 実体に存在することは確認できた。

## `ConnectNet` の DAO ハンドルに関するメモ

`ConnectNet.htm` では戻り値が `CDaoDatabase*` とされているが、現時点では **これをタグ一覧取得の正規ルートとはみなさない**。

### 根拠

- ヘルプ本文は「接続可能なサーバ情報が入った DAO データベースハンドルを返す」と読め、タグ DB を返すとは断定できない
- `JoyWApi.h` / `BC/JoyWApi.h` / `JoyWaApi.hpp` の宣言は `BOOL` / `long` で、`CDaoDatabase*` と整合しない
- 同梱サンプル `DllApiSampleDlg.cpp` は `ConnectNet()` の戻り値を使わず、タグ名解決をすべて `JWGetTagIDS2()` で行っている
- `JWGetTagIDS2.htm` / `JWWrite.htm` / `JWRead.htm` のサンプルも `ConnectNet()` の後に `JWGetTagIDS2()` を呼ぶ構成で、DAO 経由のタグ列挙例は見当たらない

### 実務上の判断

- `ConnectNet()` は「接続確立の前提を作る API」として扱う
- タグ名 → JoyWatcher ネイティブ ID の解決は `JWGetTagIDS2()` を正規ルートとして扱う
- 値読取は `JWRead()` を使う
- `CDaoDatabase*` の中身を本当に調べるなら、Rust ではなく **x86 / MFC C++ の調査用 shim** を別途作るのが安全

### いま採らない方針

- x86 bridge から `CDaoDatabase*` を MFC オブジェクトとして直接触りに行く
- DAO のテーブル構造が分からないまま、登録UIのタグ解決を `JWGetTagIDS2()` から外す

### 実務上の扱い

- `JoyWaApi.lib` 単体では実行時の API 本体にならない
- `JoyWaApi.dll` 実体は確保できたので、次は export と呼出規約の実装確認へ進める
- `Project2.dll` は代替本体として扱わず、別物として切り分けて確認する

### 追加で確認するなら

1. DLL の全エクスポート名を確認する
2. `ConnectNet` / `DisconnectNet` の呼出規約を DLL 実体基準で確定する
3. `JWGetTagIDS2` / `JWRead` が実 DLL にあるか確認する
4. x86 ブリッジ前提で `apps/joywatcher/bridge-x86/` を進める

## 提案アーキテクチャ

```text
kt_iot_hub.exe
  └ driver-joywatcher.exe        (64bit 想定 / Rust)
       ├ gRPC: GetDriverDefinition / StreamTagValues
       └ IPC client
            ↓
       joywatcher-bridge-x86.exe  (32bit 想定 / Rust or C/C++)
            ├ JoyWaApi.dll をロード
            ├ ConnectNet / DisconnectNet / JWGetTagIDS2 / JWRead
            └ JoyWatcher サーバと通信
```

## 責務分担

### `driver-joywatcher.exe`

- 本体 gRPC サーバーとの通信
- DriverDefinition の取得
- ScanGroup 単位のポーリング計画
- ブリッジプロセス起動 / 停止 / 再接続
- UI で確定保存された `tagPath -> nativeTagId` を読み、`nativeTagId` ベースで読取する
- ブリッジから受けた値を `StreamTagValues` へ流す
- ブリッジ異常終了時の再試行制御

### `joywatcher-bridge-x86.exe`

- JoyWaApi.dll のロード
- `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` の参照カウント管理
- 登録UI から呼ばれる `JWGetTagIDS2` によるタグ名 → ID 解決
- `JWRead` による値取得
- C ABI / 呼出規約差分の吸収
- 最小限のエラーコードとメッセージを IPC で返却

## IPC 方針

第一候補は **標準入出力ベースの JSON Lines** とする。

### 現在の実装状況

- `apps/joywatcher/bridge-x86/` を追加済み
- `joywatcher-bridge-x86` は `mock` モードで起動する
- `joywatcher-bridge-x86` に `dll` モードを追加し、`LoadLibraryW` / `GetProcAddress` による `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` の最小ローダを実装した
- `ping` / `connect` / `disconnect` / `forceDisconnect` / `resolveTags` / `read` の最小応答を持つ
- IPC は `stdin` / `stdout` の JSON Lines で疎通確認済み
- `resolveTags` は dll モードで `JWGetTagIDS2` を呼ぶ実装を追加済み
- 登録UI 側には `resolve_joywatcher_tag` コマンドと「Tag ID を解決」ボタンを追加し、単一タグの `nativeTagId` をフォームへ反映できるようにした
- `read` は dll モードで `JWRead` を呼び、`TCOM_DATA1` を `bool | number | string` へ変換する実装を追加済み
- 現在の開発環境で `cargo run -p joywatcher-bridge-x86 -- --mode dll` を実行すると `os error 193` で失敗し、x86 DLL を x64 プロセスへロードできないことを確認した
- `cargo build -p joywatcher-bridge-x86 --target i686-pc-windows-msvc` は成功し、x86 ビルド済み EXE では `--mode dll` の起動が成功する
- x86 ビルド済み EXE で `connect` / `disconnect` を送ると、`active_connections: 1 -> 0` の構造化応答が返ることを確認した
- `driver-joywatcher` から x86 bridge を起動する最小統合を追加済み
- runtime 側は `driver-ui/joywatcher/joywatcher-bridge-x86.exe` → `target/i686-pc-windows-msvc/debug/joywatcher-bridge-x86.exe` の順で x86 bridge を優先探索する
- bridge 側ログは stderr へ出し、runtime 側は stdout から JSON 行だけ読むようにしたため、`ping` / `connect` 応答がログ混線で壊れない
- `driver-joywatcher` は `driverSpec.nativeTagId` を読んで bridge の `read` を呼び、結果を `StreamTagValues` 用 `TagValueMessage` へ変換する最小経路を実装済み
- `driver-joywatcher` は `scan_group.scan_rate_ms` ごとにタグを束ね、継続ポーリングしながら gRPC ストリームへ値を流す実装へ更新済み
- `scripts/build-dev-joywatcher-ui.ps1` / `scripts/build-dev-joywatcher-runtime.ps1` / `scripts/build-dev-joywatcher-suite.ps1` を追加し、`driver-ui/joywatcher/` へ UI / runtime / bridge を配置できるようにした

### 理由

- x86 ブリッジを小さく保てる
- 依存追加なしで開始しやすい
- デバッグ時に入出力を目視しやすい
- 将来 Named Pipe / gRPC へ置換しやすい

### メッセージ例

#### driver → bridge

```json
{"type":"connect","endpoint":"localhost","userId":0,"password":""}
{"type":"resolveTags","tags":["Line1/Tank/Level","Line1/Tank/Temp"]}
{"type":"read","requestId":"r1","tagIds":[101,102]}
{"type":"disconnect"}
```

#### bridge → driver

```json
{"type":"connected"}
{"type":"resolvedTags","items":[{"tagPath":"Line1/Tank/Level","tagId":101}]}
{"type":"readResult","requestId":"r1","values":[{"tagId":101,"value":12.3,"quality":"good"}]}
{"type":"error","code":"CONNECT_FAILED","message":"..."}
```

## 最小プロトコル

### 初期版で必要なコマンド

1. `connect`
2. `disconnect`
3. `forceDisconnect`
4. `resolveTags`
5. `read`
6. `ping`

### 初期版で必要な応答

1. `connected`
2. `disconnected`
3. `resolvedTags`
4. `readResult`
5. `pong`
6. `error`

## 接続ライフサイクル

ヘルプ上の制約:

- `ConnectNet` は複数回呼んでもよい
- 呼んだ回数ぶん `DisconnectNet` が必要
- 異常時の退避として `DisconnectNetForce` を使う

したがってブリッジ側は以下を守る。

1. `connect` 受信で参照カウントを 1 増やす
2. `disconnect` 受信で参照カウントを 1 減らす
3. 0 未満へは絶対にしない
4. 異常終了前のクリーンアップで `DisconnectNetForce` を試す
5. タグ解決や読取は「接続済み」でのみ受け付ける

## タグ解決フロー

1. UI または設定から `tagPath` を受け取る
2. 登録UI が `resolveTags` をブリッジへ送り、`JWGetTagIDS2` で `nativeTagId` を得る
3. 登録UI は `driverSpec.tagPath` と `driverSpec.nativeTagId` をセットで保存する
4. `driver-joywatcher` は設定読込時に保存済み `nativeTagId` を利用する
5. 再読取時は `nativeTagId` 配列で `read` を送る

補足:

- システム内の永続タグ識別子は従来どおり `tag.id`
- JoyWatcher DLL が返す数値 ID は `driverSpec.nativeTagId` として別管理する
- これにより本体タグ ID と JoyWatcher ネイティブ ID を混同しない
- `ConnectNet` の戻り値に DAO ハンドル記述はあるが、登録UIのタグ解決は当面 `JWGetTagIDS2` を使う

## 値読取フロー

1. `driver-joywatcher` が poll 周期で保存済み `nativeTagId` をまとめて `read` へ送る
2. ブリッジが `JWRead` を実行する
3. `TCOM_DATA1` を `bool | number | string` へ変換する
4. 応答 JSON として返す
5. `driver-joywatcher` がタグ定義へ突き合わせて gRPC 送信する

## 配置案

### 開発時

```text
apps/joywatcher/driver/          # 64bit runtime
apps/joywatcher/bridge-x86/      # 将来追加予定の x86 bridge
```

### 配布時

```text
driver-ui/joywatcher/
  registration-ui.exe
  driver-joywatcher.exe
  joywatcher-bridge-x86.exe
  JoyWaApi.dll                   # 再配布可否確認後
```

## ビルド方針

- `driver-joywatcher.exe` は既存 workspace の Rust ビルドで管理する
- `joywatcher-bridge-x86.exe` は 32bit ターゲット専用ビルドを別途用意する
- x86 ビルドは CI で無理に通さず、当面はローカル / 専用ジョブ扱いでよい

### 開発時のビルド補助

- `scripts/build-dev-joywatcher-bridge-x86.ps1` を追加済み
- このスクリプトは `i686-pc-windows-msvc` で `joywatcher-bridge-x86` をビルドし、`driver-ui/joywatcher/joywatcher-bridge-x86.exe` へ配置する
- `scripts/build-dev-joywatcher-ui.ps1` は `driver_ui_joywatcher.exe` をビルドし、`driver-ui/joywatcher/registration-ui.exe` へ配置する
- `scripts/build-dev-joywatcher-runtime.ps1` は `driver-joywatcher.exe` をビルドし、`driver-ui/joywatcher/driver-joywatcher.exe` へ配置する
- `scripts/build-dev-joywatcher-suite.ps1` は UI / runtime / bridge をまとめてビルドし、`driver-ui/joywatcher/` 配下へ揃える

## エラー処理方針

### ブリッジ側

- DLL ロード失敗
- 必須シンボル解決失敗
- ConnectNet 失敗
- タグ未解決
- JWRead 失敗

はすべて `error` 応答へ正規化する。

### runtime 側

- 一時失敗は `warn` ログ + リトライ
- ブリッジ再起動後に `resolveTags` を再実行
- 連続失敗時も本体プロセスは落とさない

## 受け入れ条件

### Phase A: 設計確認

- x86 ブリッジ構成の責務分担が文書化されている
- 必要 API が `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` / `JWGetTagIDS2` / `JWRead` に絞られている
- 配置案と IPC 方針が明記されている

### Phase B: ブリッジ骨組み

- `joywatcher-bridge-x86.exe` が起動する
- `ping` / `pong` が通る
- DLL 不在時に明確なエラーを返す

現状:

- `joywatcher-bridge-x86` の mock 実装は作成済み
- `cargo test -p joywatcher-bridge-x86` は成功
- JSON Lines の `ping` / `connect` / `resolveTags` / `read` スモーク確認済み
- `dll` モードの最小ローダも作成済み
- ただし実 DLL は x86 のため、現状の x64 開発ビルドでは `LoadLibraryW` が `os error 193` で失敗する

### Phase C: 実通信

- `connect` / `disconnect` が成功する
- 少なくとも 1 件の `tagPath -> tagId` 解決ができる
- 少なくとも 1 件の `JWRead` 結果を `driver-joywatcher` 経由で gRPC 送信できる

現状:

- x86 bridge の `connect` / `disconnect` は最小往復まで確認済み
- `driver-joywatcher` からの bridge 起動と `ping` / `read` 実装、`nativeTagId` ベースの値読取導線を追加済み
- 登録UI の保存 JSON は `driverSpec.nativeTagId` を保持できる形へ更新済み
- UI から bridge を使って単一タグの `resolveTags` を呼ぶ導線は追加済み
- `JWRead` の実 DLL 化と gRPC 送信への最小統合は追加済み
- scan group ごとの継続ポーリングと `driver-ui/joywatcher/` への dev 配置スクリプトは追加済み
- 残タスクは UI の実機手動確認、`ConnectNet` / `DisconnectNet` 呼出規約の実機確定、設定キー名の固定

## 次の最小タスク

1. 登録UI の `resolve_joywatcher_tag` を実機で手動確認し、妥当な `nativeTagId` が返るか確認する
2. `ConnectNet` / `DisconnectNet` の呼出規約（`_cdecl` / `_stdcall`）を実機で確定する
3. `endpoint` / `user_id` / `password` の設定キー名を UI / runtime / bridge 間で固定する
4. 必要なら bridge 再起動時の再接続戦略を調整する
