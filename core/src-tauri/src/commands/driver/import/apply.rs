use super::super::runtime_sync::sync_driver_runtime;
use super::super::toml_io::{write_drivers_toml_atomic, write_tags_toml_atomic};
use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use std::str::FromStr;

pub(super) async fn apply_driver_import(
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

    sync_driver_runtime(state, driver_config).await?;
    Ok(())
}
