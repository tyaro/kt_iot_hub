use crate::config::PublisherConfig;
use crate::core::{Quality, TagBus, TagRegistry};
use crate::publishers::Publisher;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::sync::oneshot;
use tracing::{info, warn};

pub struct MqttPublisher {
    id: String,
    config: PublisherConfig,
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl MqttPublisher {
    pub fn new(config: PublisherConfig) -> Self {
        Self {
            id: config.id.clone(),
            config,
            stop_tx: None,
            task: None,
        }
    }

    fn get_string_setting(&self, key: &str, default: Option<&str>) -> Result<String> {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .or_else(|| default.map(ToString::to_string))
            .ok_or_else(|| anyhow!("Missing publisher setting: {}", key))
    }

    fn get_u16_setting(&self, key: &str, default: u16) -> u16 {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or(default)
    }

    fn get_u8_setting(&self, key: &str, default: u8) -> u8 {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as u8)
            .unwrap_or(default)
    }

    fn get_bool_setting(&self, key: &str, default: bool) -> bool {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }
}

#[async_trait]
impl Publisher for MqttPublisher {
    fn id(&self) -> &str {
        &self.id
    }

    fn publisher_type(&self) -> &str {
        "mqtt"
    }

    async fn start(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()> {
        if self.task.is_some() {
            return Ok(());
        }

        let broker = self.get_string_setting("broker", Some("127.0.0.1"))?;
        let port = self.get_u16_setting("port", 1883);
        let client_id = self.get_string_setting("client_id", Some("kt_iot_hub"))?;
        let qos = self.get_u8_setting("qos", 1);
        let retain = self.get_bool_setting("retain", false);
        let topic_prefix = self.get_string_setting("topic_prefix", Some("plant"))?;

        let mut options = MqttOptions::new(client_id, broker, port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let username = self
            .config
            .settings
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let password = self
            .config
            .settings
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !username.is_empty() {
            options.set_credentials(username, password);
        }

        let (client, mut event_loop): (AsyncClient, EventLoop) = AsyncClient::new(options, 10);
        let mut rx = bus.subscribe();
        let registry = registry.clone();
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let publisher_id = self.id.clone();
        let topic_prefix = normalize_topic_prefix(&topic_prefix);

        self.task = Some(tokio::spawn(async move {
            info!("MqttPublisher {} started", publisher_id);
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("MqttPublisher {} stopped", publisher_id);
                        break;
                    }
                    recv = rx.recv() => {
                        match recv {
                            Ok(value) => {
                                let tag = registry.get(&value.tag_id).await;
                                let topic = build_topic(&topic_prefix, value.tag_id.0.as_str());
                                let payload = serde_json::json!({
                                    "tagId": value.tag_id.0,
                                    "tagName": tag.as_ref().map(|tag| tag.name.as_str()),
                                    "value": value.value,
                                    "quality": quality_to_str(value.quality),
                                    "timestamp": value.timestamp,
                                })
                                .to_string();

                                if let Err(e) = client.publish(topic, qos_to_rumqtt(qos), retain, payload).await {
                                    warn!("MQTT publish failed: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("TagBus receive error: {}", e);
                            }
                        }
                    }
                    poll_res = event_loop.poll() => {
                        if let Err(e) = poll_res {
                            warn!("MQTT event loop error: {}", e);
                        }
                    }
                }
            }
        }));

        self.stop_tx = Some(stop_tx);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        Ok(())
    }

    async fn test_connection(&self) -> Result<()> {
        let _ = self.get_string_setting("broker", Some("127.0.0.1"))?;
        Ok(())
    }
}

fn qos_to_rumqtt(qos: u8) -> QoS {
    match qos {
        0 => QoS::AtMostOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtLeastOnce,
    }
}

fn quality_to_str(quality: Quality) -> &'static str {
    match quality {
        Quality::Good => "good",
        Quality::Uncertain => "uncertain",
        Quality::Bad => "bad",
    }
}

fn normalize_topic_prefix(prefix: &str) -> String {
    prefix.trim_matches('/').to_string()
}

fn build_topic(prefix: &str, tag_id: &str) -> String {
    if prefix.is_empty() {
        tag_id.to_string()
    } else {
        format!("{}/{}", prefix, tag_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_prefix_is_normalized() {
        assert_eq!(normalize_topic_prefix("/plant/"), "plant");
        assert_eq!(normalize_topic_prefix("plant/line1"), "plant/line1");
    }

    #[test]
    fn topic_uses_stable_tag_id() {
        assert_eq!(build_topic("plant", "tag-001"), "plant/tag-001");
        assert_eq!(build_topic("", "tag-001"), "tag-001");
    }

    #[test]
    fn quality_is_serialized_in_snake_case() {
        assert_eq!(quality_to_str(Quality::Good), "good");
        assert_eq!(quality_to_str(Quality::Uncertain), "uncertain");
        assert_eq!(quality_to_str(Quality::Bad), "bad");
    }
}
