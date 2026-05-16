// データ転送オブジェクト (DTO)
// UI と Rust バックエンド間の型安全な IPC

use serde::{Deserialize, Serialize};

/// タグレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub driver_id: String,
    pub scan_group_id: String,
    pub driver_spec: serde_json::Value,
}

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
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagRequest {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub driver_id: String,
    pub scan_group_id: String,
    pub driver_spec: serde_json::Value,
}

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

/// ドライバUI起動リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDriverUiRequest {
    pub driver_id: Option<String>,
    pub driver_type: Option<String>,
}

/// ドライバ作成/更新リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde")]
pub struct SaveDriverRequest {
    pub id: String,
    pub driver_type: String,
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
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
    pub output_json_path: String,
}

/// ドライバUI結果ファイル存在確認レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDriverUiResultResponse {
    pub ready: bool,
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

/// PostgreSQL 接続パラメータ DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresConnectionParams {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub ssl_mode: Option<String>,
}

/// PostgreSQL テーブル一覧 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresTableDto {
    pub schema: String,
    pub name: String,
}

/// PostgreSQL カラム一覧 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresColumnDto {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

/// PostgreSQL カラム取得リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresColumnsRequest {
    pub conn: PostgresConnectionParams,
    pub schema: Option<String>,
    pub table: String,
}

/// PostgreSQL 接続テスト結果 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresConnectionTestResult {
    pub ok: bool,
    pub message: String,
}

/// ドライバUI起動コンテキスト DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUiLaunchContextDto {
    pub launched_as_driver_ui: bool,
    pub session_id: Option<String>,
    pub driver_type: Option<String>,
    pub driver_id: Option<String>,
    pub input_json_path: Option<String>,
    pub output_json_path: Option<String>,
    pub request_id: Option<String>,
}

/// ドライバUI出力JSON保存リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDriverUiOutputRequest {
    pub output_json_path: Option<String>,
    pub payload: serde_json::Value,
}

/// エラーレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(err: anyhow::Error) -> Self {
        Self {
            error: err.to_string(),
            code: "INTERNAL_ERROR".to_string(),
        }
    }
}
