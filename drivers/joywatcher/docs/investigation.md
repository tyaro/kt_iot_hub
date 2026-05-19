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
- 補足:
  - JoyWatcher のタグ型は登録時点では確定しない
  - 実際の型は `JWRead` の戻り `TCOM_DATA1.dtype` を見て読取時に確定する
  - 既知の型候補は `SHORT` / `LONG` / `SINGLE` / `DOUBLE` / `BIT` / `LSTRING` / `USHORT` / `ULONG`
  - 現行 bridge では `BIT -> bool`、`LSTRING -> string`、それ以外の数値系は `number` へ集約して扱う

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
- `TCOM_DATA1.dtype` により読取時の型判定を行う前提
- 登録 UI の `dataType` は便宜上の仮置き値であり、JoyWatcher 側の実型を保証するものではない

#### 2026-05-18 実機確認で確定したこと

- `JWGetTagIDS2` で解決した `nativeTagId` を `JWRead` へ渡す経路は実機で動作する
- x86 bridge からの `JWRead` は、`TCOM_DATA1` の ABI レイアウトが正しくないとメモリ崩れを起こす
- `TCOM_DATA1` は vendor 定義どおり **x86 / MSVC レイアウトで 32 byte** として扱う必要がある
- `JWRead` の戻り値は、少なくとも今回の実機確認では **0 を返しても結果行が有効** だった
- したがって bridge 実装では、`JWRead` の成否を `0/1` の bool 的な値だけで決め打ちしない

#### `TCOM_DATA1` 実装上の注意

vendor の `common/com.h` では以下の形で定義されている。

- `long col_id;`
- `union { double dblVal; char pbVal[16]; };`
- `char dtype;`

x86 / MSVC では `double` を含む union により 8 byte alignment がかかるため、実メモリ配置は以下になる。

- `col_id`: offset 0
- padding: 4 byte
- value union (`dblVal` / `pbVal[16]`): offset 8
- `dtype`: offset 24
- trailing padding: 7 byte
- total size: 32 byte

このレイアウトに合わせないと、`JWRead` 呼び出し後に `col_id` や `dtype` が崩れて見える。

#### 実タグで確認した `dtype` 対応

以下のタグを JoyWatcher サーバ側へ登録し、`JWRead` で実測確認した。

- `LOCAL$TEST.WORD0$VALUE`
- `LOCAL$TEST.DWORD0$VALUE`
- `LOCAL$TEST.DWORD1$VALUE`
- `LOCAL$TEST.SINGLE0$VALUE`
- `LOCAL$TEST.DOUBLE0$VALUE`
- `LOCAL$TEST.BIT0$VALUE`
- `LOCAL$TEST.STR0$VALUE`
- `LOCAL$TEST.UWORD0$VALUE`
- `LOCAL$TEST.ULONG0$VALUE`

実測できた `dtype` と bridge の現在の型マッピングは以下。

| JoyWatcher テストタグ | `dtype` | 現行 bridge の返却型 |
| --- | ---: | --- |
| `WORD0` | 0 | `number` |
| `DWORD0` | 1 | `number` |
| `DWORD1` | 1 | `number` |
| `SINGLE0` | 2 | `number` |
| `DOUBLE0` | 3 | `number` |
| `BIT0` | 4 | `bool` |
| `STR0` | 5 | `string` |
| `UWORD0` | 6 | `number` |
| `ULONG0` | 7 | `number` |

補足:

- 2026-05-18 時点では、値そのものは `0.0` / `false` / `""` で返った
- これは型判定失敗ではなく、JoyWatcher サーバ側の現在値がその状態である可能性が高い
- 現行 bridge は `dtype` を JSON 応答へそのまま返さず、`bool | number | string` へ集約する

### 型の扱い

- 登録時:
  - `TagSel2` / `JWGetTagIDS2` だけでは型は確定しない
  - UI 上は互換性維持のため暫定 `dataType` を保存する
- 読取時:
  - `JWRead` の `TCOM_DATA1.dtype` を正とする
  - 実運用上の型は読取時に確定する
- 現行実装の扱い:
  - `BIT` は `bool`
  - `LSTRING` は `string`
  - `SHORT` / `LONG` / `SINGLE` / `DOUBLE` / `USHORT` / `ULONG` は `number` として返す
  - 数値系の厳密な元型名までは、現行 bridge の JSON 応答には保持していない

#### 現時点の仕様として確定したこと

- 登録 UI の「型確認 (`JWRead`)」で見るべき型は、保存済み `dataType` ではなく **読取時の `dtype` から導いた型** である
- `dataType` は暫定値として保持してもよいが、JoyWatcher 実型の正本ではない
- 実運用で UI / runtime が信頼すべき値は **`JWRead` の `dtype` に基づく型** である
- ただし現行 bridge は数値系を `number` に集約するため、`WORD` / `DWORD` / `SINGLE` / `DOUBLE` / `UWORD` / `ULONG` の細分類は UI へ出していない

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
