use crate::config::PublisherConfig;
use crate::core::{TagBus, TagRegistry};
use crate::publishers::Publisher;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration, Instant};
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

    fn get_topic_setting(&self) -> String {
        self.config
            .settings
            .get("topic")
            .or_else(|| self.config.settings.get("topic_prefix"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
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
        let topic = self.get_topic_setting();

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
        let topic_root = normalize_topic_root(&topic);

        self.task = Some(tokio::spawn(async move {
            info!("MqttPublisher {} started", publisher_id);
            let event_loop_task = tokio::spawn(async move {
                loop {
                    if let Err(e) = event_loop.poll().await {
                        warn!("MQTT event loop error: {}", e);
                    }
                }
            });

            let mut lagged_total: u64 = 0;
            let mut lagged_last_log = Instant::now();

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("MqttPublisher {} stopped", publisher_id);
                        event_loop_task.abort();
                        let _ = event_loop_task.await;
                        break;
                    }
                    recv = rx.recv() => {
                        match recv {
                            Ok(value) => {
                                let tag = registry.get(&value.tag_id).await;
                                let topic = build_topic(
                                    &topic_root,
                                    tag.as_ref().map(|tag| tag.driver_id.as_str()),
                                    tag.as_ref().map(|tag| tag.scan_group_id.as_str()),
                                    tag.as_ref().map(|tag| tag.name.as_str()),
                                    value.tag_id.0.as_str(),
                                );
                                let payload = build_payload(&value.value);

                                if let Err(e) = client.publish(topic, qos_to_rumqtt(qos), retain, payload).await {
                                    warn!("MQTT publish failed: {}", e);
                                }
                            }
                            Err(RecvError::Lagged(skipped)) => {
                                lagged_total = lagged_total.saturating_add(skipped);
                                if lagged_last_log.elapsed() >= Duration::from_secs(2) {
                                    warn!(
                                        "TagBus lag detected: skipped={} (accumulated={})",
                                        skipped,
                                        lagged_total
                                    );
                                    lagged_last_log = Instant::now();
                                }
                            }
                            Err(RecvError::Closed) => {
                                info!("TagBus is closed, stopping MqttPublisher {}", publisher_id);
                                event_loop_task.abort();
                                let _ = event_loop_task.await;
                                break;
                            }
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
        if let Some(mut task) = self.task.take() {
            match timeout(Duration::from_secs(2), &mut task).await {
                Ok(join_result) => {
                    let _ = join_result;
                }
                Err(_) => {
                    warn!("MqttPublisher {} stop timed out; aborting task", self.id);
                    task.abort();
                    let _ = task.await;
                }
            }
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

fn normalize_topic_root(prefix: &str) -> String {
    prefix.trim_matches('/').to_string()
}

fn build_topic(
    topic_root: &str,
    driver_id: Option<&str>,
    scan_group_id: Option<&str>,
    tag_name: Option<&str>,
    tag_id: &str,
) -> String {
    let driver_id = driver_id.unwrap_or("unknown-driver");
    let scan_group_id = scan_group_id.unwrap_or("unknown-group");
    let tag_segment = normalize_topic_segment(tag_name).unwrap_or(tag_id);

    if topic_root.is_empty() {
        format!("{}/tags/{}/{}", driver_id, scan_group_id, tag_segment)
    } else {
        format!(
            "{}/{}/tags/{}/{}",
            topic_root, driver_id, scan_group_id, tag_segment
        )
    }
}

fn normalize_topic_segment(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.trim_matches('/'))
        .filter(|v| !v.is_empty())
}

fn build_payload(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(v) => v.clone(),
        serde_json::Value::Number(_) | serde_json::Value::Bool(_) | serde_json::Value::Null => {
            value.to_string()
        }
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_root_is_normalized() {
        assert_eq!(normalize_topic_root("/plant/"), "plant");
        assert_eq!(normalize_topic_root("plant/line1"), "plant/line1");
    }

    #[test]
    fn topic_uses_driver_and_group_path() {
        assert_eq!(
            build_topic(
                "plant",
                Some("postgresql1"),
                Some("whr096"),
                Some("w0400"),
                "tag-001"
            ),
            "plant/postgresql1/tags/whr096/w0400"
        );
        assert_eq!(
            build_topic(
                "",
                Some("postgresql1"),
                Some("whr096"),
                Some("w0400"),
                "tag-001"
            ),
            "postgresql1/tags/whr096/w0400"
        );
    }

    #[test]
    fn topic_falls_back_to_tag_id_when_name_is_missing() {
        assert_eq!(
            build_topic(
                "plant",
                Some("postgresql1"),
                Some("whr096"),
                Some(""),
                "tag-001"
            ),
            "plant/postgresql1/tags/whr096/tag-001"
        );
    }

    #[test]
    fn payload_for_scalar_string_is_unquoted() {
        assert_eq!(build_payload(&serde_json::json!("abc")), "abc");
        assert_eq!(build_payload(&serde_json::json!(123.45)), "123.45");
    }
}
