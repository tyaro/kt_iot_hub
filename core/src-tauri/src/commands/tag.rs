// タグ操作のコマンド層

use super::dto::{
    BulkUpdateDriverScanGroupRateRequest, BulkUpdateScanGroupsResultDto, CreateTagRequest,
    ErrorResponse, ScanGroupDto, TagDto, UpdateScanGroupRateRequest,
};
use crate::app_state::AppState;
use crate::commands::driver::runtime_sync::sync_driver_runtime;
use crate::commands::driver::toml_io::write_tags_toml_atomic;
use crate::config::{ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use std::str::FromStr;

const MIN_SCAN_RATE_MS: u32 = 100;

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
        .map(|group| build_scan_group_dto(group, metrics.get(&scan_group_metric_key(group))))
        .collect())
}

#[tauri::command]
pub async fn update_scan_group_rate(
    state: tauri::State<'_, AppState>,
    req: UpdateScanGroupRateRequest,
) -> Result<ScanGroupDto, ErrorResponse> {
    validate_scan_rate_update_request(&req.driver_id, req.scan_rate_ms)?;

    let updated_group = {
        let existing_scan_groups = state.scan_groups.read().await.clone();
        let (updated_scan_groups, updated_group) = update_single_scan_group_rate_internal(
            &existing_scan_groups,
            &req.driver_id,
            &req.scan_group_id,
            req.scan_rate_ms,
        )?;

        let tags = build_tag_configs_from_registry(&state).await;
        write_tags_toml_atomic(&updated_scan_groups, &tags)?;

        {
            let mut scan_groups = state.scan_groups.write().await;
            *scan_groups = updated_scan_groups;
        }

        refresh_scan_group_metric_expectations(&state, std::slice::from_ref(&updated_group)).await;
        sync_driver_runtime_for_scan_group_change(&state, &req.driver_id).await?;
        updated_group
    };

    let metrics = state.scan_group_runtime_metrics.read().await;
    Ok(build_scan_group_dto(
        &updated_group,
        metrics.get(&scan_group_metric_key(&updated_group)),
    ))
}

#[tauri::command]
pub async fn bulk_update_driver_scan_group_rate(
    state: tauri::State<'_, AppState>,
    req: BulkUpdateDriverScanGroupRateRequest,
) -> Result<BulkUpdateScanGroupsResultDto, ErrorResponse> {
    validate_scan_rate_update_request(&req.driver_id, req.scan_rate_ms)?;

    let updated_count = {
        let existing_scan_groups = state.scan_groups.read().await.clone();
        let (updated_scan_groups, updated_driver_groups) =
            update_driver_scan_group_rate_internal(
                &existing_scan_groups,
                &req.driver_id,
                req.scan_rate_ms,
            )?;

        let tags = build_tag_configs_from_registry(&state).await;
        write_tags_toml_atomic(&updated_scan_groups, &tags)?;

        {
            let mut scan_groups = state.scan_groups.write().await;
            *scan_groups = updated_scan_groups;
        }

        refresh_scan_group_metric_expectations(&state, &updated_driver_groups).await;
        sync_driver_runtime_for_scan_group_change(&state, &req.driver_id).await?;
        updated_driver_groups.len() as u32
    };

    Ok(BulkUpdateScanGroupsResultDto {
        driver_id: req.driver_id,
        updated_count,
        scan_rate_ms: req.scan_rate_ms,
    })
}

#[tauri::command]
pub async fn delete_tag(
    state: tauri::State<'_, AppState>,
    tag_id: String,
) -> Result<(), ErrorResponse> {
    let removed = state.registry.remove(&TagId(tag_id.clone())).await;
    if removed.is_none() {
        return Err(ErrorResponse::not_found(format!(
            "Tag not found: {}",
            tag_id
        )));
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
    let scan_groups = state.scan_groups.read().await.clone();
    let tag_configs = build_tag_configs_from_registry(state).await;

    write_tags_toml_atomic(&scan_groups, &tag_configs)
}

async fn build_tag_configs_from_registry(state: &tauri::State<'_, AppState>) -> Vec<TagConfig> {
    state
        .registry
        .list_all()
        .await
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
        .collect()
}

fn build_scan_group_dto(
    group: &ScanGroupConfig,
    metric: Option<&crate::app_state::ScanGroupRuntimeMetricState>,
) -> ScanGroupDto {
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
}

fn scan_group_metric_key(group: &ScanGroupConfig) -> String {
    format!("{}::{}", group.driver, group.id)
}

fn validate_scan_rate_update_request(
    driver_id: &str,
    scan_rate_ms: u32,
) -> Result<(), ErrorResponse> {
    if driver_id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("driver_id cannot be empty"));
    }
    if scan_rate_ms < MIN_SCAN_RATE_MS {
        return Err(ErrorResponse::invalid_input(format!(
            "scan_rate_ms must be greater than or equal to {}",
            MIN_SCAN_RATE_MS
        )));
    }
    Ok(())
}

