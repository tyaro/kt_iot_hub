use crate::app_state::{AppState, DriverUiSessionState};
use crate::commands::dto::{ErrorResponse, ImportDriverUiResultRequest};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::DataType;
use kt_driver_ui_protocol::{DriverUiDriverPayload, DriverUiImportPayload};
use std::collections::HashSet;
use std::str::FromStr;

pub(super) fn validate_import_request(
    req: &ImportDriverUiResultRequest,
) -> Result<(), ErrorResponse> {
    if req.session_id.trim().is_empty() {
        return Err(ErrorResponse {
            error: "session_id cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }
    if req.output_json_path.trim().is_empty() {
        return Err(ErrorResponse {
            error: "output_json_path cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }
    Ok(())
}

pub(super) async fn validate_and_convert_payload(
    state: &tauri::State<'_, AppState>,
    session: &DriverUiSessionState,
    payload: DriverUiImportPayload,
) -> Result<(DriverConfig, Vec<ScanGroupConfig>, Vec<TagConfig>), ErrorResponse> {
    if payload.schema_version.unwrap_or(1) != 1 {
        return Err(ErrorResponse {
            error: "Unsupported schemaVersion".to_string(),
            code: "SCHEMA_MISMATCH".to_string(),
        });
    }

    let driver_payload = payload.driver;
    let effective_driver_id = driver_payload.id.clone();

    if let Some(session_driver_id) = session.target_driver_id.as_ref() {
        if effective_driver_id != *session_driver_id {
            return Err(ErrorResponse {
                error: format!(
                    "driver_id mismatch (payload={}, expected={})",
                    effective_driver_id, session_driver_id
                ),
                code: "VALIDATION_ERROR".to_string(),
            });
        }
    }

    let effective_driver_type = driver_payload.driver_type.clone();
    let driver_config =
        build_import_driver_config(&effective_driver_id, &effective_driver_type, &driver_payload)?;

    let mut scan_group_ids = HashSet::new();
    let mut new_scan_groups = Vec::new();
    let mut all_new_tags = Vec::new();

    for sg in &driver_payload.scan_groups {
        if !scan_group_ids.insert(sg.id.clone()) {
            return Err(ErrorResponse {
                error: format!("Duplicate scan_group id: {}", sg.id),
                code: "VALIDATION_ERROR".to_string(),
            });
        }

        new_scan_groups.push(ScanGroupConfig {
            id: sg.id.clone(),
            driver: effective_driver_id.clone(),
            scan_rate_ms: sg.scan_rate_ms.unwrap_or(1000),
            schema: sg.schema.clone(),
            table: sg.table.clone(),
            timestamp_column: sg.timestamp_column.clone(),
            node: sg.node.clone(),
        });

        all_new_tags.reserve(sg.tags.len());
    }

    let existing_tags = state.registry.list_all().await;
    let mut existing_tag_ids: HashSet<String> = existing_tags
        .iter()
        .filter(|t| t.driver_id != effective_driver_id)
        .map(|t| t.id.0.clone())
        .collect();

    let mut local_tag_ids = HashSet::new();
    let mut local_name_keys = HashSet::new();

    for sg in &driver_payload.scan_groups {
        for t in &sg.tags {
            if !local_tag_ids.insert(t.id.clone()) || !existing_tag_ids.insert(t.id.clone()) {
                return Err(ErrorResponse {
                    error: format!("Duplicate tag id: {}", t.id),
                    code: "VALIDATION_ERROR".to_string(),
                });
            }

            let scan_group_id = sg.id.clone();
            let name_key = format!("{}:{}:{}", effective_driver_id, scan_group_id, t.name);
            if !local_name_keys.insert(name_key) {
                return Err(ErrorResponse {
                    error: format!(
                        "Duplicate tag name in same scanGroup: {} (scanGroup={})",
                        t.name, scan_group_id
                    ),
                    code: "VALIDATION_ERROR".to_string(),
                });
            }

            DataType::from_str(&t.data_type).map_err(ErrorResponse::from)?;

            let metadata = build_metadata(&t.unit, &t.comment);
            all_new_tags.push(TagConfig {
                id: t.id.clone(),
                name: t.name.clone(),
                data_type: t.data_type.clone(),
                driver: effective_driver_id.clone(),
                scan_group: scan_group_id,
                driver_spec: t.driver_spec.clone(),
                enabled: t.enabled,
                metadata,
            });
        }
    }

    Ok((driver_config, new_scan_groups, all_new_tags))
}

fn build_metadata(unit: &Option<String>, comment: &Option<String>) -> Option<serde_json::Value> {
    let mut map = serde_json::Map::new();

    if let Some(u) = unit {
        if !u.trim().is_empty() {
            map.insert("unit".to_string(), serde_json::Value::String(u.clone()));
        }
    }

    if let Some(c) = comment {
        if !c.trim().is_empty() {
            map.insert("comment".to_string(), serde_json::Value::String(c.clone()));
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(map))
    }
}

fn build_import_driver_config(
    driver_id: &str,
    driver_type: &str,
    driver_payload: &DriverUiDriverPayload,
) -> Result<DriverConfig, ErrorResponse> {
    if driver_payload.id != driver_id {
        return Err(ErrorResponse {
            error: format!(
                "driver.id mismatch (driver.id={}, expected={})",
                driver_payload.id, driver_id
            ),
            code: "VALIDATION_ERROR".to_string(),
        });
    }

    if driver_payload.driver_type != driver_type {
        return Err(ErrorResponse {
            error: format!(
                "driver.driverType mismatch (driver.driverType={}, expected={})",
                driver_payload.driver_type, driver_type
            ),
            code: "VALIDATION_ERROR".to_string(),
        });
    }

    let settings = merge_driver_settings(driver_payload.settings.clone(), driver_payload.extra.clone());
    Ok(DriverConfig {
        id: driver_payload.id.clone(),
        driver_type: driver_payload.driver_type.clone(),
        enabled: Some(driver_payload.enabled.unwrap_or(true)),
        settings,
    })
}

fn merge_driver_settings(
    settings: Option<serde_json::Value>,
    extra: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut merged = match settings {
        Some(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };

    for (key, value) in extra {
        if key != "id"
            && key != "driverType"
            && key != "driverKind"
            && key != "enabled"
            && key != "registration_ui_path"
            && key != "driver_ui_path"
        {
            merged.insert(key, value);
        }
    }

    serde_json::Value::Object(merged)
}
