use crate::commands::dto::{ErrorResponse, SaveDriverRequest};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::Tag;

pub(super) fn build_driver_config(
    req: &SaveDriverRequest,
    driver_id: String,
    _existing: Option<&DriverConfig>,
    password_key: Option<String>,
) -> DriverConfig {
    let mut settings = serde_json::Map::new();
    settings.insert(
        "host".to_string(),
        serde_json::Value::String(req.host.clone()),
    );
    settings.insert("port".to_string(), serde_json::Value::from(req.port));
    settings.insert(
        "database".to_string(),
        serde_json::Value::String(req.database.clone()),
    );
    settings.insert(
        "username".to_string(),
        serde_json::Value::String(req.username.clone()),
    );

    if let Some(password_key) = password_key {
        settings.insert(
            "password_key".to_string(),
            serde_json::Value::String(password_key),
        );
    }

    settings.insert(
        "tls_enabled".to_string(),
        serde_json::Value::Bool(req.tls_enabled),
    );

    if let Some(value) = req
        .tls_ca_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        settings.insert(
            "tls_ca_path".to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
    if let Some(value) = req
        .tls_client_cert_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        settings.insert(
            "tls_client_cert_path".to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
    if let Some(value) = req
        .tls_client_key_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        settings.insert(
            "tls_client_key_path".to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
    if let Some(connect_timeout_ms) = req.connect_timeout_ms {
        settings.insert(
            "connect_timeout_ms".to_string(),
            serde_json::Value::from(connect_timeout_ms),
        );
    }
    if let Some(statement_timeout_ms) = req.statement_timeout_ms {
        settings.insert(
            "statement_timeout_ms".to_string(),
            serde_json::Value::from(statement_timeout_ms),
        );
    }
    settings.insert(
        "auto_restart".to_string(),
        serde_json::Value::Bool(req.auto_restart),
    );
    if let Some(max_restart_per_minute) = req.max_restart_per_minute {
        settings.insert(
            "max_restart_per_minute".to_string(),
            serde_json::Value::from(max_restart_per_minute),
        );
    }

    DriverConfig {
        id: driver_id,
        driver_type: req.driver_type.clone(),
        enabled: Some(req.enabled),
        settings: serde_json::Value::Object(settings),
    }
}

pub(super) fn build_renamed_tags(
    existing_registry_tags: &[Tag],
    original_driver_id: &str,
    new_driver_id: &str,
) -> Vec<TagConfig> {
    existing_registry_tags
        .iter()
        .map(|tag| TagConfig {
            id: tag.id.0.clone(),
            name: tag.name.clone(),
            data_type: tag.data_type.as_str().to_string(),
            driver: if tag.driver_id == original_driver_id {
                new_driver_id.to_string()
            } else {
                tag.driver_id.clone()
            },
            scan_group: tag.scan_group_id.clone(),
            driver_spec: tag.driver_spec.clone(),
            enabled: Some(true),
            metadata: tag.metadata.clone(),
        })
        .collect()
}

pub(super) fn build_renamed_scan_groups(
    existing_scan_groups: Vec<ScanGroupConfig>,
    original_driver_id: &str,
    new_driver_id: &str,
) -> Vec<ScanGroupConfig> {
    existing_scan_groups
        .into_iter()
        .map(|mut scan_group| {
            if scan_group.driver == original_driver_id {
                scan_group.driver = new_driver_id.to_string();
            }
            scan_group
        })
        .collect()
}

pub(super) fn merge_driver_configs_for_rename(
    existing_configs: Vec<DriverConfig>,
    original_driver_id: &str,
    new_config: &DriverConfig,
) -> Vec<DriverConfig> {
    let mut final_driver_configs = Vec::with_capacity(existing_configs.len() + 1);
    let mut replaced = false;

    for config in existing_configs {
        if config.id == original_driver_id {
            final_driver_configs.push(new_config.clone());
            replaced = true;
        } else if config.id != new_config.id {
            final_driver_configs.push(config);
        }
    }

    if !replaced {
        final_driver_configs.push(new_config.clone());
    }

    final_driver_configs
}

pub(super) fn ensure_non_empty_driver_id(driver_id: &str) -> Result<(), ErrorResponse> {
    if driver_id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("Driver ID cannot be empty"));
    }
    Ok(())
}
