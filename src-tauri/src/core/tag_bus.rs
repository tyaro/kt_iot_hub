// タグバス: ドライバ・パブリッシャ間の疎結合通信
// Tokio broadcast channel を使用
// 1→多 のパターンに対応（1ドライバ が複数パブリッシャへ配信）

use super::TagValue;
use tokio::sync::broadcast;

/// タグバスの容量（バッファサイズ）
/// 複数のサブスクライバが遅延することを想定
const TAG_BUS_CAPACITY: usize = 8192;

/// タグバスハンドル
/// ドライバが値を発行し、パブリッシャが購読する
#[derive(Clone)]
pub struct TagBus {
    tx: broadcast::Sender<TagValue>,
}

impl TagBus {
    /// 新規タグバスを作成
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(TAG_BUS_CAPACITY);
        Self { tx }
    }

    /// タグ値を発行（ドライバから呼ばれる）
    pub fn publish(&self, value: TagValue) {
        // エラーは無視（サブスクライバがいない場合も含む）
        let _ = self.tx.send(value);
    }

    /// サブスクライバーを取得（パブリッシャから呼ばれる）
    pub fn subscribe(&self) -> broadcast::Receiver<TagValue> {
        self.tx.subscribe()
    }

    /// 現在のサブスクライバ数を取得
    #[allow(dead_code)]
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for TagBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TagId;

    #[tokio::test]
    async fn test_tag_bus_publish_subscribe() {
        let bus = TagBus::new();
        let mut rx = bus.subscribe();

        let tag_id = TagId("test-001".to_string());
        let value = TagValue::good(tag_id.clone(), serde_json::json!(42));

        bus.publish(value.clone());

        // サブスクライバが値を受け取れるか
        let received = rx.recv().await.expect("should receive");
        assert_eq!(received.tag_id, tag_id);
    }

    #[tokio::test]
    async fn test_tag_bus_multiple_subscribers() {
        let bus = TagBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let tag_id = TagId("test-002".to_string());
        let value = TagValue::good(tag_id.clone(), serde_json::json!(100));

        bus.publish(value.clone());

        // 複数のサブスクライバが同じ値を受け取れるか
        let v1 = rx1.recv().await.expect("rx1 should receive");
        let v2 = rx2.recv().await.expect("rx2 should receive");

        assert_eq!(v1.tag_id, tag_id);
        assert_eq!(v2.tag_id, tag_id);
    }
}
