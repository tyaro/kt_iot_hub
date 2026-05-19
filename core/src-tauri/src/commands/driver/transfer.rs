use crate::app_state::AppState;
use crate::commands::driver::toml_io::{write_drivers_toml_atomic, write_tags_toml_atomic};
use crate::commands::dto::{
    ErrorResponse, ExportTagManagementSettingsRequest, ExportTagManagementSettingsResponse,
    ImportTagManagementSettingsRequest, ImportTagManagementSettingsResponse,
};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::Manager;

const TAG_MANAGEMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct TagManagementSettingsFile {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "exportedAt")]
    exported_at: String,
    drivers: Vec<DriverConfig>,
    #[serde(rename = "scanGroups")]
    scan_groups: Vec<ScanGroupConfig>,
    tags: Vec<TagConfig>,
}

#[tauri::command]
pub async fn export_tag_management_settings(
    state: tauri::State<'_, AppState>,
    req: ExportTagManagementSettingsRequest,
) -> Result<ExportTagManagementSettingsResponse, ErrorResponse> {
    let path = normalize_target_path(&req.path)?;
    ensure_parent_dir(&path)?;

    let drivers = state.driver_configs.read().await.clone();
    let scan_groups = state.scan_groups.read().await.clone();
    let tags = build_tag_configs_from_registry(&state).await;

    let payload = TagManagementSettingsFile {
        schema_version: TAG_MANAGEMENT_SCHEMA_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        drivers: drivers.clone(),
        scan_groups: scan_groups.clone(),
        tags: tags.clone(),
    };

    let json = serde_json::to_string_pretty(&payload).map_err(|error| {
        ErrorResponse::serialize_error(format!("Failed to serialize tag management settings: {}", error))
    })?;

    std::fs::write(&path, json).map_err(|error| {
        ErrorResponse::io_error(format!("Failed to write export file {}: {}", path.display(), error))
    })?;

    Ok(ExportTagManagementSettingsResponse {
        path: path.to_string_lossy().to_string(),
        driver_count: drivers.len(),
        scan_group_count: scan_groups.len(),
        tag_count: tags.len(),
    })
}

#[tauri::command]
pub async fn import_tag_management_settings(
    state: tauri::State<'_, AppState>,
    req: ImportTagManagementSettingsRequest,
) -> Result<ImportTagManagementSettingsResponse, ErrorResponse> {
    let path = normalize_target_path(&req.path)?;
    let text = std::fs::read_to_string(&path).map_err(|error| {
        ErrorResponse::io_error(format!("Failed to read import file {}: {}", path.display(), error))
    })?;

    let payload: TagManagementSettingsFile = serde_json::from_str(&text).map_err(|error| {
        ErrorResponse::validation_error(format!(
            "Failed to parse tag management settings file {}: {}",
            path.display(),
            error
        ))
    })?;

    validate_import_payload(&payload)?;

    let restart_drivers = state.runtime_status.read().await.drivers_running;
    if restart_drivers {
        let mut manager = state.drivers.write().await;
        manager.stop_all().await.map_err(ErrorResponse::from)?;
        state.runtime_status.write().await.drivers_running = false;
    }

    write_drivers_toml_atomic(&payload.drivers)?;
    write_tags_toml_atomic(&payload.scan_groups, &payload.tags)?;

    {
        let mut configs = state.driver_configs.write().await;
        *configs = payload.drivers.clone();
    }
    {
        let mut scan_groups = state.scan_groups.write().await;
        *scan_groups = payload.scan_groups.clone();
    }

    replace_registry_tags(&state, &payload.tags).await?;

    if restart_drivers {
        restart_enabled_drivers(&state).await?;
    }

    Ok(ImportTagManagementSettingsResponse {
        path: path.to_string_lossy().to_string(),
        driver_count: payload.drivers.len(),
        scan_group_count: payload.scan_groups.len(),
        tag_count: payload.tags.len(),
    })
}

#[tauri::command]
pub fn get_default_driver_ui_base_dir(
    app_handle: tauri::AppHandle,
) -> Result<Option<String>, ErrorResponse> {
    Ok(detect_default_driver_ui_base_dir(&app_handle)
        .map(|path| path.to_string_lossy().to_string()))
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

fn normalize_target_path(raw: &str) -> Result<PathBuf, ErrorResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ErrorResponse::invalid_input("path cannot be empty"));
    }
    Ok(PathBuf::from(trimmed))
}

fn ensure_parent_dir(path: &Path) -> Result<(), ErrorResponse> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            ErrorResponse::io_error(format!(
                "Failed to create export directory {}: {}",
                parent.display(),
                error
            ))
        })?;
    }
    Ok(())
}

fn validate_import_payload(payload: &TagManagementSettingsFile) -> Result<(), ErrorResponse> {
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

    let driver_ids: HashSet<&str> = payload.drivers.iter().map(|driver| driver.id.as_str()).collect();
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
            ErrorResponse::validation_error(format!("Invalid data_type for tag {}: {}", tag.id, error))
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

fn ensure_unique_ids<'a>(ids: impl IntoIterator<Item = &'a str>, label: &str) -> Result<(), ErrorResponse> {
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

async fn replace_registry_tags(
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

async fn restart_enabled_drivers(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
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

fn detect_default_driver_ui_base_dir(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        for candidate in [resource_dir.clone(), resource_dir.join("_up_")] {
            if candidate.join("driver-ui").exists() {
                return Some(candidate);
            }
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            if exe_dir.join("driver-ui").exists() {
                return Some(exe_dir.to_path_buf());
            }
        }
    }

    if let Some(repo_root) = find_repo_root() {
        if repo_root.join("driver-ui").exists() {
            return Some(repo_root);
        }
        let src_tauri_dir = repo_root.join("src-tauri");
        if src_tauri_dir.join("driver-ui").exists() {
            return Some(src_tauri_dir);
        }
    }

    None
}

fn find_repo_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let mut cursor = Some(current_dir.as_path());
    while let Some(path) = cursor {
        if path.join("Cargo.toml").exists() {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }
    None
}
