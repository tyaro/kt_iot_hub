use crate::app_state::AppState;
use crate::commands::dto::{
    ErrorResponse, GetMqttMonitorTopicDetailRequest, GetMqttMonitorTreeRequest,
    MqttMonitorMessageDto, MqttMonitorPublisherDto, MqttMonitorStatusDto,
    MqttMonitorTopicDetailDto, MqttMonitorTopicNodeDto, StartMqttMonitorRequest,
};
use crate::subscribers::mqtt_monitor::MqttMonitorStartOptions;
use std::collections::{HashMap, HashSet};
use tauri::Manager;

#[derive(Clone, Debug)]
struct MutableTopicNode {
    label: String,
    full_path: String,
    latest_message: Option<MqttMonitorMessageDto>,
    children: HashMap<String, MutableTopicNode>,
}

impl MutableTopicNode {
    fn new(label: String, full_path: String) -> Self {
        Self {
            label,
            full_path,
            latest_message: None,
            children: HashMap::new(),
        }
    }
}

#[tauri::command]
pub async fn open_mqtt_monitor_window(app: tauri::AppHandle) -> Result<(), ErrorResponse> {
    if let Some(window) = app.get_webview_window("mqtt-monitor") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        "mqtt-monitor",
        tauri::WebviewUrl::App("/?view=mqtt-monitor".into()),
    )
    .title("MQTT Monitor")
    .inner_size(1180.0, 760.0)
    .min_inner_size(920.0, 620.0)
    .resizable(true)
    .build()
    .map_err(|e| ErrorResponse {
        error: format!("Failed to open MQTT monitor window: {}", e),
        code: "WINDOW_OPEN_FAILED".to_string(),
    })?;

    Ok(())
}

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

fn build_visible_topic_tree(
    topics: &HashMap<String, crate::app_state::MqttMonitorMessageState>,
    expanded_paths: &HashSet<String>,
    publisher_topic_root: Option<String>,
    include_sys: bool,
    topic_filter: &str,
    include_all: bool,
) -> Vec<MqttMonitorTopicNodeDto> {
    let mut roots = HashMap::<String, MutableTopicNode>::new();

    if let Some(root) = publisher_topic_root
        .or_else(|| first_fixed_topic_segment(topic_filter).map(|segment| segment.to_string()))
    {
        roots
            .entry(root.clone())
            .or_insert_with(|| MutableTopicNode::new(root.clone(), root));
    }

    if include_sys {
        roots
            .entry("$SYS".to_string())
            .or_insert_with(|| MutableTopicNode::new("$SYS".to_string(), "$SYS".to_string()));
    }

    for message in topics.values() {
        let segments = message
            .topic
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            continue;
        }

        let message_dto: MqttMonitorMessageDto = message.clone().into();
        let mut current_map = &mut roots;
        let mut current_path = String::new();

        for segment in segments {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(segment);

            let node = current_map
                .entry(segment.to_string())
                .or_insert_with(|| MutableTopicNode::new(segment.to_string(), current_path.clone()));
            node.latest_message = Some(message_dto.clone());
            current_map = &mut node.children;
        }
    }

    let mut nodes = roots
        .into_values()
        .map(|node| freeze_visible_node(node, expanded_paths, include_all))
        .collect::<Vec<_>>();
    sort_topic_nodes(&mut nodes);
    nodes
}

fn freeze_visible_node(
    node: MutableTopicNode,
    expanded_paths: &HashSet<String>,
    include_all: bool,
) -> MqttMonitorTopicNodeDto {
    let has_children = !node.children.is_empty();
    let mut children = if has_children && (include_all || expanded_paths.contains(&node.full_path)) {
        node.children
            .into_values()
            .map(|child| freeze_visible_node(child, expanded_paths, include_all))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    sort_topic_nodes(&mut children);

    MqttMonitorTopicNodeDto {
        id: node.full_path.clone(),
        label: node.label,
        full_path: node.full_path,
        has_children,
        latest_message: node.latest_message,
        children,
    }
}

fn sort_topic_nodes(nodes: &mut [MqttMonitorTopicNodeDto]) {
    nodes.sort_by(|a, b| {
        let a_is_sys = a.label == "$SYS";
        let b_is_sys = b.label == "$SYS";

        a_is_sys
            .cmp(&b_is_sys)
            .reverse()
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.label.cmp(&b.label))
    });
}

fn find_latest_message_for_path(
    topics: &HashMap<String, crate::app_state::MqttMonitorMessageState>,
    full_path: &str,
) -> Option<MqttMonitorMessageDto> {
    topics
        .values()
        .filter(|message| {
            message.topic == full_path || message.topic.starts_with(&format!("{}/", full_path))
        })
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))
        .cloned()
        .map(Into::into)
}

fn first_fixed_topic_segment(topic_filter: &str) -> Option<&str> {
    topic_filter
        .split('/')
        .map(str::trim)
        .find(|segment| !segment.is_empty() && *segment != "#" && *segment != "+")
    }
