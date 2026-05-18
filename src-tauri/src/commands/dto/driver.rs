use serde::{Deserialize, Serialize};

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
