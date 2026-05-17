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
│                                │  │ Driver   │ │ Publish. │  │  │
│                                │  │ Process  │ │ Manager  │  │  │
│                                │  │ Manager  │ │          │  │  │
│                                │  └─────┬────┘ └──┬───┬───┘  │  │
│                                └────────┼─────────┼───┼──────┘  │
└──────────────────────────────────────┼──┼──┼───────┼───┼─────────┘
                           │              │   │
          ┌────────────────────────┘              │   └──┐
          ▼                                       ▼      ▼
    driver-postgres.exe                        MQTT Publisher
    (別プロセス, gRPC client)                  (本体内)
```

## 設計原則

- **疎結合**: ドライバ、パブリッシャ、UI は Tag Bus または IPC 境界を介して通信する。
- **拡張容易性**: ドライバ通信処理は独立プロセスとして追加可能にする。
- **登録UIの分離**: ドライバ固有のタグ登録 UI は本体 UI に差し込まず、別ウィンドウ/別プロセスとして実装する。
- **役割分離**: バックエンドは責務ごとにファイル分割し、原則 300 行以内/ファイルを目指す。
- **フロントは表示と入力に専念**: ビジネスロジックは Rust 側に置く。

## ドライバ登録・通信プロセス構成

タグ登録方法は PostgreSQL / SLMP / JoyWatcher で大きく異なるため、登録 UI は本体 UI に埋め込まない。
本体は「タグ定義の正本管理」「MQTT配信」「ドライバプロセス管理」を担い、
ドライバ側は「登録UIプロセス」と「通信ランタイムプロセス」に責務分離する。

```shell
┌──────────────────────────────┐
│ kt_iot_hub.exe                │
│ - 3ペイン管理画面             │
│ - Tag Registry                │
│ - config/tags.toml 保存       │
│ - Driver Process Manager      │
│ - gRPC Server (:55051)        │
│ - MQTT Publisher              │
└──────────────┬───────────────┘
           │ 起動 / 結果取込 / IPC
               ▼
┌──────────────────────────────┐
│ apps/postgres/ui (Tauri)      │
│ - PostgreSQL接続設定          │
│ - テーブル/カラム探索         │
│ - タグ候補生成                │
└──────────────┬───────────────┘
           │ gRPC (TagRegistrationService)
           ▼
    タグ定義候補を本体へ反映

┌──────────────────────────────┐
│ apps/postgres/driver (Rust)  │
│ - PostgreSQLポーリング        │
│ - 値をgRPCストリーム送信      │
└──────────────┬───────────────┘
           │ gRPC (DriverRuntimeService)
               ▼
     本体Tag Busへ値を反映
```

gRPC は localhost のみで待ち受け、登録UIと通信ランタイムの双方が本体へ接続する。

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

### Driver runtime IPC

- `TagRegistrationService`: 登録UIプロセス → 本体
- `UpsertTagRegistration`: スキャングループとタグ定義の一括登録
- `DriverRuntimeService`: 通信ランタイムプロセス ↔ 本体
- `GetDriverDefinition`: ドライバ起動時に接続設定・タグ定義を取得
- `StreamTagValues`: タグ値をストリーム送信

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
2. DriverProcessManager が `driver-{type}` 実行ファイルを子プロセス起動する。
3. ドライバプロセスが `GetDriverDefinition` で定義を取得し、外部機器/DBをポーリングする。
4. ドライバプロセスが `StreamTagValues` で値を本体へ送信し、本体が Tag Bus に publish する。
5. PublisherManager 配下の各 Publisher が購読し、MQTT 等へ送出する。
6. UI は Tauri Event または購読型 store で最新値を表示する。

## ディレクトリ構成方針

```shell
src-tauri/src/
├─ main.rs
├─ app_state.rs
├─ commands/
├─ config/
├─ core/
├─ drivers/
├─ grpc/
├─ publishers/
└─ telemetry/

apps/
└─ postgres/
    ├─ ui/      # 登録UI (Tauri)
    └─ driver/  # 通信ランタイム (Rust binary)

src/
├─ App.svelte
├─ main.ts
└─ lib/
   ├─ components/
   ├─ ipc/
   └─ stores/
```
