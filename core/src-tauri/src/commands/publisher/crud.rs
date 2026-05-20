use super::runtime_sync::sync_publisher_runtime;
use super::toml_io::write_publishers_toml_atomic;
use crate::app_state::AppState;
use crate::commands::dto::{
    ErrorResponse, GetMqttPublishModeRequest, GetMqttPublishModeResponse, PublisherDto,
    SavePublisherRequest, SetMqttPublishModeRequest, SetMqttPublishModeResponse,
};
use crate::config::PublisherConfig;
use std::collections::HashMap;

const MQTT_PUBLISH_MODE_SCAN_INTERVAL: &str = "scan_interval";
const MQTT_PUBLISH_MODE_ON_CHANGE: &str = "on_change";

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
            publish_mode_default: cfg
                .settings
                .get("publish_mode_default")
                .and_then(|v| v.as_str())
                .map(normalize_publish_mode)
                .unwrap_or(MQTT_PUBLISH_MODE_SCAN_INTERVAL)
                .to_string(),
            publish_mode_by_driver: read_publish_mode_map(&cfg.settings, "publish_mode_by_driver"),
            publish_mode_by_scan_group: read_publish_mode_map(
                &cfg.settings,
                "publish_mode_by_scan_group",
            ),
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

    let mut settings = existing
        .as_ref()
        .and_then(|cfg| cfg.settings.as_object().cloned())
        .unwrap_or_default();

    settings.insert(
        "broker".to_string(),
        serde_json::Value::String(req.broker.trim().to_string()),
    );
    settings.insert("port".to_string(), serde_json::Value::from(req.port));
    settings.insert(
        "username".to_string(),
        serde_json::Value::String(req.username.trim().to_string()),
    );
    settings.insert("password".to_string(), password);
    settings.insert(
        "client_id".to_string(),
        serde_json::Value::String(req.client_id.trim().to_string()),
    );
    settings.insert("qos".to_string(), serde_json::Value::from(req.qos));
    settings.insert("retain".to_string(), serde_json::Value::Bool(req.retain));
    settings.insert(
        "topic".to_string(),
        serde_json::Value::String(req.topic.trim_matches('/').to_string()),
    );

    if let Some(default_mode) = req.publish_mode_default.as_deref() {
        settings.insert(
            "publish_mode_default".to_string(),
            serde_json::Value::String(normalize_publish_mode(default_mode).to_string()),
        );
    }
    if let Some(by_driver) = req.publish_mode_by_driver.as_ref() {
        settings.insert(
            "publish_mode_by_driver".to_string(),
            serialize_publish_mode_map(by_driver),
        );
    }
    if let Some(by_scan_group) = req.publish_mode_by_scan_group.as_ref() {
        settings.insert(
            "publish_mode_by_scan_group".to_string(),
            serialize_publish_mode_map(by_scan_group),
        );
    }

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

#[tauri::command]
pub async fn get_mqtt_publish_mode(
    state: tauri::State<'_, AppState>,
    req: GetMqttPublishModeRequest,
) -> Result<GetMqttPublishModeResponse, ErrorResponse> {
    let driver_id = req.driver_id.trim();
    if driver_id.is_empty() {
        return Err(ErrorResponse::invalid_input("driver_id cannot be empty"));
    }

    let target = {
        let configs = state.publisher_configs.read().await;
        configs
            .iter()
            .find(|cfg| cfg.publisher_type == "mqtt")
            .cloned()
    }
    .ok_or_else(|| ErrorResponse::not_found("MQTT publisher not found".to_string()))?;

    let by_driver = read_publish_mode_map(&target.settings, "publish_mode_by_driver");
    let by_scan_group = read_publish_mode_map(&target.settings, "publish_mode_by_scan_group");
    let default_mode = target
        .settings
        .get("publish_mode_default")
        .and_then(|v| v.as_str())
        .map(normalize_publish_mode)
        .unwrap_or(MQTT_PUBLISH_MODE_SCAN_INTERVAL)
        .to_string();

    let (mode, source) = if let Some(scan_group_id) = req.scan_group_id.as_deref() {
        let key = format!("{}::{}", driver_id, scan_group_id.trim());
        if let Some(mode) = by_scan_group.get(&key) {
            (mode.clone(), "scan_group".to_string())
        } else if let Some(mode) = by_driver.get(driver_id) {
            (mode.clone(), "driver".to_string())
        } else {
            (default_mode, "default".to_string())
        }
    } else if let Some(mode) = by_driver.get(driver_id) {
        (mode.clone(), "driver".to_string())
    } else {
        (default_mode, "default".to_string())
    };

    Ok(GetMqttPublishModeResponse {
        publisher_id: target.id,
        mode,
        source,
    })
}

