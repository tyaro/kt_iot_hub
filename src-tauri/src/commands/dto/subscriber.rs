use crate::app_state::{MqttMonitorMessageState, MqttMonitorStatusState};
use serde::{Deserialize, Serialize};

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
    pub include_sys: bool,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMonitorTopicNodeDto {
    pub id: String,
    pub label: String,
    pub full_path: String,
    pub has_children: bool,
    pub latest_message: Option<MqttMonitorMessageDto>,
    pub children: Vec<MqttMonitorTopicNodeDto>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartMqttMonitorRequest {
    pub publisher_id: String,
    pub topic_filter: String,
    pub include_sys: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMqttMonitorTreeRequest {
    pub expanded_paths: Vec<String>,
    pub include_all: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMqttMonitorTopicDetailRequest {
    pub full_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMonitorTopicDetailDto {
    pub full_path: String,
    pub latest_message: Option<MqttMonitorMessageDto>,
}

impl From<MqttMonitorStatusState> for MqttMonitorStatusDto {
    fn from(value: MqttMonitorStatusState) -> Self {
        Self {
            connected: value.connected,
            subscribing: value.subscribing,
            include_sys: value.include_sys,
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
