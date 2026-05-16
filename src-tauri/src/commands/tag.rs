// タグ操作のコマンド層

use super::dto::{CreateTagRequest, ErrorResponse, ScanGroupDto, TagDto};
use crate::app_state::AppState;
use crate::config::{ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use std::str::FromStr;

#[tauri::command]
pub async fn create_tag(
    state: tauri::State<'_, AppState>,
    req: CreateTagRequest,
) -> Result<TagDto, ErrorResponse> {
    if req.id.trim().is_empty() {
        return Err(ErrorResponse {
            error: "Tag ID cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }
    if req.name.trim().is_empty() {
        return Err(ErrorResponse {
            error: "Tag name cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }
    if req.driver_id.trim().is_empty() || req.scan_group_id.trim().is_empty() {
        return Err(ErrorResponse {
            error: "driver_id and scan_group_id are required".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    validate_tag_request(&state, &req).await?;

    let data_type = DataType::from_str(&req.data_type).map_err(ErrorResponse::from)?;
    let existing = state.registry.get(&TagId(req.id.clone())).await;
    let tag = Tag {
        id: TagId(req.id.clone()),
        name: req.name.clone(),
        data_type,
        driver_id: req.driver_id.clone(),
        scan_group_id: req.scan_group_id.clone(),
        driver_spec: req.driver_spec.clone(),
        metadata: existing.and_then(|tag| tag.metadata),
    };
    state.registry.insert(tag).await;

    persist_all_tags(&state).await?;

    Ok(TagDto {
        id: req.id,
        name: req.name,
        data_type: req.data_type,
        driver_id: req.driver_id,
        scan_group_id: req.scan_group_id,
        driver_spec: req.driver_spec,
    })
}

#[tauri::command]
pub async fn list_tags(state: tauri::State<'_, AppState>) -> Result<Vec<TagDto>, ErrorResponse> {
    let tags = state.registry.list_all().await;
    Ok(tags
        .into_iter()
        .map(|tag| TagDto {
            id: tag.id.0,
            name: tag.name,
            data_type: tag.data_type.as_str().to_string(),
            driver_id: tag.driver_id,
            scan_group_id: tag.scan_group_id,
            driver_spec: tag.driver_spec,
        })
        .collect())
}

#[tauri::command]
pub async fn list_scan_groups(
    state: tauri::State<'_, AppState>,
    driver_id: Option<String>,
) -> Result<Vec<ScanGroupDto>, ErrorResponse> {
    let scan_groups = state.scan_groups.read().await;
    Ok(scan_groups
        .iter()
        .filter(|group| {
            driver_id
                .as_ref()
                .map(|target| &group.driver == target)
                .unwrap_or(true)
        })
        .map(|group| ScanGroupDto {
            id: group.id.clone(),
            driver_id: group.driver.clone(),
            table: group.table.clone(),
            timestamp_column: group.timestamp_column.clone(),
            scan_rate_ms: Some(group.scan_rate_ms),
        })
        .collect())
}

#[tauri::command]
pub async fn delete_tag(
    state: tauri::State<'_, AppState>,
    tag_id: String,
) -> Result<(), ErrorResponse> {
    let removed = state.registry.remove(&TagId(tag_id.clone())).await;
    if removed.is_none() {
        return Err(ErrorResponse {
            error: format!("Tag not found: {}", tag_id),
            code: "NOT_FOUND".to_string(),
        });
    }

    persist_all_tags(&state).await?;
    Ok(())
}

async fn validate_tag_request(
    state: &tauri::State<'_, AppState>,
    req: &CreateTagRequest,
) -> Result<(), ErrorResponse> {
    let scan_groups = state.scan_groups.read().await;
    let scan_group = scan_groups
        .iter()
        .find(|group| group.id == req.scan_group_id)
        .ok_or(ErrorResponse {
            error: format!("Unknown scan_group_id: {}", req.scan_group_id),
            code: "INVALID_INPUT".to_string(),
        })?;

    if scan_group.driver != req.driver_id {
        return Err(ErrorResponse {
            error: format!(
                "scan_group {} does not belong to driver {}",
                req.scan_group_id, req.driver_id
            ),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let tags = state.registry.list_all().await;
    if tags.iter().any(|tag| tag.id.0 == req.id) {
        return Err(ErrorResponse {
            error: format!("Duplicate tag id: {}", req.id),
            code: "VALIDATION_ERROR".to_string(),
        });
    }

    if tags.iter().any(|tag| {
        tag.id.0 != req.id
            && tag.driver_id == req.driver_id
            && tag.scan_group_id == req.scan_group_id
            && tag.name == req.name
    }) {
        return Err(ErrorResponse {
            error: format!(
                "Duplicate tag name in same scan group: {} ({}/{})",
                req.name, req.driver_id, req.scan_group_id
            ),
            code: "VALIDATION_ERROR".to_string(),
        });
    }

    Ok(())
}

async fn persist_all_tags(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
    let tags = state.registry.list_all().await;
    let scan_groups = state.scan_groups.read().await.clone();

    let tag_configs: Vec<TagConfig> = tags
        .into_iter()
        .map(|tag| TagConfig {
            id: tag.id.0,
            name: tag.name,
            data_type: tag.data_type.as_str().to_string(),
            driver: tag.driver_id,
            scan_group: tag.scan_group_id,
            driver_spec: tag.driver_spec,
            enabled: Some(true),
            metadata: tag.metadata,
        })
        .collect();

    write_tags_toml_atomic(&scan_groups, &tag_configs)
}

#[derive(serde::Serialize)]
struct TagsTomlFile {
    #[serde(rename = "scan_group")]
    scan_group: Vec<ScanGroupConfig>,
    #[serde(rename = "tag")]
    tag: Vec<TagConfig>,
}

fn write_tags_toml_atomic(
    scan_groups: &[ScanGroupConfig],
    tags: &[TagConfig],
) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    let tags_path = config_dir.join("tags.toml");
    let tmp_path = config_dir.join("tags.toml.tmp");

    let toml_text = toml::to_string_pretty(&TagsTomlFile {
        scan_group: scan_groups.to_vec(),
        tag: tags.to_vec(),
    })
    .map_err(|e| ErrorResponse {
        error: format!("Failed to serialize tags.toml: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write tags.toml.tmp: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    if tags_path.exists() {
        let _ = std::fs::remove_file(&tags_path);
    }

    if let Err(rename_error) = std::fs::rename(&tmp_path, &tags_path) {
        std::fs::write(&tags_path, &toml_text).map_err(|write_error| ErrorResponse {
            error: format!(
                "Failed to replace tags.toml (rename: {}; write fallback: {})",
                rename_error, write_error
            ),
            code: "IO_ERROR".to_string(),
        })?;
        let _ = std::fs::remove_file(&tmp_path);
    }

    Ok(())
}

fn resolve_config_dir() -> std::path::PathBuf {
    let relative = std::path::PathBuf::from("../config");
    if relative.exists() {
        return relative;
    }
    std::path::PathBuf::from("config")
}
