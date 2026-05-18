// タグ操作のコマンド層

use super::dto::{CreateTagRequest, ErrorResponse, ScanGroupDto, TagDto};
use crate::app_state::AppState;
use crate::commands::driver::toml_io::write_tags_toml_atomic;
use crate::config::TagConfig;
use crate::core::{DataType, Tag, TagId};
use std::str::FromStr;

#[tauri::command]
pub async fn create_tag(
    state: tauri::State<'_, AppState>,
    req: CreateTagRequest,
) -> Result<TagDto, ErrorResponse> {
    if req.id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("Tag ID cannot be empty"));
    }
    if req.name.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("Tag name cannot be empty"));
    }
    if req.driver_id.trim().is_empty() || req.scan_group_id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input(
            "driver_id and scan_group_id are required",
        ));
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
    let metrics = state.scan_group_runtime_metrics.read().await;
    Ok(scan_groups
        .iter()
        .filter(|group| {
            driver_id
                .as_ref()
                .map(|target| &group.driver == target)
                .unwrap_or(true)
        })
        .map(|group| {
            let key = format!("{}::{}", group.driver, group.id);
            let metric = metrics.get(&key);
            let cycle_delta_ratio = metric.and_then(|m| m.cycle_delta_ratio);
            let cycle_status = cycle_delta_ratio.map(|ratio| {
                if ratio <= 0.10 {
                    "ok".to_string()
                } else if ratio <= 0.30 {
                    "warn".to_string()
                } else {
                    "danger".to_string()
                }
            });

            ScanGroupDto {
                id: group.id.clone(),
                driver_id: group.driver.clone(),
                table: group.table.clone(),
                timestamp_column: group.timestamp_column.clone(),
                scan_rate_ms: Some(group.scan_rate_ms),
                observed_cycle_ms: metric.and_then(|m| m.last_cycle_ms),
                observed_p95_cycle_ms: metric.and_then(|m| m.p95_cycle_ms),
                cycle_delta_ratio,
                cycle_status,
            }
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
        return Err(ErrorResponse::not_found(format!("Tag not found: {}", tag_id)));
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
        .ok_or(ErrorResponse::invalid_input(format!(
            "Unknown scan_group_id: {}",
            req.scan_group_id
        )))?;

    if scan_group.driver != req.driver_id {
        return Err(ErrorResponse::invalid_input(format!(
            "scan_group {} does not belong to driver {}",
            req.scan_group_id, req.driver_id
        )));
    }

    let tags = state.registry.list_all().await;
    if tags.iter().any(|tag| tag.id.0 == req.id) {
        return Err(ErrorResponse::validation_error(format!(
            "Duplicate tag id: {}",
            req.id
        )));
    }

    if tags.iter().any(|tag| {
        tag.id.0 != req.id
            && tag.driver_id == req.driver_id
            && tag.scan_group_id == req.scan_group_id
            && tag.name == req.name
    }) {
        return Err(ErrorResponse::validation_error(format!(
            "Duplicate tag name in same scan group: {} ({}/{})",
            req.name, req.driver_id, req.scan_group_id
        )));
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
