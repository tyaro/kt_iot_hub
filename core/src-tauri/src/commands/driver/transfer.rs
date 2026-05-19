use super::transfer_impl::{
    replace_registry_tags, restart_enabled_drivers, validate_import_payload,
    TagManagementSettingsFile, TAG_MANAGEMENT_SCHEMA_VERSION,
};
use crate::app_state::AppState;
use crate::commands::driver::toml_io::{write_drivers_toml_atomic, write_tags_toml_atomic};
use crate::commands::dto::{
    ErrorResponse, ExportTagManagementSettingsRequest, ExportTagManagementSettingsResponse,
    ImportTagManagementSettingsRequest, ImportTagManagementSettingsResponse,
};
use crate::config::TagConfig;
use std::path::{Path, PathBuf};
use tauri::Manager;

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
        ErrorResponse::serialize_error(format!(
            "Failed to serialize tag management settings: {}",
            error
        ))
    })?;

    std::fs::write(&path, json).map_err(|error| {
        ErrorResponse::io_error(format!(
            "Failed to write export file {}: {}",
            path.display(),
            error
        ))
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
        ErrorResponse::io_error(format!(
            "Failed to read import file {}: {}",
            path.display(),
            error
        ))
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

fn detect_default_driver_ui_base_dir(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        for candidate in [resource_dir.clone(), resource_dir.join("_up_")] {
            if candidate.join("ops").join("driver-ui").exists() {
                return Some(candidate);
            }
            if candidate.join("driver-ui").exists() {
                return Some(candidate);
            }
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            if exe_dir.join("ops").join("driver-ui").exists() {
                return Some(exe_dir.to_path_buf());
            }
            if exe_dir.join("driver-ui").exists() {
                return Some(exe_dir.to_path_buf());
            }
        }
    }

    if let Some(repo_root) = find_repo_root() {
        if repo_root.join("ops").join("driver-ui").exists() {
            return Some(repo_root);
        }
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
