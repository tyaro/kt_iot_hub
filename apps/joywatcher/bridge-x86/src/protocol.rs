use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BridgeRequest {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "connect")]
    Connect {
        endpoint: Option<String>,
        user_id: Option<i32>,
        password: Option<String>,
    },
    #[serde(rename = "disconnect")]
    Disconnect,
    #[serde(rename = "forceDisconnect")]
    ForceDisconnect,
    #[serde(rename = "resolveTags")]
    ResolveTags { tags: Vec<String> },
    #[serde(rename = "browseTags")]
    BrowseTags,
    #[serde(rename = "read")]
    Read { request_id: String, tag_ids: Vec<i32> },
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum BridgeResponse {
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "connected")]
    Connected {
        active_connections: usize,
        mode: &'static str,
    },
    #[serde(rename = "disconnected")]
    Disconnected { active_connections: usize },
    #[serde(rename = "resolvedTags")]
    ResolvedTags { items: Vec<ResolvedTag> },
    #[serde(rename = "browsedTags")]
    BrowsedTags { items: Vec<String> },
    #[serde(rename = "readResult")]
    ReadResult {
        request_id: String,
        values: Vec<ReadValuePayload>,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTag {
    pub tag_path: String,
    pub tag_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadValuePayload {
    pub tag_id: i32,
    pub quality: String,
    pub value: MockValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MockValue {
    Bool(bool),
    Number(f64),
    String(String),
}

impl BridgeResponse {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}
