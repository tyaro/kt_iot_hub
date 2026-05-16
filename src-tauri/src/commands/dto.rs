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
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
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
