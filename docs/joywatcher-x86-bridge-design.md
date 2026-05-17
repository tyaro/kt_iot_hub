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

### エクスポートで見えた主要関数

- `ConnectNet`
- `DisconnectNet`
- `DisconnectNetForce`
- `GetTagCount`
- `GetTagName`
- `JWAsyncSelect`
- `JWAsyncSelect2`

少なくとも `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` が DLL 実体に存在することは確認できた。

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
- ブリッジから受けた値を `StreamTagValues` へ流す
- ブリッジ異常終了時の再試行制御

### `joywatcher-bridge-x86.exe`

- JoyWaApi.dll のロード
- `ConnectNet` / `DisconnectNet` / `DisconnectNetForce` の参照カウント管理
- `JWGetTagIDS2` によるタグ名 → ID 解決
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
- `resolveTags` / `read` はまだ mock 応答である
- 現在の開発環境で `cargo run -p joywatcher-bridge-x86 -- --mode dll` を実行すると `os error 193` で失敗し、x86 DLL を x64 プロセスへロードできないことを確認した

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
2. `driver-joywatcher` が `resolveTags` をブリッジへ送る
3. ブリッジが `JWGetTagIDS2` を呼んで `tagId` を得る
4. `driver-joywatcher` は `tagPath -> tagId` をキャッシュする
5. 読取時は `tagId` 配列で `read` を送る

## 値読取フロー

1. `driver-joywatcher` が poll 周期で `read` を送る
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

## 次の最小タスク

1. `joywatcher-bridge-x86` を x86 ターゲットでビルド・起動できるようにする
2. `driver-joywatcher` から `joywatcher-bridge-x86` を起動する導線を作る
3. `ConnectNet` / `DisconnectNet` の呼出規約（`_cdecl` / `_stdcall`）を実機で確定する
4. `JWGetTagIDS2` / `JWRead` を実 DLL 呼び出しへ差し替える
