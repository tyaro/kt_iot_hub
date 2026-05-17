use crate::app_state::{MqttMonitorMessageState, MqttMonitorStatusState};
use anyhow::Result;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, Packet, QoS};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

const MAX_MONITOR_MESSAGES: usize = 200;

pub struct MqttMonitor {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone, Debug)]
pub struct MqttMonitorStartOptions {
    pub publisher_id: String,
    pub broker: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub topic_filter: String,
    pub include_sys: bool,
}

impl MqttMonitor {
    pub fn new() -> Self {
        Self {
            stop_tx: None,
            task: None,
        }
    }

    pub async fn start(
        &mut self,
        status: std::sync::Arc<tokio::sync::RwLock<MqttMonitorStatusState>>,
        messages: std::sync::Arc<tokio::sync::RwLock<std::collections::VecDeque<MqttMonitorMessageState>>>,
        topics: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, MqttMonitorMessageState>>>,
        options: MqttMonitorStartOptions,
    ) -> Result<()> {
        self.stop(status.clone()).await?;

        {
            let mut buffer = messages.write().await;
            buffer.clear();
        }

        {
            let mut topic_map = topics.write().await;
            topic_map.clear();
        }

        {
            let mut state = status.write().await;
            state.connected = false;
            state.subscribing = true;
            state.include_sys = options.include_sys;
            state.publisher_id = Some(options.publisher_id.clone());
            state.broker = options.broker.clone();
            state.port = options.port;
            state.topic_filter = options.topic_filter.clone();
            state.message_count = 0;
            state.last_message_at = None;
            state.last_error = None;
        }

        let mut mqtt_options = MqttOptions::new(
            format!("kt_iot_hub_monitor_{}", options.publisher_id),
            options.broker.clone(),
            options.port,
        );
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        if !options.username.trim().is_empty() {
            mqtt_options.set_credentials(options.username, options.password);
        }

        let (client, mut event_loop): (AsyncClient, EventLoop) = AsyncClient::new(mqtt_options, 10);
        client
            .subscribe(options.topic_filter.clone(), QoS::AtMostOnce)
            .await?;
        if options.include_sys && options.topic_filter != "$SYS/#" {
            client.subscribe("$SYS/#", QoS::AtMostOnce).await?;
        }

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let status_handle = status.clone();
        let messages_handle = messages.clone();
        let topics_handle = topics.clone();
        let publisher_id = options.publisher_id.clone();
        let broker = options.broker.clone();
        let topic_filter = options.topic_filter.clone();

        self.task = Some(tokio::spawn(async move {
            info!(
                "MQTT monitor started: publisher_id={} broker={} topic_filter={}",
                publisher_id, broker, topic_filter
            );

            {
                let mut state = status_handle.write().await;
                state.connected = true;
                state.subscribing = true;
            }

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("MQTT monitor stopped: publisher_id={}", publisher_id);
                        let mut state = status_handle.write().await;
                        state.connected = false;
                        state.subscribing = false;
                        break;
                    }
                    event = event_loop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::Publish(publish))) => {
                                let message = MqttMonitorMessageState {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    topic: publish.topic.clone(),
                                    payload: String::from_utf8_lossy(&publish.payload).to_string(),
                                    qos: qos_to_u8(publish.qos),
                                    retain: publish.retain,
                                };

                                {
                                    let mut buffer = messages_handle.write().await;
                                    buffer.push_back(message.clone());
                                    while buffer.len() > MAX_MONITOR_MESSAGES {
                                        buffer.pop_front();
                                    }
                                }

                                {
                                    let mut topic_map = topics_handle.write().await;
                                    topic_map.insert(message.topic.clone(), message.clone());
                                }

                                {
                                    let buffer = messages_handle.read().await;
                                    let mut state = status_handle.write().await;
                                    state.message_count = buffer.len();
                                    state.last_message_at = Some(message.timestamp.clone());
                                    state.last_error = None;
                                }
                            }
                            Ok(Event::Incoming(Incoming::Disconnect)) => {
                                let mut state = status_handle.write().await;
                                state.connected = false;
                                state.subscribing = false;
                                state.last_error = Some("MQTT broker disconnected".to_string());
                            }
                            Ok(_) => {}
                            Err(e) => {
                                warn!("MQTT monitor event loop error: {}", e);
                                let mut state = status_handle.write().await;
                                state.connected = false;
                                state.subscribing = false;
                                state.last_error = Some(format!("MQTT monitor error: {}", e));
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

    pub async fn stop(
        &mut self,
        status: std::sync::Arc<tokio::sync::RwLock<MqttMonitorStatusState>>,
    ) -> Result<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }

        if let Some(mut task) = self.task.take() {
            if timeout(Duration::from_secs(2), &mut task).await.is_err() {
                warn!("MQTT monitor stop timed out; aborting task");
                task.abort();
            }
            let _ = task.await;
        }

        let mut state = status.write().await;
        state.connected = false;
        state.subscribing = false;
        Ok(())
    }
}

impl Default for MqttMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn qos_to_u8(qos: QoS) -> u8 {
    match qos {
        QoS::AtMostOnce => 0,
        QoS::AtLeastOnce => 1,
        QoS::ExactlyOnce => 2,
    }
}
