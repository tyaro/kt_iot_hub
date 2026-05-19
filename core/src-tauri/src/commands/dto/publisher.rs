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
}
