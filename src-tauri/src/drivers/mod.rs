pub mod postgres;

// ドライバ trait と管理

use crate::core::{TagBus, TagId, TagRegistry};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info};

/// すべてのドライバが実装すべき trait
#[async_trait]
pub trait Driver: Send + Sync {
    /// ドライバの一意識別子
    fn id(&self) -> &str;

    /// ドライバの種類（"postgres", "mqtt" など）
    fn driver_type(&self) -> &str;

    /// 登録スキーマ（UI が設定フォーム生成に使う JSON Schema）
    fn registration_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    /// 起動（非同期、リソース取得など）
    async fn start(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()>;

    /// 停止（クリーンアップ）
    async fn stop(&mut self) -> Result<()>;

    /// タグを登録
    async fn register_tag(&mut self, tag_id: &TagId) -> Result<()>;

    /// タグの登録を解除
    async fn unregister_tag(&mut self, tag_id: &TagId) -> Result<()>;
}

/// ドライバマネージャー
pub struct DriverManager {
    drivers: HashMap<String, Box<dyn Driver>>,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    /// ドライバを登録
    pub fn register(&mut self, driver: Box<dyn Driver>) {
        let id = driver.id().to_string();
        self.drivers.insert(id, driver);
    }

    /// ドライバを起動
    pub async fn start_driver(
        &mut self,
        driver_id: &str,
        registry: &TagRegistry,
        bus: &TagBus,
    ) -> Result<()> {
        if let Some(driver) = self.drivers.get_mut(driver_id) {
            info!("Starting driver: {}", driver_id);
            driver.start(registry, bus).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Driver not found: {}", driver_id))
        }
    }

    /// すべてのドライバを起動
    pub async fn start_all(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()> {
        for driver in self.drivers.values_mut() {
            if let Err(e) = driver.start(registry, bus).await {
                error!("Failed to start driver {}: {}", driver.id(), e);
            }
        }
        Ok(())
    }

    /// ドライバを停止
    pub async fn stop_driver(&mut self, driver_id: &str) -> Result<()> {
        if let Some(driver) = self.drivers.get_mut(driver_id) {
            info!("Stopping driver: {}", driver_id);
            driver.stop().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Driver not found: {}", driver_id))
        }
    }

    /// すべてのドライバを停止
    pub async fn stop_all(&mut self) -> Result<()> {
        for driver in self.drivers.values_mut() {
            if let Err(e) = driver.stop().await {
                error!("Failed to stop driver {}: {}", driver.id(), e);
            }
        }
        Ok(())
    }

    /// ドライバを取得
    pub fn get(&self, id: &str) -> Option<&Box<dyn Driver>> {
        self.drivers.get(id)
    }

    /// 登録済みドライバのID一覧
    pub fn list_driver_ids(&self) -> Vec<&str> {
        self.drivers.keys().map(|s| s.as_str()).collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.drivers.contains_key(id)
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}
