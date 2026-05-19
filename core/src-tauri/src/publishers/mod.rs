pub mod mqtt;

// パブリッシャ trait と管理

use crate::core::{TagBus, TagRegistry};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info};

/// すべてのパブリッシャが実装すべき trait
#[async_trait]
pub trait Publisher: Send + Sync {
    /// パブリッシャの一意識別子
    fn id(&self) -> &str;

    /// パブリッシャの種類（"mqtt", "rest" など）
    #[allow(dead_code)]
    fn publisher_type(&self) -> &str;

    /// 設定スキーマ
    #[allow(dead_code)]
    fn settings_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    /// 起動
    async fn start(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()>;

    /// 停止
    async fn stop(&mut self) -> Result<()>;

    /// 接続テスト
    #[allow(dead_code)]
    async fn test_connection(&self) -> Result<()>;
}

/// パブリッシャマネージャー
pub struct PublisherManager {
    publishers: HashMap<String, Box<dyn Publisher>>,
}

impl PublisherManager {
    pub fn new() -> Self {
        Self {
            publishers: HashMap::new(),
        }
    }

    /// パブリッシャを登録
    pub fn register(&mut self, publisher: Box<dyn Publisher>) {
        let id = publisher.id().to_string();
        self.publishers.insert(id, publisher);
    }

    /// パブリッシャを登録解除する
    pub fn unregister(&mut self, publisher_id: &str) -> Option<Box<dyn Publisher>> {
        self.publishers.remove(publisher_id)
    }

    /// パブリッシャを起動
    pub async fn start_publisher(
        &mut self,
        publisher_id: &str,
        registry: &TagRegistry,
        bus: &TagBus,
    ) -> Result<()> {
        if let Some(publisher) = self.publishers.get_mut(publisher_id) {
            info!("Starting publisher: {}", publisher_id);
            publisher.start(registry, bus).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Publisher not found: {}", publisher_id))
        }
    }

    /// すべてのパブリッシャを起動
    #[allow(dead_code)]
    pub async fn start_all(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()> {
        for publisher in self.publishers.values_mut() {
            if let Err(e) = publisher.start(registry, bus).await {
                error!("Failed to start publisher {}: {}", publisher.id(), e);
            }
        }
        Ok(())
    }

    /// パブリッシャを停止
    pub async fn stop_publisher(&mut self, publisher_id: &str) -> Result<()> {
        if let Some(publisher) = self.publishers.get_mut(publisher_id) {
            info!("Stopping publisher: {}", publisher_id);
            publisher.stop().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Publisher not found: {}", publisher_id))
        }
    }

    /// すべてのパブリッシャを停止
    pub async fn stop_all(&mut self) -> Result<()> {
        for publisher in self.publishers.values_mut() {
            if let Err(e) = publisher.stop().await {
                error!("Failed to stop publisher {}: {}", publisher.id(), e);
            }
        }
        Ok(())
    }

    /// パブリッシャを取得
    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&dyn Publisher> {
        self.publishers.get(id).map(|p| p.as_ref())
    }

    /// 登録済みパブリッシャのID一覧
    #[allow(dead_code)]
    pub fn list_publisher_ids(&self) -> Vec<&str> {
        self.publishers.keys().map(|s| s.as_str()).collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.publishers.contains_key(id)
    }
}

impl Default for PublisherManager {
    fn default() -> Self {
        Self::new()
    }
}
