//! ドライバUI 結果 JSON のバリデーションと、本体状態への取り込み処理。

use super::runtime_sync::sync_driver_runtime;
use super::toml_io::{write_drivers_toml_atomic, write_tags_toml_atomic};
use super::ui_paths::normalize_optional_string;
use crate::app_state::{AppState, DriverUiSessionState};
use crate::commands::dto::{ErrorResponse, ImportDriverUiResultRequest, ImportDriverUiResultResponse};
use crate::commands::driver_ui_protocol::{
    DriverUiDriverPayload, DriverUiImportPayload,
};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use std::collections::HashSet;
use std::str::FromStr;

#[tauri::command]
pub async fn import_driver_ui_result(
    state: tauri::State<'_, AppState>,
    req: ImportDriverUiResultRequest,
) -> Result<ImportDriverUiResultResponse, ErrorResponse> {
    validate_import_request(&req)?;

    {
        let imported = state.imported_driver_ui_sessions.read().await;
        if imported.contains(&req.session_id) {
            return Err(ErrorResponse {
                error: format!("Session already imported: {}", req.session_id),
                code: "DUPLICATE_IMPORT".to_string(),
            });
        }
    }

    let session = {
        let sessions = state.active_driver_ui_sessions.read().await;
        sessions.get(&req.session_id).cloned().ok_or(ErrorResponse {
            error: format!("No active driver UI session: {}", req.session_id),
            code: "NO_ACTIVE_SESSION".to_string(),
        })?
    };

    if let Some(request_driver_id) = req
        .driver_id
        .as_ref()
        .and_then(|value| normalize_optional_string(Some(value.clone())))
    {
        if let Some(session_driver_id) = session.target_driver_id.as_ref() {
            if request_driver_id != *session_driver_id {
                return Err(ErrorResponse {
                    error: format!(
                        "Session mismatch for driver {} (expected={}, got={})",
                        request_driver_id, session_driver_id, request_driver_id
                    ),
                    code: "SESSION_MISMATCH".to_string(),
                });
            }
        }
    }

    let output_path = std::path::PathBuf::from(&req.output_json_path);
    if !output_path.exists() {
        return Err(ErrorResponse {
            error: format!("Result file not found: {}", req.output_json_path),
            code: "RESULT_NOT_READY".to_string(),
        });
    }

    let json_text = std::fs::read_to_string(&output_path).map_err(|e| ErrorResponse {
        error: format!("Failed to read result file: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    let payload: DriverUiImportPayload =
        serde_json::from_str(&json_text).map_err(|e| ErrorResponse {
            error: format!("Failed to parse driver UI JSON: {}", e),
            code: "INVALID_JSON".to_string(),
        })?;

    let (driver_config, new_scan_groups, new_tags) =
        validate_and_convert_payload(&state, &session, payload).await?;

    apply_driver_import(&state, &driver_config, &new_scan_groups, &new_tags).await?;

    {
        let mut imported = state.imported_driver_ui_sessions.write().await;
        imported.insert(req.session_id.clone());
    }
    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        sessions.remove(&req.session_id);
    }

    Ok(ImportDriverUiResultResponse {
        driver_id: driver_config.id,
        session_id: req.session_id,
        imported_tag_count: new_tags.len(),
        imported_scan_group_count: new_scan_groups.len(),
    })
}

fn validate_import_request(req: &ImportDriverUiResultRequest) -> Result<(), ErrorResponse> {
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

async fn validate_and_convert_payload(
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
        build_import_driver_config(&effective_driver_id, &effective_driver_type, driver_payload)?;

    let mut scan_group_ids = HashSet::new();
    let mut new_scan_groups = Vec::with_capacity(payload.scan_groups.len());
    for sg in payload.scan_groups {
        if !scan_group_ids.insert(sg.id.clone()) {
            return Err(ErrorResponse {
                error: format!("Duplicate scan_group id: {}", sg.id),
                code: "VALIDATION_ERROR".to_string(),
            });
        }
        new_scan_groups.push(ScanGroupConfig {
            id: sg.id,
            driver: effective_driver_id.clone(),
            scan_rate_ms: sg.scan_rate_ms.unwrap_or(1000),
            schema: sg.schema,
            table: sg.table,
            timestamp_column: sg.timestamp_column,
            node: sg.node,
        });
    }

    let existing_tags = state.registry.list_all().await;
    let mut existing_tag_ids: HashSet<String> = existing_tags
        .iter()
        .filter(|t| t.driver_id != effective_driver_id)
        .map(|t| t.id.0.clone())
        .collect();

    let mut local_tag_ids = HashSet::new();
    let mut local_name_keys = HashSet::new();
    let mut new_tags = Vec::with_capacity(payload.tags.len());

    for t in payload.tags {
        if !local_tag_ids.insert(t.id.clone()) || !existing_tag_ids.insert(t.id.clone()) {
            return Err(ErrorResponse {
                error: format!("Duplicate tag id: {}", t.id),
                code: "VALIDATION_ERROR".to_string(),
            });
        }

        let scan_group_id = t
            .driver_spec
            .get("scanGroup")
            .and_then(|v| v.as_str())
            .ok_or(ErrorResponse {
                error: format!("Tag {} missing driverSpec.scanGroup", t.id),
                code: "VALIDATION_ERROR".to_string(),
            })?
            .to_string();

        if !scan_group_ids.contains(&scan_group_id) {
            return Err(ErrorResponse {
                error: format!("Tag {} references unknown scanGroup {}", t.id, scan_group_id),
                code: "VALIDATION_ERROR".to_string(),
            });
        }

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
        new_tags.push(TagConfig {
            id: t.id,
            name: t.name,
            data_type: t.data_type,
            driver: effective_driver_id.clone(),
            scan_group: scan_group_id,
            driver_spec: t.driver_spec,
            enabled: t.enabled,
            metadata,
        });
    }

    Ok((driver_config, new_scan_groups, new_tags))
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

async fn apply_driver_import(
    state: &tauri::State<'_, AppState>,
    driver_config: &DriverConfig,
    new_scan_groups: &[ScanGroupConfig],
    new_tags: &[TagConfig],
) -> Result<(), ErrorResponse> {
    let driver_id = driver_config.id.as_str();

    let existing_registry_tags = state.registry.list_all().await;
    let mut final_tags_for_file: Vec<TagConfig> = existing_registry_tags
        .iter()
        .filter(|t| t.driver_id != driver_id)
        .map(|t| TagConfig {
            id: t.id.0.clone(),
            name: t.name.clone(),
            data_type: t.data_type.as_str().to_string(),
            driver: t.driver_id.clone(),
            scan_group: t.scan_group_id.clone(),
            driver_spec: t.driver_spec.clone(),
            enabled: Some(true),
            metadata: t.metadata.clone(),
        })
        .collect();
    final_tags_for_file.extend_from_slice(new_tags);

    let existing_scan_groups = state.scan_groups.read().await.clone();
    let mut final_scan_groups: Vec<ScanGroupConfig> = existing_scan_groups
        .into_iter()
        .filter(|sg| sg.driver != driver_id)
        .collect();
    final_scan_groups.extend_from_slice(new_scan_groups);

    let final_driver_configs = {
        let existing_configs = state.driver_configs.read().await.clone();
        let mut configs: Vec<DriverConfig> = existing_configs
            .into_iter()
            .filter(|cfg| cfg.id != driver_id)
            .collect();
        configs.push(driver_config.clone());
        configs
    };

    write_tags_toml_atomic(&final_scan_groups, &final_tags_for_file)?;
    write_drivers_toml_atomic(&final_driver_configs)?;

    {
        let mut scan_groups = state.scan_groups.write().await;
        *scan_groups = final_scan_groups;
    }
    {
        let mut driver_configs = state.driver_configs.write().await;
        *driver_configs = final_driver_configs;
    }

    sync_driver_runtime(state, driver_config).await?;

    for tag in existing_registry_tags
        .into_iter()
        .filter(|t| t.driver_id == driver_id)
    {
        let _ = state.registry.remove(&tag.id).await;
    }
    for tag_cfg in new_tags {
        let data_type = DataType::from_str(&tag_cfg.data_type).map_err(ErrorResponse::from)?;
        state
            .registry
            .insert(Tag {
                id: TagId(tag_cfg.id.clone()),
                name: tag_cfg.name.clone(),
                data_type,
                driver_id: tag_cfg.driver.clone(),
                scan_group_id: tag_cfg.scan_group.clone(),
                driver_spec: tag_cfg.driver_spec.clone(),
                metadata: tag_cfg.metadata.clone(),
            })
            .await;
    }

    Ok(())
}

fn build_import_driver_config(
    driver_id: &str,
    driver_type: &str,
    driver_payload: DriverUiDriverPayload,
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

    let settings = merge_driver_settings(driver_payload.settings, driver_payload.extra);
    Ok(DriverConfig {
        id: driver_payload.id,
        driver_type: driver_payload.driver_type,
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
