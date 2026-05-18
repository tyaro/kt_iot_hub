use serde::{Deserialize, Serialize};

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
