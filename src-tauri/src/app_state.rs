use crate::config::{DriverConfig, PublisherConfig, ScanGroupConfig};
use crate::core::{TagBus, TagRegistry};
use crate::drivers::DriverProcessManager;
use crate::publishers::PublisherManager;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct DriverUiSessionState {
    pub target_driver_id: Option<String>,
    pub driver_type: String,
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
    pub grpc_shutdown_tx:
        std::sync::Arc<tokio::sync::RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub runtime_status: std::sync::Arc<tokio::sync::RwLock<RuntimeStatusState>>,
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
            grpc_shutdown_tx: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            runtime_status: std::sync::Arc::new(tokio::sync::RwLock::new(RuntimeStatusState::default())),
        }
    }
}
