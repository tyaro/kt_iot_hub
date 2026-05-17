# MQTT モニタ機能設計

## 目的

本体が MQTT ブローカーへ publish した topic / payload を、ユーザーが本体 UI 上で直接確認できるようにする。

本機能は以下を満たすことを目的とする。

- broker に実際に流れた topic / payload を確認できる
- topic 設定や QoS 設定が意図どおりか即座に検証できる
- 現場オペレータがダッシュボードから短手数で状態確認できる
- MQTT Explorer 等の外部ツールがなくても最低限の確認ができる

## 方針

- MQTT Subscriber は **本体の基本機能** として実装する
- ただし責務は Publisher と分離し、`subscribers/` モジュールを新設する
- 監視対象は Tag Bus ではなく **MQTT ブローカー** とする
- UI 導線は `ログ` ページではなく **ダッシュボードから展開するモニタパネル** を採用する

## 別プロセスにしない理由

今回は汎用の外部監視ツールではなく、IoT Hub の運用確認機能が目的である。
そのため以下の利点が大きい。

- 既存の publisher 設定（broker / port / username / password / topic）を再利用できる
- ダッシュボードの runtime 状態と同じ画面で確認できる
- 「配信設定」と「実際に broker に出た内容」を並べて確認できる
- 外部ツール導入なしで現場確認できる

一方で、大量保存・全文検索・長期監査・複数 broker 常時監視のような要件は初期スコープに含めない。
これらが必要になった場合のみ別プロセス化を再検討する。

## スコープ

### 初期スコープ

- 1 つの MQTT モニタセッションを本体内で管理する
- 選択した publisher 設定を初期値として broker へ subscribe する
- topic filter を指定できる（例: `plant/#`）
- 受信メッセージの最新 N 件を表示する
- 表示項目は timestamp / topic / payload / retain / qos とする
- 接続開始 / 停止 / クリアを提供する
- ダッシュボードから展開表示できる

### 初期スコープ外

- DB 永続化
- 全文検索
- エクスポート
- 複数 broker 同時接続
- 受信 payload の JSON ツリー表示
- topic 単位の統計集計

## UI 設計

### 導線

ダッシュボードの「ドライバ / MQTT 状態」カード付近に、MQTT モニタの展開ボタンを配置する。

例:

- `MQTT モニタを開く`
- 展開時はダッシュボード内の下部パネルとして表示

### パネル構成

#### 上部操作行

- publisher 選択
- topic filter 入力
- 接続開始ボタン
- 停止ボタン
- クリアボタン
- 自動スクロール ON/OFF

#### 状態表示

- 接続状態: `未接続 / 接続中 / 購読中 / エラー`
- 現在の broker / port
- 現在の topic filter
- 受信件数
- 最終受信時刻
- 直近エラー

#### メッセージ一覧

列:

- 受信時刻
- topic
- payload
- qos
- retain

新着が下に追加されるシンプルなログビュー形式とする。

## バックエンド設計

## モジュール構成

```text
src-tauri/src/
├─ subscribers/
│  ├─ mod.rs
│  └─ mqtt_monitor.rs
├─ commands/
│  └─ subscriber/
│     ├─ mod.rs
│     └─ monitor.rs
```

## 役割分担

### `subscribers/mqtt_monitor.rs`

- rumqttc で MQTT broker へ接続
- 指定 topic filter を subscribe
- 受信メッセージをリングバッファへ格納
- start / stop / status / snapshot を提供

### `commands/subscriber/monitor.rs`

- Tauri command を薄く提供する
- UI 入力のバリデーション
- AppState 上の monitor state へ橋渡しする

## AppState 追加項目

- `mqtt_monitor`: 単一モニタセッションの状態
- `mqtt_monitor_messages`: 直近メッセージのリングバッファ

## データ構造

```rust
pub struct MqttMonitorMessage {
    pub timestamp: String,
    pub topic: String,
    pub payload: String,
    pub qos: u8,
    pub retain: bool,
}

pub struct MqttMonitorStatus {
    pub connected: bool,
    pub subscribing: bool,
    pub publisher_id: Option<String>,
    pub broker: String,
    pub port: u16,
    pub topic_filter: String,
    pub message_count: usize,
    pub last_message_at: Option<String>,
    pub last_error: Option<String>,
}
```

## IPC 設計

### コマンド

- `get_mqtt_monitor_status`
- `list_mqtt_monitor_publishers`
- `start_mqtt_monitor`
- `stop_mqtt_monitor`
- `clear_mqtt_monitor_messages`
- `list_mqtt_monitor_messages`

### start リクエスト

```ts
{
  publisherId: string;
  topicFilter: string;
}
```

### 表示更新方式

初期実装は **ポーリング** とする。

- status: 1〜2 秒間隔
- messages: 1〜2 秒間隔

理由:

- 既存 UI が runtime status をポーリングしている
- 実装が単純
- 初期スコープでは十分

将来的に件数が増えた場合のみ Tauri event push へ移行する。

## publisher 設定との関係

モニタ接続先は publisher 設定を基準とする。

- broker
- port
- username
- password

topic filter は monitor 側で指定する。
初期値は publisher の `topic` をもとに次のように生成する。

- `topic = plant` → `plant/#`
- 空文字 → `#`

## エラー処理

- 接続失敗時は monitor status の `last_error` に保持する
- subscribe 失敗時は接続を停止し、UI に失敗メッセージを返す
- stop はべき等とする
- 再接続は初期スコープでは自動化せず、ユーザーの再実行に委ねる

## 性能と制限

- メッセージ保持件数は初期 200 件
- payload は文字列化して保持する
- 1 message あたりの payload 長さが大きすぎる場合は UI 側で折りたたむ
- 受信が高頻度でも UI が固まらないよう、バックエンドはリングバッファで古い項目を破棄する

## 実装ステップ

1. Rust 側に subscriber モジュールと monitor state を追加
2. monitor 用 Tauri command を追加
3. ダッシュボードに展開パネルを追加
4. publisher 選択・topic filter・受信一覧を実装
5. `plant/#` で実 broker の表示確認を行う

## 受け入れ条件

- ダッシュボードから MQTT モニタを開ける
- publisher 設定を選んで monitor 開始できる
- `plant/#` を購読すると実配信 topic が一覧に表示される
- 停止ボタンで monitor が停止する
- ドライバ / MQTT runtime の停止と monitor 停止が競合せず動作する
