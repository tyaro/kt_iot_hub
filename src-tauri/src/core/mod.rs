// コアドメイン型とタグバス

pub mod tag;
pub mod tag_bus;

pub use tag::{DataType, Quality, Tag, TagId, TagValue};
pub use tag_bus::TagBus;

use std::collections::HashMap;

/// インメモリタグレジストリ
/// タグIDからタグ定義へのマッピングを管理
#[derive(Clone)]
pub struct TagRegistry {
    tags: std::sync::Arc<tokio::sync::RwLock<HashMap<TagId, Tag>>>,
}

impl TagRegistry {
    /// 新規レジストリ生成
    pub fn new() -> Self {
        Self {
            tags: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// タグを登録
    pub async fn insert(&self, tag: Tag) {
        let mut tags = self.tags.write().await;
        tags.insert(tag.id.clone(), tag);
    }

    /// タグIDでルックアップ
    pub async fn get(&self, id: &TagId) -> Option<Tag> {
        let tags = self.tags.read().await;
        tags.get(id).cloned()
    }

    /// すべてのタグを取得
    pub async fn list_all(&self) -> Vec<Tag> {
        let tags = self.tags.read().await;
        tags.values().cloned().collect()
    }

    /// タグを削除
    pub async fn remove(&self, id: &TagId) -> Option<Tag> {
        let mut tags = self.tags.write().await;
        tags.remove(id)
    }
}

impl Default for TagRegistry {
    fn default() -> Self {
        Self::new()
    }
}
