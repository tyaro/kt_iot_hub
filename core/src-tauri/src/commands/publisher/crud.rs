use super::runtime_sync::sync_publisher_runtime;
use super::toml_io::write_publishers_toml_atomic;
use crate::app_state::AppState;
use crate::commands::dto::{ErrorResponse, PublisherDto, SavePublisherRequest};
use crate::config::PublisherConfig;

#[tauri::command]
pub async fn list_publishers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PublisherDto>, ErrorResponse> {
    let configs = state.publisher_configs.read().await;
    Ok(configs
        .iter()
        .map(|cfg| PublisherDto {
            id: cfg.id.clone(),
            publisher_type: cfg.publisher_type.clone(),
            enabled: cfg.enabled.unwrap_or(true),
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
            username: cfg
                .settings
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            client_id: cfg
                .settings
                .get("client_id")
                .and_then(|v| v.as_str())
                .unwrap_or("kt_iot_hub")
                .to_string(),
            qos: cfg
                .settings
                .get("qos")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u8,
            retain: cfg
                .settings
                .get("retain")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            topic: cfg
                .settings
                .get("topic")
                .or_else(|| cfg.settings.get("topic_prefix"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn save_publisher(
    state: tauri::State<'_, AppState>,
    req: SavePublisherRequest,
) -> Result<(), ErrorResponse> {
    let publisher_id = req.id.trim().to_string();
    if publisher_id.is_empty() {
        return Err(ErrorResponse::invalid_input("Publisher ID cannot be empty"));
    }
    if req.publisher_type.trim() != "mqtt" {
        return Err(ErrorResponse::invalid_input(
            "Only mqtt publisher is supported",
        ));
    }
    if req.broker.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("Broker cannot be empty"));
    }
    if req.client_id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("Client ID cannot be empty"));
    }
    if req.qos > 2 {
        return Err(ErrorResponse::invalid_input("QoS must be 0, 1, or 2"));
    }

    let existing_configs = state.publisher_configs.read().await.clone();
    let existing = existing_configs
        .iter()
        .find(|cfg| cfg.id == publisher_id)
        .cloned();

    let password = if req.password.trim().is_empty() {
        existing
            .as_ref()
            .and_then(|cfg| cfg.settings.get("password"))
            .cloned()
            .unwrap_or_else(|| serde_json::Value::String(String::new()))
    } else {
        serde_json::Value::String(req.password.clone())
    };

    let settings = serde_json::Map::from_iter([
        (
            "broker".to_string(),
            serde_json::Value::String(req.broker.trim().to_string()),
        ),
        ("port".to_string(), serde_json::Value::from(req.port)),
        (
            "username".to_string(),
            serde_json::Value::String(req.username.trim().to_string()),
        ),
        ("password".to_string(), password),
        (
            "client_id".to_string(),
            serde_json::Value::String(req.client_id.trim().to_string()),
        ),
        ("qos".to_string(), serde_json::Value::from(req.qos)),
        ("retain".to_string(), serde_json::Value::Bool(req.retain)),
        (
            "topic".to_string(),
            serde_json::Value::String(req.topic.trim_matches('/').to_string()),
        ),
    ]);

    let config = PublisherConfig {
        id: publisher_id.clone(),
        publisher_type: req.publisher_type,
        enabled: Some(req.enabled),
        settings: serde_json::Value::Object(settings),
    };

    let final_configs = {
        let mut configs = state.publisher_configs.write().await;
        if let Some(existing) = configs.iter_mut().find(|cfg| cfg.id == config.id) {
            *existing = config.clone();
        } else {
            configs.push(config.clone());
        }
        configs.clone()
    };

    write_publishers_toml_atomic(&final_configs)?;
    sync_publisher_runtime(&state, &config).await?;

    Ok(())
}
