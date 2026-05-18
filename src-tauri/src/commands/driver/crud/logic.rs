use crate::commands::dto::{ErrorResponse, SaveDriverRequest};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::Tag;

pub(super) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    crate::commands::util::normalize_optional_string(value)
}

pub(super) fn build_driver_config(
    req: &SaveDriverRequest,
    driver_id: String,
    existing: Option<&DriverConfig>,
) -> DriverConfig {
    let password = if req.password.trim().is_empty() {
        existing
            .and_then(|cfg| cfg.settings.get("password"))
            .cloned()
            .unwrap_or_else(|| serde_json::Value::String(String::new()))
    } else {
        serde_json::Value::String(req.password.clone())
    };

    let settings = serde_json::Map::from_iter([
        (
            "host".to_string(),
            serde_json::Value::String(req.host.clone()),
        ),
        ("port".to_string(), serde_json::Value::from(req.port)),
        (
            "database".to_string(),
            serde_json::Value::String(req.database.clone()),
        ),
        (
            "username".to_string(),
            serde_json::Value::String(req.username.clone()),
        ),
        ("password".to_string(), password),
    ]);

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
