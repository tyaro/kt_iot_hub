# ドライバマニフェスト駆動ディスカバリ設計

## 1. 背景

現状の接続先ドライバ選択は、フロントエンドの `knownDriverTypes` による埋め込み定義と、
本体側の `driver-ui/<type>/registration-ui(.exe)` 探索の組み合わせで動作している。

この方式は既存 DriverType では動作するが、以下の課題がある。

- DriverType 追加時に本体改修（候補追加）が必要
- 「ドライバとしての自己記述情報」がなく、検証/互換判定が弱い
- ディレクトリへファイルを配置しただけで拡張する運用に不向き

本設計は、**設定画面で指定したドライバベースフォルダを走査し、マニフェストからドライバ候補を自動発見する**ための設計を定義する。

## 2. 目的と非目的

### 2.1 目的

- 本体の埋め込み DriverType 依存を解消し、追加拡張の本体改修を最小化する。
- ドライバ実体（登録UI/ランタイム）に関するメタ情報を明文化する。
- 検証可能な契約（schema version, 必須フィールド, 互換性判定）を導入する。

### 2.2 非目的

- 既存 TOML / Tauri コマンド / JSON 形の即時破壊的変更。
- Linux/macOS 向けパッケージ構成の同時対応（本設計の主対象は Windows 配布）。
- ドライバ署名検証までの即時実装（将来拡張とする）。

## 3. 用語

- Driver Package: 1 DriverType の実行物 + マニフェストを含む配布単位
- Driver Manifest: Driver Package の自己記述ファイル（`driver-manifest.json`）
- Discovery: ベースフォルダ走査で Driver Manifest を収集・検証・一覧化する処理

## 4. 現状との差分

### 4.1 現状

- DriverType 候補はフロント埋め込み（例: postgres, joywatcher）
- 本体は指定 DriverType について実行ファイル候補を探索
- メタ情報（表示名・バージョン・互換条件）は暗黙

### 4.2 目標

- DriverType 候補は Discovery 結果から生成
- 実行ファイルパスはマニフェストを第一候補として解決
- 互換性と無効理由をユーザーへ可視化

## 5. 配置規約

### 5.1 ベースフォルダ

- 設定画面の `driverUiBaseDir` を Discovery の起点とする。

### 5.2 推奨ディレクトリ構成

- `<base>/driver-ui/<driver_type>/driver-manifest.json`
- `<base>/driver-ui/<driver_type>/registration-ui.exe`
- `<base>/driver-ui/<driver_type>/driver-<driver_type>.exe`

### 5.3 後方互換

- 既存の `driver-ui/<type>/registration-ui(.exe)` 探索は当面維持する。
- マニフェストが無い既存ドライバは「legacy discovery」で扱う。

## 6. マニフェスト仕様（v1）

### 6.1 ファイル名

- `driver-manifest.json`（UTF-8, BOM なし）

### 6.2 必須項目

- `manifestVersion`: `1`
- `driverType`: 文字列（`[a-z0-9][a-z0-9_-]*`）
- `displayName`: 文字列
- `registrationUi`: 相対パス（例: `registration-ui.exe`）
- `runtime`: 相対パス（例: `driver-postgres.exe`）
- `protocol`: オブジェクト
  - `driverUiRequestVersion`
  - `driverUiResponseVersion`

### 6.3 任意項目

- `description`
- `vendor`
- `version`（SemVer 推奨）
- `homepage`
- `minHubVersion` / `maxHubVersion`
- `capabilities`（例: `tagBrowse`, `connectionTest`, `schemaProvided`）

### 6.4 例

```json
{
  "manifestVersion": 1,
  "driverType": "joywatcher",
  "displayName": "JoyWatcher 接続",
  "description": "TagSel2 ベースのタグ取込に対応",
  "version": "0.2.0",
  "vendor": "kt_iot_hub",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-joywatcher.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  },
  "capabilities": ["tagBrowse", "schemaProvided"]
}
```

## 7. Discovery 処理

### 7.1 走査ルール

1. `driverUiBaseDir` を正規化
2. `<base>/driver-ui/*/driver-manifest.json` を列挙
3. 各 manifest を parse + validate
4. `driverType` 重複を検出
5. 実行ファイル存在確認（相対パス解決）
6. 結果を `available` / `invalid` / `incompatible` へ分類

### 7.2 重複時ポリシー

- 同一 `driverType` が複数見つかった場合:
  - デフォルトは全件無効化し、UI に競合理由を表示
  - 将来は優先順位（バージョン高い方）をオプション化可

### 7.3 エラー可視化

- 例: `MANIFEST_PARSE_ERROR`, `MANIFEST_SCHEMA_ERROR`, `EXECUTABLE_NOT_FOUND`, `HUB_VERSION_INCOMPATIBLE`
- 設定画面/タグ管理画面から詳細を参照可能にする。

## 8. API 設計

### 8.1 新規コマンド（案）

- `discover_driver_packages(req)`
  - 入力: `driver_ui_base_dir`
  - 出力: `DriverPackageDiscoveryResult[]`

### 8.2 応答 DTO（案）

- `driver_type`
- `display_name`
- `manifest_path`
- `registration_ui_path`
- `runtime_path`
- `version`
- `available`
- `status_code`
- `status_message`
- `capabilities[]`

### 8.3 既存コマンドとの関係

- `check_driver_ui_available` は内部的に Discovery 結果参照へ寄せる。
- `launch_driver_ui` は `driver_id` 指定なし時、`driver_type` から Discovery で実体解決する。

## 9. UI 設計

### 9.1 ドライバ種別ピッカー

- 既存の `knownDriverTypes` 埋め込みを廃止し、Discovery 結果から表示。
- 表示項目: `displayName`, `driverType`, `version`, `status`。

### 9.2 設定画面

- `driverUiBaseDir` 保存時に Discovery を再実行。
- 無効ドライバの件数と理由を表示。

### 9.3 後方互換表示

- legacy discovery（manifest なし）で見つかったドライバは `legacy` バッジ表示。

## 10. セキュリティ/運用

- 初期段階: ローカル配置前提、実行ファイル存在とパス正規化を厳格化
- 将来拡張:
  - 署名検証（配布元検証）
  - 許可リスト（vendor / hash）
  - 実行前のポリシーチェック

## 11. 段階移行計画

### Phase 1（導入）

- マニフェスト仕様 v1 定義
- Discovery コマンド追加
- UI は Discovery 優先 + 既存埋め込み併用（フォールバック）

### Phase 2（切替）

- ドライバ種別候補を Discovery 由来へ全面切替
- `check_driver_ui_available` を Discovery 基盤化

### Phase 3（収束）

- 埋め込み `knownDriverTypes` を削除
- legacy discovery を段階縮退（必要に応じて期限設定）

## 12. 受け入れ条件

- 新しい Driver Package をベースフォルダへ配置すると、再起動または再読込で候補に表示される。
- 本体コード改修なしで DriverType 候補へ追加される。
- manifest 不備時に無効理由が UI とログへ表示される。
- 既存 postgres / joywatcher の運用が後方互換で維持される。

## 13. 未解決論点

1. `discover_driver_packages` の呼び出しタイミング（onMount / baseDir保存時 / 手動再読込）
2. 重複 `driverType` の既定ポリシー（全無効 vs 優先解決）
3. legacy discovery 廃止時期
4. `protocol` バージョン互換判定の厳密条件

## 14. 関連ドキュメント

- `docs/driver-property-extensibility-design.md`
- `docs/ui-registration.md`
- `docs/driver-development.md`
- `docs/config-spec.md`
- `docs/decisions.md`
