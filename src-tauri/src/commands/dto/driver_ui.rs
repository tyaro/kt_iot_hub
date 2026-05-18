use serde::{Deserialize, Serialize};

/// ドライバUI起動リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDriverUiRequest {
    pub driver_id: Option<String>,
    pub driver_type: Option<String>,
    pub driver_ui_base_dir: Option<String>,
    pub editing_tag_id: Option<String>,
}

/// ドライバUI起動レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct LaunchDriverUiResponse {
    pub session_id: String,
    pub output_json_path: String,
    pub driver_id: Option<String>,
    pub driver_type: String,
}

/// ドライバUI結果ファイル存在確認リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDriverUiResultRequest {
    pub session_id: Option<String>,
    pub output_json_path: String,
}

/// ドライバUI結果ファイル存在確認レスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDriverUiResultResponse {
    pub ready: bool,
    pub process_active: bool,
}

/// ドライバUI結果取り込みリクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDriverUiResultRequest {
    pub session_id: String,
    pub driver_id: Option<String>,
    pub output_json_path: String,
}

/// ドライバUI結果取り込みレスポンス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDriverUiResultResponse {
    pub driver_id: String,
    pub session_id: String,
    pub imported_tag_count: usize,
    pub imported_scan_group_count: usize,
}
