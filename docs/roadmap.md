# 実装ロードマップ

## 開発フェーズ案

| フェーズ | 内容 |
| --------- | ------ |
| Phase 1 | 骨格構築: Tauri 起動、Tag Bus、TOML ロード、PostgreSQL ドライバ PoC、MQTT Publisher、最低限の UI |
| Phase 2 | タグ管理 UI、ドライバ別タグ登録ツール、TOML インポート/エクスポート、ログビューア、Mosquitto 同梱インストーラ整備 |
| Phase 3 | JoyWatcher ドライバ、子プロセス IPC、ランタイム/登録 IPC の統一 |
| Phase 4 | SLMP ドライバ追加 |
| Phase 5+ | 外部ネットワーク対応時の暗号化・認証の追加検討 |

## Phase 1: 骨格構築

**目標**: MQTT 配信まで、最低限の UI で動作確認する。

### Rust バックエンド

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| プロジェクト初期化 | Cargo.toml 依存追加、基本モジュール構成作成 | 1-2日 |
| Core 層実装 | Tag, TagValue, Quality, DataType, Tag Bus, タグレジストリ | 2-3日 |
| Config 層実装 | TOML パーサ、`config/tags.toml`, `drivers.toml`, `publishers.toml` の読み込み | 2日 |
| PostgreSQL ドライバ PoC | 接続、タグ値の定期取得、Tag Bus へ publish | 4-5日 |
| MQTT Publisher 実装 | `rumqttc` との接続、Tag Bus 購読、MQTT 配信 | 3-4日 |
| Tauri Commands 層 | tag.rs, driver.rs, publisher.rs の実装 | 2-3日 |
| ロギング・監視 | `tracing-subscriber` の設定 | 1-2日 |
| テスト | タグレジストリ、Config 層、Tag Bus のテスト | 2-3日 |

### Svelte フロントエンド

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| フロント初期化 | Svelte 5 + Vite 基本構成 | 1日 |
| IPC ラッパ層 | `src/lib/ipc/` に invoke 関数定義 | 1-2日 |
| 3ペイン レイアウト | 左ツリー、中央リスト、右詳細 | 2-3日 |
| 状態管理 | tags, drivers, publishers, systemStatus | 1-2日 |
| ダッシュボード | 稼働状況サマリ、接続状態表示 | 2-3日 |
| ドライバ一覧画面 | 追加・削除 UI、簡易テーブル | 2-3日 |
| タグ一覧画面（簡易） | テーブル形式の簡易版 | 2-3日 |

### Phase 1 マイルストーン

- `npm run tauri dev` で起動可能。
- PostgreSQL から値を取得できる。
- MQTT にタグ値が流れる。
- Tauri UI で稼働状況・タグ一覧を確認できる。

## Phase 2: タグ管理 UI + Mosquitto 同梱

**目標**: UI からのタグ管理、TOML の import/export、Mosquitto をインストーラに統合する。

### 後端

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| Config ホットリロード | TOML 変更時の再ロード | 2-3日 |
| Tag 作成・更新・削除コマンド | TOML 編集・保存、mtime チェック | 2-3日 |
| 登録結果インポート | 一時 JSON の検証・取り込み | 2-3日 |
| ドライバ登録ツール起動 | 別ウィンドウ/別プロセス launcher | 2-3日 |
| Tauri Event | フロントへの変更通知 | 1日 |

### フロント

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| タグ管理テーブル UI | 行追加・削除・コピー・ペースト | 5-7日 |
| インポート・エクスポート | CSV/TOML ファイル選択、一括読込、export | 2-3日 |
| バリデーション表示 | テーブルセル内のエラー表示 | 2-3日 |
| PostgreSQL 登録ツール PoC | テーブル/カラム探索、タグ候補生成 | 4-6日 |
| ログビューア | `/logs` ページ、ログファイル読み込み表示 | 2-3日 |

### Mosquitto インストーラ統合

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| WiX / NSIS スクリプト | Mosquitto EXE サイレント実行、Windows サービス登録 | 3-4日 |
| サービス管理 UI | 状態表示、start/stop/restart ボタン | 2-3日 |
| `mosquitto.conf` UI | リスナーポート等の編集画面 | 2-3日 |
| インストーラテスト | 複数環境での MSI/NSIS 試験 | 2-3日 |

