# アーキテクチャ設計

## 全体構成

```text
┌────────────────────────────────────────────────────────────────┐
│                    Tauri App (kt_iot_hub)                       │
│                                                                 │
│  ┌──────────────────────┐        ┌───────────────────────────┐ │
│  │ Frontend (Svelte)    │ <----> │ Tauri Commands / Events   │ │
│  │  - 3ペイン管理画面    │  IPC   │  (薄い橋渡し層)            │ │
│  └──────────────────────┘        └─────────────┬─────────────┘ │
│                                                 │               │
│                                ┌────────────────▼────────────┐  │
│                                │  Core (Rust)                │  │
│                                │  ┌──────────────────────┐   │  │
│                                │  │ Tag Bus (broadcast)  │   │  │
│                                │  └─────┬───────────┬────┘   │  │
│                                │        │           │        │  │
│                                │  ┌─────▼────┐ ┌────▼─────┐  │  │
│                                │  │ Drivers  │ │ Publish. │  │  │
│                                │  │ Manager  │ │ Manager  │  │  │
│                                │  └─┬──┬──┬──┘ └──┬───┬───┘  │  │
│                                └────┼──┼──┼───────┼───┼──────┘  │
└──────────────────────────────────────┼──┼──┼───────┼───┼─────────┘
                                       │  │  │       │   │
              ┌────────────────────────┘  │  └──┐    │   └──┐
              ▼                           ▼     ▼    ▼      ▼
        PostgreSQL                  JoyWatcher  ... MQTT   (OPC UA/DA 将来)
        (DB ドライバ)               (SCADA)         Broker
```

## 設計原則

- **疎結合**: ドライバ、パブリッシャ、UI は Tag Bus または IPC 境界を介して通信する。
- **拡張容易性**: ドライバは独立プロセスとして追加可能にする。
- **登録UIの分離**: ドライバ固有のタグ登録 UI は本体 UI に差し込まず、別ウィンドウ/別プロセスとして実装する。
- **役割分離**: バックエンドは責務ごとにファイル分割し、原則 300 行以内/ファイルを目指す。
- **フロントは表示と入力に専念**: ビジネスロジックは Rust 側に置く。

## ドライバ登録プロセス構成

タグ登録方法は PostgreSQL / SLMP / JoyWatcher で大きく異なるため、登録 UI は本体 UI に埋め込まない。
本体は「タグ定義の正本管理」と「共通項目の検証・保存」を担い、ドライバ登録プロセスは「接続先探索」と「タグ候補生成」を担う。

```shell
┌──────────────────────────────┐
│ kt_iot_hub.exe                │
│ - 3ペイン管理画面             │
│ - Tag Registry                │
│ - config/tags.toml 保存       │
│ - Driver Process Manager      │
└──────────────┬───────────────┘
               │ 起動 / 結果取込
               ▼
┌──────────────────────────────┐
│ driver-postgres.exe register  │
│ - PostgreSQL接続設定          │
│ - テーブル/カラム探索         │
│ - タグ候補生成                │
└──────────────┬───────────────┘
               │ JSON / IPC
               ▼
        タグ定義候補を本体へ返却
```

初期実装では、一時 JSON ファイル経由でタグ定義候補を受け渡す。将来的には gRPC / Named Pipe に統一する。

## ドメインモデル

### Tag

```rust
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub data_type: DataType,
    pub driver_id: DriverId,
    pub driver_spec: serde_json::Value,
    pub metadata: TagMetadata,
}

pub struct TagValue {
    pub tag_id: TagId,
    pub value: Value,
    pub quality: Quality,
    pub timestamp: DateTime<Utc>,
}
```

### Driver trait

```rust
#[async_trait]
pub trait Driver: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn start(&mut self, ctx: DriverCtx) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn register_tag(&mut self, spec: TagSpec) -> Result<TagId>;
    async fn unregister_tag(&mut self, id: TagId) -> Result<()>;
}
```

### Publisher trait

```rust
#[async_trait]
pub trait Publisher: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn start(&mut self, rx: TagValueReceiver) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
}
```

## Tag Bus

- `tokio::sync::broadcast::Sender<TagValue>` を中心とした内部イベントバス。
- Driver は送信、Publisher / UI は購読。
- 1対多のファンアウトと疎結合を実現する。

## データフロー

1. 起動時に TOML ファイルからタグ定義・ドライバ設定・パブリッシャ設定をロードする。
2. DriverManager が各ドライバを起動する。
3. ドライバが周期またはイベントで値を取得し Tag Bus に publish する。
4. PublisherManager 配下の各 Publisher が購読し、MQTT 等へ送出する。
5. UI は Tauri Event または購読型 store で最新値を表示する。

## ディレクトリ構成方針

```shell
src-tauri/src/
├─ main.rs
├─ app_state.rs
├─ commands/
├─ config/
├─ core/
├─ drivers/
├─ publishers/
└─ telemetry/

src/
├─ App.svelte
├─ main.ts
└─ lib/
   ├─ components/
   ├─ ipc/
   └─ stores/
```