fn update_single_scan_group_rate_internal(
    existing_scan_groups: &[ScanGroupConfig],
    driver_id: &str,
    scan_group_id: &str,
    scan_rate_ms: u32,
) -> Result<(Vec<ScanGroupConfig>, ScanGroupConfig), ErrorResponse> {
    if scan_group_id.trim().is_empty() {
        return Err(ErrorResponse::invalid_input("scan_group_id cannot be empty"));
    }

    let mut found = None;
    let updated_scan_groups = existing_scan_groups
        .iter()
        .cloned()
        .map(|mut group| {
            if group.driver == driver_id && group.id == scan_group_id {
                group.scan_rate_ms = scan_rate_ms;
                found = Some(group.clone());
            }
            group
        })
        .collect::<Vec<_>>();

    match found {
        Some(group) => Ok((updated_scan_groups, group)),
        None => Err(ErrorResponse::not_found(format!(
            "Scan group not found: {}/{}",
            driver_id, scan_group_id
        ))),
    }
}

fn update_driver_scan_group_rate_internal(
    existing_scan_groups: &[ScanGroupConfig],
    driver_id: &str,
    scan_rate_ms: u32,
) -> Result<(Vec<ScanGroupConfig>, Vec<ScanGroupConfig>), ErrorResponse> {
    let mut updated_groups = Vec::new();
    let updated_scan_groups = existing_scan_groups
        .iter()
        .cloned()
        .map(|mut group| {
            if group.driver == driver_id {
                group.scan_rate_ms = scan_rate_ms;
                updated_groups.push(group.clone());
            }
            group
        })
        .collect::<Vec<_>>();

    if updated_groups.is_empty() {
        return Err(ErrorResponse::not_found(format!(
            "No scan groups found for driver: {}",
            driver_id
        )));
    }

    Ok((updated_scan_groups, updated_groups))
}

async fn refresh_scan_group_metric_expectations(
    state: &tauri::State<'_, AppState>,
    updated_groups: &[ScanGroupConfig],
) {
    let mut metrics = state.scan_group_runtime_metrics.write().await;
    for group in updated_groups {
        if let Some(metric) = metrics.get_mut(&scan_group_metric_key(group)) {
            metric.expected_scan_rate_ms = Some(group.scan_rate_ms);
        }
    }
}

async fn sync_driver_runtime_for_scan_group_change(
    state: &tauri::State<'_, AppState>,
    driver_id: &str,
) -> Result<(), ErrorResponse> {
    let driver_config = {
        let configs = state.driver_configs.read().await;
        configs.iter().find(|cfg| cfg.id == driver_id).cloned()
    }
    .ok_or_else(|| ErrorResponse::not_found(format!("Driver not found: {}", driver_id)))?;

    sync_driver_runtime(state, &driver_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_group(driver: &str, id: &str, scan_rate_ms: u32) -> ScanGroupConfig {
        ScanGroupConfig {
            id: id.to_string(),
            driver: driver.to_string(),
            scan_rate_ms,
            schema: None,
            table: None,
            timestamp_column: None,
            node: None,
        }
    }

    #[test]
    fn single_scan_group_rate_update_uses_driver_and_group_id() {
        let groups = vec![sample_group("driver-a", "group-1", 1000)];

        let (_, updated) =
            update_single_scan_group_rate_internal(&groups, "driver-a", "group-1", 2500)
                .expect("single update should succeed");

        assert_eq!(updated.scan_rate_ms, 2500);
    }

    #[test]
    fn single_scan_group_rate_update_fails_when_target_missing() {
        let groups = vec![sample_group("driver-a", "group-1", 1000)];

        let error =
            update_single_scan_group_rate_internal(&groups, "driver-b", "group-1", 2500)
                .expect_err("missing target should fail");

        assert_eq!(error.code, "NOT_FOUND");
    }

    #[test]
    fn bulk_scan_group_rate_update_updates_only_target_driver() {
        let groups = vec![
            sample_group("driver-a", "group-1", 1000),
            sample_group("driver-a", "group-2", 1000),
            sample_group("driver-b", "group-1", 1000),
        ];

        let (updated_groups, updated_driver_groups) =
            update_driver_scan_group_rate_internal(&groups, "driver-a", 5000)
                .expect("bulk update should succeed");

        assert_eq!(updated_driver_groups.len(), 2);
        assert_eq!(updated_groups[0].scan_rate_ms, 5000);
        assert_eq!(updated_groups[1].scan_rate_ms, 5000);
        assert_eq!(updated_groups[2].scan_rate_ms, 1000);
    }
}
