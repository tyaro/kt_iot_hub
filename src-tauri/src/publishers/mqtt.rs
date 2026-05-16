use crate::config::PublisherConfig;
use crate::core::{TagBus, TagRegistry};
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
                                let topic = match registry.get(&value.tag_id).await {
                                    Some(tag) => format!("plant/{}", tag.name),
                                    None => format!("plant/{}", value.tag_id.0),
                                };
                                let payload = serde_json::json!({
                                    "tagId": value.tag_id.0,
                                    "value": value.value,
                                    "quality": format!("{:?}", value.quality),
                                    "timestamp": value.timestamp,
                                })
                                .to_string();

                                if let Err(e) = client.publish(topic, qos_to_rumqtt(qos), false, payload).await {
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
