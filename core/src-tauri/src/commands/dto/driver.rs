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
    pub password_key: Option<String>,
    #[serde(default)]
    pub tls_enabled: bool,
    pub tls_ca_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub tls_client_key_path: Option<String>,
    pub connect_timeout_ms: Option<u64>,
    pub statement_timeout_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub auto_restart: bool,
    pub max_restart_per_minute: Option<u32>,
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
    #[serde(default, alias = "password_key")]
    pub password_key: Option<String>,
    #[serde(default, alias = "tls_enabled")]
    pub tls_enabled: bool,
    #[serde(default, alias = "tls_ca_path")]
    pub tls_ca_path: Option<String>,
    #[serde(default, alias = "tls_client_cert_path")]
    pub tls_client_cert_path: Option<String>,
    #[serde(default, alias = "tls_client_key_path")]
    pub tls_client_key_path: Option<String>,
    #[serde(default, alias = "connect_timeout_ms")]
    pub connect_timeout_ms: Option<u64>,
    #[serde(default, alias = "statement_timeout_ms")]
    pub statement_timeout_ms: Option<u64>,
    #[serde(default = "default_true", alias = "auto_restart")]
    pub auto_restart: bool,
    #[serde(default, alias = "max_restart_per_minute")]
    pub max_restart_per_minute: Option<u32>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde")]
pub struct ExportTagManagementSettingsRequest {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct ExportTagManagementSettingsResponse {
    pub path: String,
    #[serde(rename = "driver_count")]
    pub driver_count: usize,
    #[serde(rename = "scan_group_count")]
    pub scan_group_count: usize,
    #[serde(rename = "tag_count")]
    pub tag_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde")]
pub struct ImportTagManagementSettingsRequest {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct ImportTagManagementSettingsResponse {
    pub path: String,
    #[serde(rename = "driver_count")]
    pub driver_count: usize,
    #[serde(rename = "scan_group_count")]
    pub scan_group_count: usize,
    #[serde(rename = "tag_count")]
    pub tag_count: usize,
}
