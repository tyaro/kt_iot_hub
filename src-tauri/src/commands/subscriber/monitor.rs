use crate::app_state::AppState;
use crate::commands::dto::{
    ErrorResponse, MqttMonitorMessageDto, MqttMonitorPublisherDto, MqttMonitorStatusDto,
    StartMqttMonitorRequest,
};
use crate::subscribers::mqtt_monitor::MqttMonitorStartOptions;

#[tauri::command]
pub async fn list_mqtt_monitor_publishers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MqttMonitorPublisherDto>, ErrorResponse> {
    let configs = state.publisher_configs.read().await;
    Ok(configs
        .iter()
        .filter(|cfg| cfg.publisher_type == "mqtt")
        .map(|cfg| MqttMonitorPublisherDto {
            id: cfg.id.clone(),
            broker: cfg
                .settings
                .get("broker")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: cfg
                .settings
                .get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(1883) as u16,
            topic: cfg
                .settings
                .get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn get_mqtt_monitor_status(
    state: tauri::State<'_, AppState>,
) -> Result<MqttMonitorStatusDto, ErrorResponse> {
    let status = state.mqtt_monitor_status.read().await.clone();
    Ok(status.into())
}

#[tauri::command]
pub async fn list_mqtt_monitor_messages(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MqttMonitorMessageDto>, ErrorResponse> {
    let messages = state.mqtt_monitor_messages.read().await;
    Ok(messages.iter().cloned().map(Into::into).collect())
}

#[tauri::command]
pub async fn clear_mqtt_monitor_messages(
    state: tauri::State<'_, AppState>,
) -> Result<(), ErrorResponse> {
    state.mqtt_monitor_messages.write().await.clear();
    state.mqtt_monitor_status.write().await.message_count = 0;
    Ok(())
}

#[tauri::command]
pub async fn start_mqtt_monitor(
    state: tauri::State<'_, AppState>,
    req: StartMqttMonitorRequest,
) -> Result<MqttMonitorStatusDto, ErrorResponse> {
    let publisher_id = req.publisher_id.trim().to_string();
    if publisher_id.is_empty() {
        return Err(ErrorResponse {
            error: "Publisher ID cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let topic_filter = req.topic_filter.trim().to_string();
    if topic_filter.is_empty() {
        return Err(ErrorResponse {
            error: "Topic filter cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let config = {
        let configs = state.publisher_configs.read().await;
        configs
            .iter()
            .find(|cfg| cfg.id == publisher_id && cfg.publisher_type == "mqtt")
            .cloned()
    }
    .ok_or_else(|| ErrorResponse {
        error: format!("Publisher not found: {}", publisher_id),
        code: "NOT_FOUND".to_string(),
    })?;

    let options = MqttMonitorStartOptions {
        publisher_id: config.id.clone(),
        broker: config
            .settings
            .get("broker")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1")
            .to_string(),
        port: config
            .settings
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(1883) as u16,
        username: config
            .settings
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        password: config
            .settings
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        topic_filter,
    };

    let mut monitor = state.mqtt_monitor.lock().await;
    monitor
        .start(
            state.mqtt_monitor_status.clone(),
            state.mqtt_monitor_messages.clone(),
            options,
        )
        .await
        .map_err(ErrorResponse::from)?;

    let status = state.mqtt_monitor_status.read().await.clone();
    Ok(status.into())
}

#[tauri::command]
pub async fn stop_mqtt_monitor(
    state: tauri::State<'_, AppState>,
) -> Result<MqttMonitorStatusDto, ErrorResponse> {
    let mut monitor = state.mqtt_monitor.lock().await;
    monitor
        .stop(state.mqtt_monitor_status.clone())
        .await
        .map_err(ErrorResponse::from)?;

    let status = state.mqtt_monitor_status.read().await.clone();
    Ok(status.into())
}