#[tauri::command]
pub async fn set_mqtt_publish_mode(
    state: tauri::State<'_, AppState>,
    req: SetMqttPublishModeRequest,
) -> Result<SetMqttPublishModeResponse, ErrorResponse> {
    let mode = normalize_publish_mode(req.mode.as_str()).to_string();
    let driver_id = req.driver_id.trim().to_string();
    if driver_id.is_empty() {
        return Err(ErrorResponse::invalid_input("driver_id cannot be empty"));
    }

    let target_publisher_id = {
        let configs = state.publisher_configs.read().await;
        if let Some(publisher_id) = req.publisher_id.as_deref() {
            let found = configs
                .iter()
                .find(|cfg| cfg.id == publisher_id && cfg.publisher_type == "mqtt")
                .map(|cfg| cfg.id.clone());
            found.ok_or_else(|| {
                ErrorResponse::not_found(format!("MQTT publisher not found: {}", publisher_id))
            })?
        } else {
            configs
                .iter()
                .find(|cfg| cfg.publisher_type == "mqtt")
                .map(|cfg| cfg.id.clone())
                .ok_or_else(|| ErrorResponse::not_found("MQTT publisher not found".to_string()))?
        }
    };

    let (updated_config, final_configs) = {
        let mut configs = state.publisher_configs.write().await;
        let target = configs
            .iter_mut()
            .find(|cfg| cfg.id == target_publisher_id)
            .ok_or_else(|| {
                ErrorResponse::not_found(format!("Publisher not found: {}", target_publisher_id))
            })?;

        let settings = target.settings.as_object_mut().ok_or_else(|| {
            ErrorResponse::invalid_input("publisher settings must be an object")
        })?;

        let scope = req.scope.trim().to_ascii_lowercase();
        match scope.as_str() {
            "driver" => {
                upsert_publish_mode_entry(settings, "publish_mode_by_driver", &driver_id, &mode)?;
            }
            "scan_group" => {
                let scan_group_id = req
                    .scan_group_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        ErrorResponse::invalid_input("scan_group_id is required for scan_group scope")
                    })?;
                let key = format!("{}::{}", driver_id, scan_group_id);
                upsert_publish_mode_entry(settings, "publish_mode_by_scan_group", &key, &mode)?;
            }
            _ => {
                return Err(ErrorResponse::invalid_input(
                    "scope must be 'driver' or 'scan_group'",
                ));
            }
        }

        let updated = target.clone();
        (updated, configs.clone())
    };

    write_publishers_toml_atomic(&final_configs)?;
    sync_publisher_runtime(&state, &updated_config).await?;

    Ok(SetMqttPublishModeResponse {
        publisher_id: updated_config.id,
        scope: req.scope,
        driver_id,
        scan_group_id: req.scan_group_id,
        mode,
    })
}

fn normalize_publish_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "on_change" | "change_only" | "changed_only" => MQTT_PUBLISH_MODE_ON_CHANGE,
        _ => MQTT_PUBLISH_MODE_SCAN_INTERVAL,
    }
}

fn read_publish_mode_map(
    settings: &serde_json::Value,
    key: &str,
) -> HashMap<String, String> {
    settings
        .get(key)
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    v.as_str()
                        .map(|mode| (k.clone(), normalize_publish_mode(mode).to_string()))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default()
}

fn serialize_publish_mode_map(map: &HashMap<String, String>) -> serde_json::Value {
    serde_json::Value::Object(
        map.iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::Value::String(normalize_publish_mode(v).to_string()),
                )
            })
            .collect(),
    )
}

fn upsert_publish_mode_entry(
    settings: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    entry_key: &str,
    mode: &str,
) -> Result<(), ErrorResponse> {
    let target = settings
        .entry(key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let object = target.as_object_mut().ok_or_else(|| {
        ErrorResponse::invalid_input(format!("{} must be an object", key))
    })?;
    object.insert(
        entry_key.to_string(),
        serde_json::Value::String(normalize_publish_mode(mode).to_string()),
    );
    Ok(())
}
