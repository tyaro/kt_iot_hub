use serde::{Deserialize, Serialize};

/// ドライバレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct DriverDto {
    pub id: String,
    #[serde(rename = "driver_type")]
    pub driver_type: String,
    pub enabled: bool,
    #[serde(rename = "registration_ui_available")]
    pub registration_ui_available: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
}

/// ドライバ作成/更新リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct SaveDriverRequest {
    pub id: String,
    #[serde(rename = "original_id")]
    pub original_id: Option<String>,
    #[serde(rename = "driver_type")]
    pub driver_type: String,
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}
