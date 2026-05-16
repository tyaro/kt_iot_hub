use crate::config::{DriverConfig, PublisherConfig, ScanGroupConfig};
use crate::core::{TagBus, TagRegistry};
use crate::drivers::DriverManager;
use crate::publishers::PublisherManager;

/// アプリ全体で共有する状態
#[derive(Clone)]
pub struct AppState {
    pub registry: TagRegistry,
    pub tag_bus: TagBus,
    pub drivers: std::sync::Arc<tokio::sync::RwLock<DriverManager>>,
    pub publishers: std::sync::Arc<tokio::sync::RwLock<PublisherManager>>,
    pub driver_configs: std::sync::Arc<tokio::sync::RwLock<Vec<DriverConfig>>>,
    pub publisher_configs: std::sync::Arc<tokio::sync::RwLock<Vec<PublisherConfig>>>,
    pub scan_groups: std::sync::Arc<tokio::sync::RwLock<Vec<ScanGroupConfig>>>,
}

impl AppState {
    pub fn new(
        registry: TagRegistry,
        tag_bus: TagBus,
        drivers: DriverManager,
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
        }
    }
}
