use crate::config::{DriverConfig, PublisherConfig, ScanGroupConfig};
use crate::core::{TagBus, TagRegistry};
use crate::drivers::DriverProcessManager;
use crate::publishers::PublisherManager;
use crate::subscribers::mqtt_monitor::MqttMonitor;
use std::collections::{HashMap, HashSet, VecDeque};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct DriverUiSessionState {
    pub target_driver_id: Option<String>,
    pub driver_type: String,
    pub process_active: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeStatusState {
    pub drivers_running: bool,
    pub publishers_running: bool,
    pub grpc_running: bool,
    pub last_error: Option<String>,
}

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
    pub last_driver_io_samples: HashMap<u32, DriverIoSampleState>,
    pub last_app_cpu_sample: Option<AppCpuSampleState>,
}

/// アプリ全体で共有する状態
#[derive(Clone)]
pub struct AppState {
    pub registry: TagRegistry,
    pub tag_bus: TagBus,
    pub drivers: std::sync::Arc<tokio::sync::RwLock<DriverProcessManager>>,
    pub publishers: std::sync::Arc<tokio::sync::RwLock<PublisherManager>>,
    pub driver_configs: std::sync::Arc<tokio::sync::RwLock<Vec<DriverConfig>>>,
    pub publisher_configs: std::sync::Arc<tokio::sync::RwLock<Vec<PublisherConfig>>>,
    pub scan_groups: std::sync::Arc<tokio::sync::RwLock<Vec<ScanGroupConfig>>>,
    /// session_id -> driver ui session context
    pub active_driver_ui_sessions:
        std::sync::Arc<tokio::sync::RwLock<HashMap<String, DriverUiSessionState>>>,
    /// 重複取込防止用 session_id 集合
    pub imported_driver_ui_sessions: std::sync::Arc<tokio::sync::RwLock<HashSet<String>>>,
    pub driver_ui_base_dir: std::sync::Arc<tokio::sync::RwLock<Option<String>>>,
    pub grpc_shutdown_tx:
        std::sync::Arc<tokio::sync::RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub runtime_status: std::sync::Arc<tokio::sync::RwLock<RuntimeStatusState>>,
    pub mqtt_monitor: std::sync::Arc<tokio::sync::Mutex<MqttMonitor>>,
    pub mqtt_monitor_status: std::sync::Arc<tokio::sync::RwLock<MqttMonitorStatusState>>,
    pub mqtt_monitor_messages:
        std::sync::Arc<tokio::sync::RwLock<VecDeque<MqttMonitorMessageState>>>,
    pub mqtt_monitor_topics:
        std::sync::Arc<tokio::sync::RwLock<HashMap<String, MqttMonitorMessageState>>>,
    pub scan_group_runtime_metrics:
        std::sync::Arc<tokio::sync::RwLock<HashMap<String, ScanGroupRuntimeMetricState>>>,
    pub runtime_metrics_cache:
        std::sync::Arc<tokio::sync::RwLock<RuntimeMetricsCacheState>>,
}

impl AppState {
    pub fn new(
        registry: TagRegistry,
        tag_bus: TagBus,
        drivers: DriverProcessManager,
        publishers: PublisherManager,
        driver_configs: Vec<DriverConfig>,
        publisher_configs: Vec<PublisherConfig>,
        scan_groups: Vec<ScanGroupConfig>,
    ) -> Self {
        Self {
            registry,
            tag_bus,
            drivers: std::sync::Arc::new(tokio::sync::RwLock::new(drivers)),
            publishers: std::sync::Arc::new(tokio::sync::RwLock::new(publishers)),
            driver_configs: std::sync::Arc::new(tokio::sync::RwLock::new(driver_configs)),
            publisher_configs: std::sync::Arc::new(tokio::sync::RwLock::new(publisher_configs)),
            scan_groups: std::sync::Arc::new(tokio::sync::RwLock::new(scan_groups)),
            active_driver_ui_sessions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            imported_driver_ui_sessions: std::sync::Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            driver_ui_base_dir: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            grpc_shutdown_tx: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            runtime_status: std::sync::Arc::new(tokio::sync::RwLock::new(RuntimeStatusState::default())),
            mqtt_monitor: std::sync::Arc::new(tokio::sync::Mutex::new(MqttMonitor::new())),
            mqtt_monitor_status: std::sync::Arc::new(tokio::sync::RwLock::new(MqttMonitorStatusState::default())),
            mqtt_monitor_messages: std::sync::Arc::new(tokio::sync::RwLock::new(VecDeque::new())),
            mqtt_monitor_topics: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            scan_group_runtime_metrics: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            runtime_metrics_cache: std::sync::Arc::new(tokio::sync::RwLock::new(RuntimeMetricsCacheState::default())),
        }
    }
}
