# 接続先プロパティ拡張設計（タグ管理 右ペイン）

## 1. 背景

タグ管理の右ペインに表示される接続先プロパティは、現在 `Host / Port / Database / Username / Password` 前提の固定レイアウトである。
この形は PostgreSQL には適合するが、JoyWatcher（JWS）には不適合であり、今後 DriverType が増えると本体改修コストが増大する。

本設計は、**ドライバ追加時に本体改修を最小化するための接続先プロパティ責務分離**を定義する。

## 2. 目的と非目的

### 2.1 目的

- DriverType ごとの接続先プロパティ差異を自然に扱える設計にする。
- 新規 DriverType 追加時に、本体 UI / DTO の変更量を最小化する。
- 既存 PostgreSQL フローを壊さず段階的に移行する。

### 2.2 非目的

- 既存コマンド/JSON/TOML の互換性を即時に破壊する変更。
- 1 回の変更で全ドライバを統一 UI に切り替えること。
- ドライバ UI と本体 UI の責務を完全に同一化すること。

## 3. 現状の課題

1. 本体 DTO が PostgreSQL 形に寄っている
   - `DriverDto` / `SaveDriverRequest` が `host/port/database/username/password` を固定フィールドとして持つ。
2. 右ペイン編集フォームが固定
   - DriverType に応じた項目切替ができない。
3. ドライバ固有知識が本体に流入
   - 本体 UI 変更なしに DriverType を増やしにくい。

## 4. 設計方針（採用）

### 4.1 方針サマリ

**採用方針: ハイブリッド方式**

- 接続先プロパティ定義の正本は `driver.settings`（JSON）とする。
- 入力 UX の主担当はドライバ UI（別プロセス）とする。
- 本体右ペインは以下を担当する。
  - 共通メタ情報の表示（`id`, `driver_type`, `enabled`）
  - `settings` の要約表示（read-only）
  - 必要に応じた汎用編集（スキーマがある場合のみ）

### 4.2 期待効果

- 新しい DriverType 追加時に、本体へ固定フィールドを足す作業を避けられる。
- ドライバ固有 UX（探索、候補生成、専用バリデーション）をドライバ UI に閉じ込められる。
- 本体は「保存・検証・表示」に集中できる。

## 5. 責務分担

### 5.1 本体（kt_iot_hub）

- `drivers.toml` の永続化と整合性検証
- 右ペインでの共通情報表示
- ドライバ UI 起動・結果取込
- 最低限の整合性チェック（必須キー、型、driver_type 整合）

### 5.2 ドライバ UI

- DriverType 固有の接続先入力フォーム
- 接続テスト/探索/補助入力
- 保存用 payload（driver + scanGroups + tags）生成

### 5.3 ドライバ runtime

- 実運転時に必要な設定キーの解釈
- 接続確立、値取得、再試行

## 6. データモデル

### 6.1 正本

- 接続先設定は `DriverConfig.settings: serde_json::Value` を正本とする。
- 固定フィールドを増やすのではなく、`settings` のキー空間を DriverType ごとに定義する。

### 6.2 互換方針

既存 UI/IPC 互換のため、段階移行中は以下を許容する。

- 既存の固定フィールド DTO は暫定維持（PostgreSQL 向け）
- 追加で `settings` 原文（object）を返せる API を導入
- 最終的には UI を `settings` 中心へ移行

## 7. 右ペイン UI 方針

### 7.1 表示モード

- 標準: 共通情報 + settings 要約（read-only）
- 編集: 原則「ドライバ UI で編集」導線を優先

### 7.2 例外（フォールバック）

- 専用ドライバ UI が無い DriverType のみ、本体側汎用フォームを許可
- 汎用フォームはスキーマ駆動（JSON Schema）を前提とする

## 8. API 移行方針（段階）

### Phase 1（互換維持）

- 既存 `list_drivers` / `save_driver` は維持
- 右ペインは DriverType が `joywatcher` の場合、固定 Postgres フォームを表示しない
- `settings` 要約表示 + 「編集はドライバ UI」導線へ切替

### Phase 2（拡張 API 追加）

- 例: `list_drivers_v2` / `save_driver_v2`（名称は実装時に最終決定）
- `settings` object を一次データとして扱う
- 必要に応じて `schema`（または schemaId）を返す

### Phase 3（統一）

- 既存固定フィールド依存 UI を縮退
- 汎用表示 + 専用ドライバ UI の構成へ統一

## 9. リスクと対策

1. 既存 PostgreSQL UX の退行
   - 対策: Phase 1 は既存フォーム維持、JoyWatcher から先に切替
2. settings キー不整合
   - 対策: ドライバ UI 側で入力検証 + 本体側で最低限の schema 検証
3. 互換破壊
   - 対策: コマンド追加で移行し、既存コマンドは段階的に廃止

## 10. 受け入れ条件（設計時点）

- JoyWatcher 選択時に右ペインが PostgreSQL 固定項目を表示しない。
- 右ペインからドライバ UI 編集導線で往復できる。
- 新規 DriverType の追加時に本体へ固定フィールド追加を要求しない。
- 既存 PostgreSQL 保存フローが動作維持される。

## 11. 実装前に確定すべき論点

1. `list_drivers_v2/save_driver_v2` の命名と導入順
2. schema の置き場所（本体内固定 / ドライバ配布物 / protocol-rs）
3. 右ペイン汎用フォームをいつ導入するか（Phase 2 同時 or 後続）

## 12. 参考

- `docs/ui-registration.md`
- `docs/architecture.md`
- `docs/config-spec.md`
- `docs/driver-development.md`
