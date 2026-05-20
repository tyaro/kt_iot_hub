use serde::{Deserialize, Serialize};

/// パブリッシャレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct PublisherDto {
    pub id: String,
    #[serde(rename = "publisher_type")]
    pub publisher_type: String,
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub username: String,
    #[serde(rename = "client_id")]
    pub client_id: String,
    pub qos: u8,
    pub retain: bool,
    pub topic: String,
    #[serde(default)]
    pub publish_mode_default: String,
    #[serde(default)]
    pub publish_mode_by_driver: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub publish_mode_by_scan_group: std::collections::HashMap<String, String>,
}

/// パブリッシャ作成/更新リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct SavePublisherRequest {
    pub id: String,
    #[serde(rename = "publisher_type")]
    pub publisher_type: String,
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(rename = "client_id")]
    pub client_id: String,
    pub qos: u8,
    pub retain: bool,
    pub topic: String,
    #[serde(default)]
    pub publish_mode_default: Option<String>,
    #[serde(default)]
    pub publish_mode_by_driver: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub publish_mode_by_scan_group: Option<std::collections::HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct GetMqttPublishModeRequest {
    #[serde(rename = "driver_id", alias = "driverId")]
    pub driver_id: String,
    #[serde(rename = "scan_group_id", alias = "scanGroupId")]
    pub scan_group_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct GetMqttPublishModeResponse {
    pub publisher_id: String,
    pub mode: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct SetMqttPublishModeRequest {
    #[serde(rename = "publisher_id", alias = "publisherId")]
    pub publisher_id: Option<String>,
    pub scope: String,
    #[serde(rename = "driver_id", alias = "driverId")]
    pub driver_id: String,
    #[serde(rename = "scan_group_id", alias = "scanGroupId")]
    pub scan_group_id: Option<String>,
    pub mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "camelCase")]
pub struct SetMqttPublishModeResponse {
    pub publisher_id: String,
    pub scope: String,
    pub driver_id: String,
    pub scan_group_id: Option<String>,
    pub mode: String,
}
