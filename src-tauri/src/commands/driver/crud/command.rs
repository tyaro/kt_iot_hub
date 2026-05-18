use super::logic::{
    build_driver_config, build_renamed_scan_groups, build_renamed_tags,
    ensure_non_empty_driver_id, merge_driver_configs_for_rename, normalize_optional_string,
};
use super::super::runtime_sync::sync_driver_runtime;
use super::super::toml_io::{write_drivers_toml_atomic, write_tags_toml_atomic};
use super::super::ui_launcher::paths::resolve_driver_ui_path;
use crate::app_state::AppState;
use crate::commands::dto::{DriverDto, ErrorResponse, SaveDriverRequest};
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::Tag;

#[tauri::command]
pub async fn list_drivers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DriverDto>, ErrorResponse> {
    let configs = state.driver_configs.read().await;
    Ok(configs
        .iter()
        .map(|cfg| DriverDto {
            id: cfg.id.clone(),
            driver_type: cfg.driver_type.clone(),
            enabled: cfg.enabled.unwrap_or(true),
            registration_ui_available: resolve_driver_ui_path(cfg).is_some(),
            host: cfg
                .settings
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: cfg
                .settings
                .get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(5432) as u16,
            database: cfg
                .settings
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            username: cfg
                .settings
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn save_driver(
    state: tauri::State<'_, AppState>,
    req: SaveDriverRequest,
) -> Result<(), ErrorResponse> {
    let driver_id = req.id.trim().to_string();
    ensure_non_empty_driver_id(&driver_id)?;

    let original_driver_id =
        normalize_optional_string(req.original_id.clone()).unwrap_or_else(|| driver_id.clone());
    let renaming = original_driver_id != driver_id;

    let existing_configs = state.driver_configs.read().await.clone();
    let existing = existing_configs
        .iter()
        .find(|d| d.id == original_driver_id)
        .cloned();

    if renaming {
        ensure_driver_ui_session_not_active(&state, &original_driver_id).await?;
    }

    if renaming && existing_configs.iter().any(|cfg| cfg.id == driver_id) {
        return Err(ErrorResponse::new(
            "ALREADY_EXISTS",
            format!("Driver ID already exists: {}", driver_id),
        ));
    }

    let config = build_driver_config(&req, driver_id.clone(), existing.as_ref());

    if renaming {
        apply_driver_id_rename(&state, &original_driver_id, &config, existing_configs).await?;
    } else {
        let final_configs = {
            let mut configs = state.driver_configs.write().await;
            if let Some(existing) = configs.iter_mut().find(|d| d.id == config.id) {
                *existing = config.clone();
            } else {
                configs.push(config.clone());
            }
            configs.clone()
        };

        write_drivers_toml_atomic(&final_configs)?;
        sync_driver_runtime(&state, &config).await?;
    }

    Ok(())
}

async fn ensure_driver_ui_session_not_active(
    state: &tauri::State<'_, AppState>,
    driver_id: &str,
) -> Result<(), ErrorResponse> {
    let sessions = state.active_driver_ui_sessions.read().await;
    if sessions
        .values()
        .any(|session| session.target_driver_id.as_deref() == Some(driver_id))
    {
        return Err(ErrorResponse::new(
            "SESSION_ACTIVE",
            format!(
                "Driver UI session exists for {}. Close or import the external UI result first.",
                driver_id
            ),
        ));
    }
    Ok(())
}

async fn apply_driver_id_rename(
    state: &tauri::State<'_, AppState>,
    original_driver_id: &str,
    new_config: &DriverConfig,
    existing_configs: Vec<DriverConfig>,
) -> Result<(), ErrorResponse> {
    let existing_registry_tags = state.registry.list_all().await;
    let final_tags_for_file =
        build_renamed_tags(&existing_registry_tags, original_driver_id, &new_config.id);

    let existing_scan_groups = state.scan_groups.read().await.clone();
    let final_scan_groups =
        build_renamed_scan_groups(existing_scan_groups, original_driver_id, &new_config.id);

    let final_driver_configs =
        merge_driver_configs_for_rename(existing_configs, original_driver_id, new_config);

    write_drivers_toml_atomic(&final_driver_configs)?;
    write_tags_toml_atomic(&final_scan_groups, &final_tags_for_file)?;

    {
        let mut driver_configs = state.driver_configs.write().await;
        *driver_configs = final_driver_configs;
    }
    {
        let mut scan_groups = state.scan_groups.write().await;
        *scan_groups = final_scan_groups;
    }
    {
        let mut manager = state.drivers.write().await;
        if manager.is_running(original_driver_id) {
            let _ = manager.stop_driver(original_driver_id).await;
        }
    }

    for tag in existing_registry_tags
        .into_iter()
        .filter(|tag| tag.driver_id == original_driver_id)
    {
        state
            .registry
            .insert(Tag {
                id: tag.id,
                name: tag.name,
                data_type: tag.data_type,
                driver_id: new_config.id.clone(),
                scan_group_id: tag.scan_group_id,
                driver_spec: tag.driver_spec,
                metadata: tag.metadata,
            })
            .await;
    }

    sync_driver_runtime(state, new_config).await?;

    Ok(())
}

#[tauri::command]
pub async fn delete_driver(
    state: tauri::State<'_, AppState>,
    driver_id: String,
) -> Result<(), ErrorResponse> {
    let driver_id = driver_id.trim().to_string();
    if driver_id.is_empty() {
        return Err(ErrorResponse::invalid_input("driver_id cannot be empty"));
    }

    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        sessions.retain(|_, session| {
            session.process_active
                || session.target_driver_id.as_deref() != Some(driver_id.as_str())
        });

        if sessions.values().any(|session| {
            session.process_active
                && session.target_driver_id.as_deref() == Some(driver_id.as_str())
        }) {
            return Err(ErrorResponse::new(
                "SESSION_ACTIVE",
                format!(
                    "Driver UI session is active for {}. Close the external UI first.",
                    driver_id
                ),
            ));
        }
    }

    let existing_driver_configs = state.driver_configs.read().await.clone();
    if !existing_driver_configs.iter().any(|cfg| cfg.id == driver_id) {
        return Err(ErrorResponse::not_found(format!("Driver not found: {}", driver_id)));
    }

    let final_driver_configs: Vec<DriverConfig> = existing_driver_configs
        .into_iter()
        .filter(|cfg| cfg.id != driver_id)
        .collect();

    let existing_scan_groups = state.scan_groups.read().await.clone();
    let final_scan_groups: Vec<ScanGroupConfig> = existing_scan_groups
        .into_iter()
        .filter(|scan_group| scan_group.driver != driver_id)
        .collect();

    let existing_registry_tags = state.registry.list_all().await;
    let final_tags_for_file: Vec<TagConfig> = existing_registry_tags
        .iter()
        .filter(|tag| tag.driver_id != driver_id)
        .map(|tag| TagConfig {
            id: tag.id.0.clone(),
            name: tag.name.clone(),
            data_type: tag.data_type.as_str().to_string(),
            driver: tag.driver_id.clone(),
            scan_group: tag.scan_group_id.clone(),
            driver_spec: tag.driver_spec.clone(),
            enabled: Some(true),
            metadata: tag.metadata.clone(),
        })
        .collect();

    write_drivers_toml_atomic(&final_driver_configs)?;
    write_tags_toml_atomic(&final_scan_groups, &final_tags_for_file)?;

    {
        let mut configs = state.driver_configs.write().await;
        *configs = final_driver_configs;
    }
    {
        let mut scan_groups = state.scan_groups.write().await;
        *scan_groups = final_scan_groups;
    }

    for tag in existing_registry_tags
        .into_iter()
        .filter(|tag| tag.driver_id == driver_id)
    {
        let _ = state.registry.remove(&tag.id).await;
    }

    {
        let mut manager = state.drivers.write().await;
        if manager.is_running(&driver_id) {
            let _ = manager.stop_driver(&driver_id).await;
        }
    }

    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        sessions.retain(|_, session| {
            session.target_driver_id.as_deref() != Some(driver_id.as_str())
        });
    }

    Ok(())
}
