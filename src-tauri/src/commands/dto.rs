// データ転送オブジェクト (DTO)
// UI と Rust バックエンド間の型安全な IPC

use serde::{Deserialize, Serialize};
use crate::app_state::{MqttMonitorMessageState, MqttMonitorStatusState};

/// タグレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagPayload {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub driver_id: String,
    pub scan_group_id: String,
    pub driver_spec: serde_json::Value,
}

pub type TagDto = TagPayload;

/// スキャングループ DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanGroupDto {
    pub id: String,
    pub driver_id: String,
    pub table: Option<String>,
    pub timestamp_column: Option<String>,
    pub scan_rate_ms: Option<u32>,
}

/// タグ値レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct TagValueDto {
    pub tag_id: String,
    pub value: serde_json::Value,
    pub quality: String,
    pub timestamp: String,
}

/// タグ作成リクエスト DTO
pub type CreateTagRequest = TagPayload;

/// ドライバレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DriverDto {
    pub id: String,
    pub driver_type: String,
    pub enabled: bool,
    pub registration_ui_available: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
}

/// パブリッシャレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct PublisherDto {
    pub id: String,
    pub publisher_type: String,
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub username: String,
    pub client_id: String,
    pub qos: u8,
    pub retain: bool,
    pub topic: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMonitorPublisherDto {
    pub id: String,
    pub broker: String,
    pub port: u16,
    pub topic: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMonitorStatusDto {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMonitorMessageDto {
    pub timestamp: String,
    pub topic: String,
    pub payload: String,
    pub qos: u8,
    pub retain: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartMqttMonitorRequest {
    pub publisher_id: String,
    pub topic_filter: String,
}

/// ドライバUI起動リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDriverUiRequest {
    pub driver_id: Option<String>,
    pub driver_type: Option<String>,
    pub driver_ui_base_dir: Option<String>,
    pub editing_tag_id: Option<String>,
}

/// ドライバ作成/更新リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde")]
pub struct SaveDriverRequest {
    pub id: String,
    pub original_id: Option<String>,
    pub driver_type: String,
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

/// パブリッシャ作成/更新リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde")]
pub struct SavePublisherRequest {
    pub id: String,
    pub publisher_type: String,
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub qos: u8,
    pub retain: bool,
    pub topic: String,
}

/// ドライバUI起動レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct LaunchDriverUiResponse {
    pub session_id: String,
    pub output_json_path: String,
    pub driver_id: Option<String>,
    pub driver_type: String,
}

/// ドライバUI結果ファイル存在確認リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDriverUiResultRequest {
    pub session_id: Option<String>,
    pub output_json_path: String,
}

/// ドライバUI結果ファイル存在確認レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDriverUiResultResponse {
    pub ready: bool,
    pub process_active: bool,
}

/// ドライバUI結果取り込みリクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDriverUiResultRequest {
    pub session_id: String,
    pub driver_id: Option<String>,
    pub output_json_path: String,
}

/// ドライバUI結果取り込みレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDriverUiResultResponse {
    pub driver_id: String,
    pub session_id: String,
    pub imported_tag_count: usize,
    pub imported_scan_group_count: usize,
}

/// ランタイム状態 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusDto {
    pub drivers_running: bool,
    pub publishers_running: bool,
    pub grpc_running: bool,
    pub last_error: Option<String>,
}

/// ランタイム起動リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRuntimeServicesRequest {
    pub driver_ui_base_dir: Option<String>,
}

impl From<MqttMonitorStatusState> for MqttMonitorStatusDto {
    fn from(value: MqttMonitorStatusState) -> Self {
        Self {
            connected: value.connected,
            subscribing: value.subscribing,
            publisher_id: value.publisher_id,
            broker: value.broker,
            port: value.port,
            topic_filter: value.topic_filter,
            message_count: value.message_count,
            last_message_at: value.last_message_at,
            last_error: value.last_error,
        }
    }
}

impl From<MqttMonitorMessageState> for MqttMonitorMessageDto {
    fn from(value: MqttMonitorMessageState) -> Self {
        Self {
            timestamp: value.timestamp,
            topic: value.topic,
            payload: value.payload,
            qos: value.qos,
            retain: value.retain,
        }
    }
}

/// `ErrorResponse` は本体とドライバUI 双方で利用する共通エラー型。
/// その他の PostgreSQL DTO 群はドライバUI 専用のため、ここでは re-export しない
/// （必要な場合は `kt_driver_ui_host::dto` から直接 import すること）。
pub use kt_driver_ui_host::dto::ErrorResponse;
