use chrono::{DateTime, Utc};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct ScanGroupRuntimeMetricState {
    pub expected_scan_rate_ms: Option<u32>,
    pub last_cycle_anchor_at: Option<DateTime<Utc>>,
    pub last_cycle_ms: Option<u64>,
    pub avg_cycle_ms: Option<f64>,
    pub p95_cycle_ms: Option<u64>,
    pub cycle_history_ms: VecDeque<u64>,
    pub cycle_delta_ratio: Option<f64>,
    pub consecutive_lag_count: u32,
    pub last_warn_at: Option<DateTime<Utc>>,
}

impl ScanGroupRuntimeMetricState {
    pub fn new(expected_scan_rate_ms: Option<u32>) -> Self {
        Self {
            expected_scan_rate_ms,
            last_cycle_anchor_at: None,
            last_cycle_ms: None,
            avg_cycle_ms: None,
            p95_cycle_ms: None,
            cycle_history_ms: VecDeque::with_capacity(32),
            cycle_delta_ratio: None,
            consecutive_lag_count: 0,
            last_warn_at: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DriverIoSampleState {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub sampled_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct DriverIoTotalState {
    pub rx_bytes_total: u64,
    pub tx_bytes_total: u64,
    pub sampled_at: DateTime<Utc>,
}
