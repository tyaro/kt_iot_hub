use serde::{Deserialize, Serialize};

/// ランタイム状態 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusDto {
    pub drivers_running: bool,
    pub publishers_running: bool,
    pub grpc_running: bool,
    pub last_error: Option<String>,
}

/// ランタイム起動リクエスト DTO
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRuntimeServicesRequest {
    pub driver_ui_base_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStartupConfigDto {
    pub auto_start_runtime_services: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRuntimeStartupConfigRequest {
    pub auto_start_runtime_services: bool,
}
