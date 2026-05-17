# JoyWatcher 調査メモ（2026-05-17）

> 目的: JoyWatcher 連携でタグ選択・タグ解決・値取得をどの経路で実装するかを確定する。

## 結論

JoyWatcher 連携は **DLL の `TagSel2` をタグ選択の正規ルート** とし、
タグ解決は **`JWGetTagIDS2`**、値取得は **`JWRead`** を使う。

OCX / ActiveX 経路は調査済みだが、最終実装ルートとしては採用しない。

## 採用ルート

### 1. タグ選択

- 使用 API: `TagSel2(char *ed)`
- 役割: vendor のタグ選択ダイアログを開き、選択済みタグ名一覧を返す
- 現在の実装:
  - `apps/joywatcher/bridge-x86/src/dll_api.rs`
  - `apps/joywatcher/ui/src/main.rs`
  - `apps/joywatcher/ui/assets/app.js`
- UI では `browse_joywatcher_tags` から呼び出し済み

### 2. タグ名 → nativeTagId 解決

- 使用 API: `JWGetTagIDS2`
- 役割: タグ名文字列配列を JoyWatcher の数値 ID に変換する
- 保存先: `driverSpec.nativeTagId`
- 備考: 本体の `tag.id` とは別管理にする

### 3. 値取得

- 使用 API: `JWRead`
- 役割: `nativeTagId` 配列をまとめて読取り、bridge から runtime に返す
- 現在の実装:
  - x86 bridge 側で `TCOM_DATA1` を `bool | number | string` へ変換済み
  - `driver-joywatcher` 側で `TagValueMessage` に変換する導線あり

## ここまでで確認できたこと

### DLL / ABI / 配置

- `JoyWaApi.dll` は x86 DLL
- `C:\Windows\SysWOW64\JoyWaApi.dll` に実体あり
- `参考\JoyWaApi.dll` も同サイズで存在
- 64bit プロセスからは `os error 193` でロード不可
- したがって **x86 bridge プロセスが必須**

### `ConnectNet` / `DisconnectNet`

- DLL エクスポートに存在する
- x86 build 済み bridge から `connect` / `disconnect` の往復は確認済み
- 接続参照カウントは bridge 側で管理済み

### `TagSel2`

- `JoywApi.h` に宣言あり
- x86 bridge 側で `TagSel2` をロードし、戻りバッファを CRLF 分割してタグ一覧化する実装済み
- 登録 UI の `JoyWatcher タグ参照` から呼べる導線を実装済み
- **タグ選択 UI を vendor 標準に寄せられるため、採用**

### `JWGetTagIDS2`

- DLL 実装として利用可能
- 固定長バッファ配列でタグ名を渡し、対応する数値 ID を取得する経路を実装済み
- 登録 UI から単一タグの解決も可能

### `JWRead`

- DLL 実装として利用可能
- x86 bridge 側で複数 ID をまとめて読み、値を JSON 応答へ変換する実装済み
- 実タグ値の読取り経路まで確認済み

### OCX / ActiveX 調査結果

- 対象 OCX:
  - `JwComApi.ocx`
  - `JwAXApi.ocx`
  - `JWSrvCtl.ocx`
- 結論:
  - `JWSrvCtl.ocx` はサーバ管理用で、タグ列挙用途ではない
  - `JwAXApi.ocx` は `JwComApi.ocx` とほぼ同系統
  - `JwComApi.ocx` は WinForms `AxHost` で初期化すれば `Open()` 自体は成功する
  - ただし `PLCTagCount` / `PLCTagName` は **TagList に登録したタグだけ**を対象とする
  - 全タグ列挙 API ではない
- TagList の仕様:
  - 改行区切り文字列で複数タグ登録できる
  - 形式は `LOCAL$<タグ名>$VALUE`
  - 変更時に接続が切れるため再 `Open()` が必要

### OCX で確認できたことの価値

OCX 調査は最終ルートには採用しないが、以下の知見は有効だった。

- JoyWatcher のタグ名実形式が `LOCAL$<name>$VALUE` 系であること
- CHM / サンプルから TagList が改行区切り文字列だと確認できたこと
- OCX の `PLCTagCount` が「全タグ数」ではなく「登録タグ数」だと確認できたこと
- ActiveX 経路はタグ全件列挙用途には不向きだと確認できたこと

### `.jdf` / CHM 調査結果

- `LOCAL.jdf` / `dbdef.jdf` から 10,000 件超のタグ名候補を抽出できた
- `.jdf` は `REPO` ヘッダを持つ独自バイナリで SQLite ではない
- CHM 展開から以下を確認した
  - `TagList` は改行区切り
  - `PLCTagCount` は登録タグ数
  - .NET では `get_PLCTagName` / `set_PLCTagName` のようなアクセサが見える
- ただし、この経路は **補助的な調査材料** とし、実装の正規ルートにはしない

## 非採用にした経路

### OCX / ActiveX を本実装ルートにする案

非採用理由:

- タグ全件列挙 API ではない
- WinForms `AxHost` 前提で扱いが重い
- `TagList` の事前登録が必要で、DLL の `TagSel2` / `JWGetTagIDS2` より遠回り

### `GetTagCount` / `GetTagName` の ABI 推測で全件列挙する案

非採用理由:

- 変種探索が必要で不安定
- 実装を複雑にする割に、`TagSel2` 採用後は必須ではない
- 維持コストが高い

### `.jdf` を正規データソースにする案

非採用理由:

- 実行時サーバ状態と一致する保証がない
- DLL が持つ正規 API より優先する根拠が薄い
- 差分タグの説明が難しい

## 実装方針

1. 登録 UI では `TagSel2` でタグ選択する
2. 選択タグを `JWGetTagIDS2` で `nativeTagId` に解決する
3. 保存時は `driverSpec.nativeTagId` を持たせる
4. runtime は `nativeTagId` を使って `JWRead` する
5. bridge は x86 専用のまま維持する

## 片付け方針

- OCX 調査用の `apps/joywatcher/ocx-wrapper/` は削除対象
- 一時的に作成した `test_ocx.vbs` も削除対象
- `GetTagName` の ABI 探査コードは bridge から除去対象

## 次に見るべき場所

- `apps/joywatcher/bridge-x86/src/dll_api.rs`
- `apps/joywatcher/ui/src/main.rs`
- `apps/joywatcher/driver/src/joywatcher_runtime.rs`