### Phase 2 マイルストーン

- UI から新しいタグを追加 → TOML に保存 → Rust 側が読み込み → MQTT に配信。
- PostgreSQL 登録ツールから複数タグを一括登録できる。
- Mosquitto が Windows サービスで自動起動し、UI から停止・再起動できる。

## Phase 3: JoyWatcher ドライバ + 子プロセス IPC

**目標**: Borland C++ サンプルを解析・移植し、JoyWatcher ドライバを子プロセス化する。

### 前提作業

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| Borland C++ サンプル解析 | API シーケンス・DLL 依存性確認 | 3-5日 |
| C ABI 設計 | 関数署名、構造体、FFI 境界設計 | 2-3日 |

### Rust 実装

| タスク | 詳細 | 見積 |
| -------- | ------ | ------ |
| JoyWatcher ドライバ実装 | C++ ロジック移植、同期 API は `spawn_blocking` | 5-7日 |
| FFI バインディング | `bindgen` / `libloading` による安全ラッパ | 2-3日 |
| 子プロセス IPC 層 | gRPC / Named Pipe の Driver service 定義 | 3-4日 |
| メインプロセス側クライアント | 接続管理、再接続、プロセス監視 | 2-3日 |
| 子プロセス launcher | EXE 起動、stdout 監視、クラッシュ時再起動 | 2-3日 |
| テスト・モック | IPC サーバのモック、結合テスト | 2-3日 |

### Phase 3 マイルストーン

- `driver-joywatcher.exe` が子プロセスとして起動可能。
- Main app から JoyWatcher ドライバ初期化 → タグ値取得 → MQTT 配信。
- JoyWatcher DLL クラッシュ時に main app はダウンしない。

## Phase 4: SLMP ドライバ追加

**目標**: Phase 3 の子プロセス + IPC パターンを SLMP に適用する。

- SLMP 仕様確認。
- 既存 Rust ライブラリ利用可否調査。
- SLMP ドライバ実装。
- シミュレータまたは実機によるテスト環境構築。

## Phase 5+: 外部ネットワーク対応

必要性が出た時点で、MQTT TLS + 認証、ドライバ間の通信暗号化を検討する。

## 技術検討事項

| 機能 | 候補 | 状態 |
| ------ | ------ | ------ |
| MQTT ブローカー | Mosquitto 公式バイナリ | PoC 予定 |
| gRPC | `tonic` | Phase 3 で検証 |
| テーブル UI | Tabulator / `svelte-headless-table` | Phase 2 で PoC |
| FFI | `bindgen` / `libloading` | Phase 3 で実装 |
| ファイル監視 | `notify` | Phase 2 で実装 |
| Windows サービス | `windows-service` or `sc.exe` 呼び出し | Phase 2 で実装 |

## リスク管理

| リスク | 影響度 | 対策 |
| -------- | -------- | ------ |
| JoyWatcher DLL の入手困難 | 高 | 早期確保、不可なら代替ドライバ検討 |
| TOML パース エラー ユーザ対応 | 中 | 詳細エラー表示、テンプレート提供 |
| IPC オーバーヘッド | 中 | 計測、バッチ処理を検討 |
| Windows サービス登録の権限問題 | 低 | インストーラで UAC 昇格 |
| Mosquitto ライセンス確認漏れ | 高 | 法務確認、ドキュメント作成 |

## 品質戦略

- タグ数 1000〜10000 で MQTT 配信レイテンシ、メモリ使用量を定期測定する。
- 各 Phase で panic 0 を目指す。
- コード・API・セットアップ手順を継続的に整備する。
- CI/CD は lint, test, build 自動化から開始する。

## マイルストーン定義

| マイルストーン | 達成条件 |
| -------- | -------- |
| Alpha | MQTT にタグ値が流れている、Tauri UI で稼働状況確認可能 |
| Beta | UI からタグ管理可能、Mosquitto インストーラ統合 |
| RC | JoyWatcher ドライバが動作、複数ドライバ並行運用 |
| v1.0 | SLMP ドライバ追加、基本的なドライバプラグイン構造確立 |
