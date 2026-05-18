#[derive(Clone, Debug, Default)]
pub struct MqttMonitorStatusState {
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

#[derive(Clone, Debug)]
pub struct MqttMonitorMessageState {
    pub timestamp: String,
    pub topic: String,
    pub payload: String,
    pub qos: u8,
    pub retain: bool,
}
