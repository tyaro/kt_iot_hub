use super::{DriverIoSampleState, DriverIoTotalState};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct AppCpuSampleState {
    pub process_kernel_time: u64,
    pub process_user_time: u64,
    pub sampled_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeMetricsCacheState {
    pub last_network_rx_bytes: Option<u64>,
    pub last_network_tx_bytes: Option<u64>,
    pub last_sampled_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub last_driver_io_samples: HashMap<u32, DriverIoSampleState>,
    pub last_driver_reported_io_totals: HashMap<String, DriverIoTotalState>,
    pub last_driver_reported_io_samples: HashMap<String, DriverIoSampleState>,
    pub last_app_cpu_sample: Option<AppCpuSampleState>,
}
