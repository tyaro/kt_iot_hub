use serde::{Deserialize, Serialize};

/// アプリメトリクス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetricsDto {
    pub process_cpu_percent: Option<f32>,
    pub process_memory_bytes: Option<u64>,
    pub system_cpu_percent: Option<f32>,
    pub system_memory_used_bytes: Option<u64>,
    pub system_memory_total_bytes: Option<u64>,
    pub network_rx_bytes_per_sec: Option<f64>,
    pub network_tx_bytes_per_sec: Option<f64>,
    pub sampled_at: String,
}

/// ドライバ別メトリクス DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverMetricsDto {
    pub driver_id: String,
    pub driver_type: String,
    pub pid: u32,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub network_rx_bytes_per_sec: Option<f64>,
    pub network_tx_bytes_per_sec: Option<f64>,
}
