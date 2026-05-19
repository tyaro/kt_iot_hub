// タグ管理設定の import/export に関わる検証・適用処理
// transfer.rs のコマンド実装から分離した内部ヘルパー群

use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;

/// タグ管理設定ファイルのスキーマバージョン
pub(super) const TAG_MANAGEMENT_SCHEMA_VERSION: u32 = 1;

/// タグ管理設定ファイルの内部表現（JSON import/export 用）
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TagManagementSettingsFile {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(rename = "exportedAt")]
    pub exported_at: String,
    pub drivers: Vec<DriverConfig>,
    #[serde(rename = "scanGroups")]
    pub scan_groups: Vec<ScanGroupConfig>,
    pub tags: Vec<TagConfig>,
}

/// インポートペイロードの整合性を検証する
pub(super) fn validate_import_payload(
    payload: &TagManagementSettingsFile,
) -> Result<(), ErrorResponse> {
    if payload.schema_version != TAG_MANAGEMENT_SCHEMA_VERSION {
        return Err(ErrorResponse::validation_error(format!(
            "Unsupported schemaVersion: {} (expected {})",
            payload.schema_version, TAG_MANAGEMENT_SCHEMA_VERSION
        )));
    }

    ensure_unique_ids(
        payload.drivers.iter().map(|driver| driver.id.as_str()),
        "driver id",
    )?;
    ensure_unique_ids(
        payload
            .scan_groups
            .iter()
            .map(|scan_group| scan_group.id.as_str()),
        "scan group id",
    )?;
    ensure_unique_ids(payload.tags.iter().map(|tag| tag.id.as_str()), "tag id")?;

    let driver_ids: HashSet<&str> = payload
        .drivers
        .iter()
        .map(|driver| driver.id.as_str())
        .collect();
    let scan_group_ids: HashSet<&str> = payload
        .scan_groups
        .iter()
        .map(|scan_group| scan_group.id.as_str())
        .collect();

    for scan_group in &payload.scan_groups {
        if !driver_ids.contains(scan_group.driver.as_str()) {
            return Err(ErrorResponse::validation_error(format!(
                "Scan group {} refers to unknown driver {}",
                scan_group.id, scan_group.driver
            )));
        }
    }

    for tag in &payload.tags {
        DataType::from_str(&tag.data_type).map_err(|error| {
            ErrorResponse::validation_error(format!(
                "Invalid data_type for tag {}: {}",
                tag.id, error
            ))
        })?;

        if !driver_ids.contains(tag.driver.as_str()) {
            return Err(ErrorResponse::validation_error(format!(
                "Tag {} refers to unknown driver {}",
                tag.id, tag.driver
            )));
        }

        if !scan_group_ids.contains(tag.scan_group.as_str()) {
            return Err(ErrorResponse::validation_error(format!(
                "Tag {} refers to unknown scan group {}",
                tag.id, tag.scan_group
            )));
        }

        if let Some(scan_group) = payload
            .scan_groups
            .iter()
            .find(|scan_group| scan_group.id == tag.scan_group)
        {
            if scan_group.driver != tag.driver {
                return Err(ErrorResponse::validation_error(format!(
                    "Tag {} refers to scan group {} owned by different driver {}",
                    tag.id, tag.scan_group, scan_group.driver
                )));
            }
        }
    }

    Ok(())
}

fn ensure_unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), ErrorResponse> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id.to_string()) {
            return Err(ErrorResponse::validation_error(format!(
                "Duplicate {}: {}",
                label, id
            )));
        }
    }
    Ok(())
}

/// レジストリのタグを import データで置き換える
pub(super) async fn replace_registry_tags(
    state: &tauri::State<'_, AppState>,
    imported_tags: &[TagConfig],
) -> Result<(), ErrorResponse> {
    let existing_tags = state.registry.list_all().await;
    for tag in existing_tags {
        let _ = state.registry.remove(&tag.id).await;
    }

    for tag in imported_tags {
        let data_type = DataType::from_str(&tag.data_type).map_err(ErrorResponse::from)?;
        state
            .registry
            .insert(Tag {
                id: TagId(tag.id.clone()),
                name: tag.name.clone(),
                data_type,
                driver_id: tag.driver.clone(),
                scan_group_id: tag.scan_group.clone(),
                driver_spec: tag.driver_spec.clone(),
                metadata: tag.metadata.clone(),
            })
            .await;
    }

    Ok(())
}

/// 有効化済みドライバを再起動する
pub(super) async fn restart_enabled_drivers(
    state: &tauri::State<'_, AppState>,
) -> Result<(), ErrorResponse> {
    let driver_ui_base_dir = state.driver_ui_base_dir.read().await.clone();
    let drivers_to_start: Vec<(String, String)> = state
        .driver_configs
        .read()
        .await
        .iter()
        .filter(|cfg| cfg.enabled.unwrap_or(true))
        .map(|cfg| (cfg.id.clone(), cfg.driver_type.clone()))
        .collect();

    let has_drivers = !drivers_to_start.is_empty();
    let mut manager = state.drivers.write().await;
    for (driver_id, driver_type) in drivers_to_start {
        manager
            .start_driver(&driver_id, &driver_type, driver_ui_base_dir.as_deref())
            .await
            .map_err(ErrorResponse::from)?;
    }

    state.runtime_status.write().await.drivers_running =
        has_drivers && !manager.running_driver_ids().is_empty();
    Ok(())
}
