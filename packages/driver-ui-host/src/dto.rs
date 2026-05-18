//! 本体・ドライバUI で共通利用する DTO 群。
//! すべて `camelCase` 直列化で UI 側と整合する。

use serde::{Deserialize, Serialize};

/// PostgreSQL 接続パラメータ DTO
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresConnectionParams {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub ssl_mode: Option<String>,
}

/// PostgreSQL 接続テスト結果 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresConnectionTestResult {
    pub ok: bool,
    pub message: String,
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

/// ドライバUI起動コンテキスト DTO (UI へ返却)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editing_tag_id: Option<String>,
}

/// ドライバUI 出力 JSON 保存リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDriverUiOutputRequest {
    pub output_json_path: Option<String>,
    pub payload: serde_json::Value,
}

/// 共通エラーレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

pub mod error_code {
    pub const ALREADY_EXISTS: &str = "ALREADY_EXISTS";
    pub const ALREADY_RUNNING: &str = "ALREADY_RUNNING";
    pub const DUPLICATE_IMPORT: &str = "DUPLICATE_IMPORT";
    pub const GRPC_BIND_FAILED: &str = "GRPC_BIND_FAILED";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const INVALID_JSON: &str = "INVALID_JSON";
    pub const IO_ERROR: &str = "IO_ERROR";
    pub const METRICS_PROCESS_MEMORY_ERROR: &str = "METRICS_PROCESS_MEMORY_ERROR";
    pub const METRICS_PROCESS_TIMES_ERROR: &str = "METRICS_PROCESS_TIMES_ERROR";
    pub const NOT_CONFIGURED: &str = "NOT_CONFIGURED";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const NO_ACTIVE_SESSION: &str = "NO_ACTIVE_SESSION";
    pub const PROCESS_LAUNCH_FAILED: &str = "PROCESS_LAUNCH_FAILED";
    pub const RESULT_NOT_READY: &str = "RESULT_NOT_READY";
    pub const SCHEMA_MISMATCH: &str = "SCHEMA_MISMATCH";
    pub const SERIALIZE_ERROR: &str = "SERIALIZE_ERROR";
    pub const SESSION_ACTIVE: &str = "SESSION_ACTIVE";
    pub const SESSION_MISMATCH: &str = "SESSION_MISMATCH";
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const WINDOW_OPEN_FAILED: &str = "WINDOW_OPEN_FAILED";
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }

    pub fn invalid_input(error: impl Into<String>) -> Self {
        Self::new(error_code::INVALID_INPUT, error)
    }

    pub fn not_found(error: impl Into<String>) -> Self {
        Self::new(error_code::NOT_FOUND, error)
    }

    pub fn io_error(error: impl Into<String>) -> Self {
        Self::new(error_code::IO_ERROR, error)
    }

    pub fn serialize_error(error: impl Into<String>) -> Self {
        Self::new(error_code::SERIALIZE_ERROR, error)
    }

    pub fn validation_error(error: impl Into<String>) -> Self {
        Self::new(error_code::VALIDATION_ERROR, error)
    }
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(err: anyhow::Error) -> Self {
        Self::new(error_code::INTERNAL_ERROR, err.to_string())
    }
}
