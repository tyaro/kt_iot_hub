use serde::{Deserialize, Serialize};

/// タグレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagPayload {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub driver_id: String,
    pub scan_group_id: String,
    pub driver_spec: serde_json::Value,
}

pub type TagDto = TagPayload;

/// スキャングループ DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanGroupDto {
    pub id: String,
    pub driver_id: String,
    pub table: Option<String>,
    pub timestamp_column: Option<String>,
    pub scan_rate_ms: Option<u32>,
    pub observed_cycle_ms: Option<u64>,
    pub observed_p95_cycle_ms: Option<u64>,
    pub cycle_delta_ratio: Option<f64>,
    pub cycle_status: Option<String>,
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
pub type CreateTagRequest = TagPayload;
