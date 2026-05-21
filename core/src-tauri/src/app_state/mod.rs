mod app_cpu;
mod driver_ui_session;
mod mqtt_monitor;
mod scan_metrics;

pub use app_cpu::{AppCpuSampleState, RuntimeMetricsCacheState, SystemCpuSampleState};
pub use driver_ui_session::DriverUiSessionState;
pub use mqtt_monitor::{MqttMonitorMessageState, MqttMonitorStatusState};
pub use scan_metrics::{DriverIoSampleState, DriverIoTotalState, ScanGroupRuntimeMetricState};

use crate::config::{DriverConfig, PublisherConfig, ScanGroupConfig};
use crate::core::{TagBus, TagRegistry};
use crate::drivers::DriverProcessManager;
use crate::publishers::PublisherManager;
use crate::subscribers::mqtt_monitor::MqttMonitor;
use std::collections::{HashMap, HashSet, VecDeque};

pub type Shared<T> = std::sync::Arc<tokio::sync::RwLock<T>>;

fn shared<T>(value: T) -> Shared<T> {
    std::sync::Arc::new(tokio::sync::RwLock::new(value))
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeStatusState {
    pub drivers_running: bool,
    pub publishers_running: bool,
    pub grpc_running: bool,
    pub last_error: Option<String>,
}

/// アプリ全体で共有する状態
#[derive(Clone)]
pub struct AppState {
    pub registry: TagRegistry,
    pub tag_bus: TagBus,
    pub drivers: Shared<DriverProcessManager>,
    pub publishers: Shared<PublisherManager>,
    pub driver_configs: Shared<Vec<DriverConfig>>,
    pub publisher_configs: Shared<Vec<PublisherConfig>>,
    pub scan_groups: Shared<Vec<ScanGroupConfig>>,
    /// session_id -> driver ui session context
    pub active_driver_ui_sessions: Shared<HashMap<String, DriverUiSessionState>>,
    /// 重複取込防止用 session_id 集合
    pub imported_driver_ui_sessions: Shared<HashSet<String>>,
    pub driver_ui_base_dir: Shared<Option<String>>,
    pub grpc_shutdown_tx: Shared<Option<tokio::sync::oneshot::Sender<()>>>,
    pub runtime_status: Shared<RuntimeStatusState>,
    pub mqtt_monitor: std::sync::Arc<tokio::sync::Mutex<MqttMonitor>>,
    pub mqtt_monitor_status: Shared<MqttMonitorStatusState>,
    pub mqtt_monitor_messages: Shared<VecDeque<MqttMonitorMessageState>>,
    pub mqtt_monitor_topics: Shared<HashMap<String, MqttMonitorMessageState>>,
    pub scan_group_runtime_metrics: Shared<HashMap<String, ScanGroupRuntimeMetricState>>,
    pub runtime_metrics_cache: Shared<RuntimeMetricsCacheState>,
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
            drivers: shared(drivers),
            publishers: shared(publishers),
            driver_configs: shared(driver_configs),
            publisher_configs: shared(publisher_configs),
            scan_groups: shared(scan_groups),
            active_driver_ui_sessions: shared(HashMap::new()),
            imported_driver_ui_sessions: shared(HashSet::new()),
            driver_ui_base_dir: shared(None),
            grpc_shutdown_tx: shared(None),
            runtime_status: shared(RuntimeStatusState::default()),
            mqtt_monitor: std::sync::Arc::new(tokio::sync::Mutex::new(MqttMonitor::new())),
            mqtt_monitor_status: shared(MqttMonitorStatusState::default()),
            mqtt_monitor_messages: shared(VecDeque::new()),
            mqtt_monitor_topics: shared(HashMap::new()),
            scan_group_runtime_metrics: shared(HashMap::new()),
            runtime_metrics_cache: shared(RuntimeMetricsCacheState::default()),
        }
    }
}
