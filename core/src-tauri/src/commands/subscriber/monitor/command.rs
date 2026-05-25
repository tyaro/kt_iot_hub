use super::tree::{
    build_visible_topic_tree, find_latest_message_for_path, first_fixed_topic_segment,
};
use crate::app_state::AppState;
use crate::commands::dto::{
    ErrorResponse, GetMqttMonitorTopicDetailRequest, GetMqttMonitorTreeRequest,
    MqttMonitorMessageDto, MqttMonitorPublisherDto, MqttMonitorStatusDto,
    MqttMonitorTopicDetailDto, MqttMonitorTopicNodeDto, StartMqttMonitorRequest,
};
use crate::commands::secret_store::read_password_setting;
use crate::subscribers::mqtt_monitor::MqttMonitorStartOptions;
use std::collections::HashSet;

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
pub async fn get_mqtt_monitor_tree(
    state: tauri::State<'_, AppState>,
    req: GetMqttMonitorTreeRequest,
) -> Result<Vec<MqttMonitorTopicNodeDto>, ErrorResponse> {
    let include_all = req.include_all;
    let topics = state.mqtt_monitor_topics.read().await;
    let status = state.mqtt_monitor_status.read().await.clone();
    let publisher_topic_root = {
        let publisher_id = status.publisher_id.clone();
        let configs = state.publisher_configs.read().await;
        publisher_id.and_then(|id| {
            configs
                .iter()
                .find(|cfg| cfg.id == id && cfg.publisher_type == "mqtt")
                .and_then(|cfg| cfg.settings.get("topic"))
                .and_then(|v| v.as_str())
                .and_then(first_fixed_topic_segment)
                .map(|segment| segment.to_string())
        })
    };
    let expanded_paths = req
        .expanded_paths
        .into_iter()
        .map(|path| path.trim().trim_matches('/').to_string())
        .filter(|path| !path.is_empty())
        .collect::<HashSet<_>>();

    Ok(build_visible_topic_tree(
        &topics,
        &expanded_paths,
        publisher_topic_root,
        status.include_sys,
        &status.topic_filter,
        include_all,
    ))
}

#[tauri::command]
pub async fn get_mqtt_monitor_topic_detail(
    state: tauri::State<'_, AppState>,
    req: GetMqttMonitorTopicDetailRequest,
) -> Result<MqttMonitorTopicDetailDto, ErrorResponse> {
    let full_path = req.full_path.trim().trim_matches('/').to_string();
    if full_path.is_empty() {
        return Ok(MqttMonitorTopicDetailDto {
            full_path,
            latest_message: None,
        });
    }

    let topics = state.mqtt_monitor_topics.read().await;
    let latest_message = find_latest_message_for_path(&topics, &full_path);

    Ok(MqttMonitorTopicDetailDto {
        full_path,
        latest_message,
    })
}

#[tauri::command]
pub async fn clear_mqtt_monitor_messages(
    state: tauri::State<'_, AppState>,
) -> Result<(), ErrorResponse> {
    state.mqtt_monitor_messages.write().await.clear();
    state.mqtt_monitor_topics.write().await.clear();
    let mut status = state.mqtt_monitor_status.write().await;
    status.message_count = 0;
    status.last_message_at = None;
    status.last_error = None;
    Ok(())
}

#[tauri::command]
pub async fn start_mqtt_monitor(
    state: tauri::State<'_, AppState>,
    req: StartMqttMonitorRequest,
) -> Result<MqttMonitorStatusDto, ErrorResponse> {
    let publisher_id = req.publisher_id.trim().to_string();
    if publisher_id.is_empty() {
        return Err(ErrorResponse::invalid_input("Publisher ID cannot be empty"));
    }

    let topic_filter = req.topic_filter.trim().to_string();
    if topic_filter.is_empty() {
        return Err(ErrorResponse::invalid_input("Topic filter cannot be empty"));
    }

    let config = {
        let configs = state.publisher_configs.read().await;
        configs
            .iter()
            .find(|cfg| cfg.id == publisher_id && cfg.publisher_type == "mqtt")
            .cloned()
    }
    .ok_or_else(|| ErrorResponse::not_found(format!("Publisher not found: {}", publisher_id)))?;

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
        password: read_password_setting(&config.settings).unwrap_or_default(),
        topic_filter,
        include_sys: req.include_sys,
    };

    let mut monitor = state.mqtt_monitor.lock().await;
    monitor
        .start(
            state.mqtt_monitor_status.clone(),
            state.mqtt_monitor_messages.clone(),
            state.mqtt_monitor_topics.clone(),
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
