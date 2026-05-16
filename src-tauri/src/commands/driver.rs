use super::dto::{
    CheckDriverUiResultRequest, CheckDriverUiResultResponse, DriverDto, ErrorResponse,
    ImportDriverUiResultRequest, ImportDriverUiResultResponse, LaunchDriverUiRequest,
    LaunchDriverUiResponse, SaveDriverRequest,
};
use super::driver_ui_protocol::{
    DriverUiDriverPayload, DriverUiImportPayload, DriverUiLaunchContext, DriverUiLaunchData,
    DriverUiLaunchDriver, DriverUiLaunchScanGroup, DriverUiLaunchSession, DriverUiLaunchTag,
};
use crate::app_state::AppState;
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use crate::core::{DataType, Tag, TagId};
use crate::drivers::postgres::PostgresDriver;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

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
    if req.id.trim().is_empty() {
        return Err(ErrorResponse {
            error: "Driver ID cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let existing = {
        let configs = state.driver_configs.read().await;
        configs.iter().find(|d| d.id == req.id).cloned()
    };

    let password = if req.password.trim().is_empty() {
        existing
            .as_ref()
            .and_then(|cfg| cfg.settings.get("password"))
            .cloned()
            .unwrap_or_else(|| serde_json::Value::String(String::new()))
    } else {
        serde_json::Value::String(req.password.clone())
    };

    let registration_ui_path = existing
        .as_ref()
        .and_then(|cfg| cfg.settings.get("registration_ui_path"))
        .cloned();
    let driver_ui_path = existing
        .as_ref()
        .and_then(|cfg| cfg.settings.get("driver_ui_path"))
        .cloned();

    let mut settings = serde_json::Map::from_iter([
        ("host".to_string(), serde_json::Value::String(req.host.clone())),
        ("port".to_string(), serde_json::Value::from(req.port)),
        (
            "database".to_string(),
            serde_json::Value::String(req.database.clone()),
        ),
        (
            "username".to_string(),
            serde_json::Value::String(req.username.clone()),
        ),
        ("password".to_string(), password),
    ]);
    if let Some(path) = registration_ui_path {
        settings.insert("registration_ui_path".to_string(), path);
    }
    if let Some(path) = driver_ui_path {
        settings.insert("driver_ui_path".to_string(), path);
    }

    let config = DriverConfig {
        id: req.id.clone(),
        driver_type: req.driver_type.clone(),
        enabled: Some(req.enabled),
        settings: serde_json::Value::Object(settings),
    };

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

    Ok(())
}

#[tauri::command]
pub async fn delete_driver(
    state: tauri::State<'_, AppState>,
    driver_id: String,
) -> Result<(), ErrorResponse> {
    let driver_id = driver_id.trim().to_string();
    if driver_id.is_empty() {
        return Err(ErrorResponse {
            error: "driver_id cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    {
        let sessions = state.active_driver_ui_sessions.read().await;
        if sessions
            .values()
            .any(|session| session.target_driver_id.as_deref() == Some(driver_id.as_str()))
        {
            return Err(ErrorResponse {
                error: format!(
                    "Driver UI session is active for {}. Close the external UI first.",
                    driver_id
                ),
                code: "SESSION_ACTIVE".to_string(),
            });
        }
    }

    let existing_driver_configs = state.driver_configs.read().await.clone();
    if !existing_driver_configs.iter().any(|cfg| cfg.id == driver_id) {
        return Err(ErrorResponse {
            error: format!("Driver not found: {}", driver_id),
            code: "NOT_FOUND".to_string(),
        });
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
        if manager.contains(&driver_id) {
            let _ = manager.stop_driver(&driver_id).await;
            manager.remove(&driver_id);
        }
    }

    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        sessions.retain(|_, session| session.target_driver_id.as_deref() != Some(driver_id.as_str()));
    }

    Ok(())
}

#[tauri::command]
pub async fn launch_driver_ui(
    state: tauri::State<'_, AppState>,
    req: LaunchDriverUiRequest,
) -> Result<LaunchDriverUiResponse, ErrorResponse> {
    let requested_driver_id = normalize_optional_string(req.driver_id);
    let requested_driver_type = normalize_optional_string(req.driver_type);

    if requested_driver_id.is_none() && requested_driver_type.is_none() {
        return Err(ErrorResponse {
            error: "driver_id or driver_type is required".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let driver_configs = state.driver_configs.read().await.clone();

    let (driver_id, driver_type, executable_path) = if let Some(driver_id) = requested_driver_id.clone() {
        let driver_config = driver_configs
            .iter()
            .find(|cfg| cfg.id == driver_id)
            .cloned()
            .ok_or(ErrorResponse {
                error: format!("Driver not found: {}", driver_id),
                code: "NOT_FOUND".to_string(),
            })?;

        let executable_path = resolve_driver_ui_path(&driver_config).ok_or(ErrorResponse {
            error: format!(
                "Driver UI executable not found for driver {} (set registration_ui_path/driver_ui_path or place it under driver-ui/{}/registration-ui(.exe))",
                driver_id,
                driver_config.driver_type
            ),
            code: "NOT_CONFIGURED".to_string(),
        })?;

        (Some(driver_id), driver_config.driver_type, executable_path)
    } else {
        let driver_type = requested_driver_type.clone().ok_or(ErrorResponse {
            error: "driver_type is required when driver_id is omitted".to_string(),
            code: "INVALID_INPUT".to_string(),
        })?;

        let executable_path = resolve_driver_ui_path_for_type(&driver_configs, &driver_type).ok_or(ErrorResponse {
            error: format!(
                "Driver UI executable not found for driver type {} (set registration_ui_path/driver_ui_path or place it under driver-ui/{}/registration-ui(.exe))",
                driver_type,
                driver_type
            ),
            code: "NOT_CONFIGURED".to_string(),
        })?;

        (None, driver_type, executable_path)
    };

    let session_id = Uuid::new_v4().to_string();

    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        if let Some(existing_driver_id) = driver_id.as_ref() {
            if sessions.values().any(|session| session.target_driver_id.as_ref() == Some(existing_driver_id)) {
                return Err(ErrorResponse {
                    error: format!("Driver UI is already active for driver {}", existing_driver_id),
                    code: "ALREADY_RUNNING".to_string(),
                });
            }
        }
        sessions.insert(
            session_id.clone(),
            crate::app_state::DriverUiSessionState {
                target_driver_id: driver_id.clone(),
                driver_type: driver_type.clone(),
            },
        );
    }

    let output_json_path = std::env::temp_dir().join(format!(
        "kt_iot_hub_driver_ui_{}_{}.json",
        driver_id.clone().unwrap_or_else(|| format!("new-{}", driver_type)),
        session_id
    ));

    let input_json_path = std::env::temp_dir().join(format!(
        "kt_iot_hub_driver_ui_input_{}_{}.json",
        driver_id.clone().unwrap_or_else(|| format!("new-{}", driver_type)),
        session_id
    ));

    let launch_context = build_driver_ui_launch_context(
        &state,
        &session_id,
        driver_id.clone(),
        driver_type.clone(),
        output_json_path.to_string_lossy().to_string(),
    )
    .await?;

    let launch_context_text = serde_json::to_string_pretty(&launch_context).map_err(|e| ErrorResponse {
        error: format!("Failed to serialize driver UI launch context: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    let state_for_input_write = state.clone();
    std::fs::write(&input_json_path, launch_context_text).map_err(|e| {
        let session_id_clone = session_id.clone();
        tauri::async_runtime::block_on(async move {
            let mut sessions = state_for_input_write.active_driver_ui_sessions.write().await;
            sessions.remove(&session_id_clone);
        });
        ErrorResponse {
            error: format!("Failed to write driver UI input JSON: {}", e),
            code: "IO_ERROR".to_string(),
        }
    })?;

    let mut command = std::process::Command::new(&executable_path);
    command.arg("--session-id")
        .arg(&session_id)
        .arg("--driver-type")
        .arg(&driver_type)
        .arg("--input-json")
        .arg(&input_json_path)
        .arg("--output-json")
        .arg(&output_json_path);

    if let Some(driver_id) = driver_id.as_ref() {
        command.arg("--driver-id").arg(driver_id);
    }

    let state_for_spawn = state.clone();
    command.spawn().map_err(|e| {
        let session_id_clone = session_id.clone();
        tauri::async_runtime::block_on(async move {
            let mut sessions = state_for_spawn.active_driver_ui_sessions.write().await;
            sessions.remove(&session_id_clone);
        });
        let _ = std::fs::remove_file(&input_json_path);

        ErrorResponse {
        error: format!(
            "Failed to launch driver UI: {} (path={})",
            e,
            executable_path
        ),
        code: "PROCESS_LAUNCH_FAILED".to_string(),
    }})?;

    Ok(LaunchDriverUiResponse {
        session_id,
        output_json_path: output_json_path.to_string_lossy().to_string(),
        driver_id,
        driver_type,
    })
}

async fn build_driver_ui_launch_context(
    state: &tauri::State<'_, AppState>,
    session_id: &str,
    driver_id: Option<String>,
    driver_type: String,
    output_json_path: String,
) -> Result<DriverUiLaunchContext, ErrorResponse> {
    let scan_groups = state.scan_groups.read().await.clone();
    let tags = state.registry.list_all().await;

    let filtered_scan_groups: Vec<DriverUiLaunchScanGroup> = scan_groups
        .into_iter()
        .filter(|group| {
            if let Some(ref target_driver_id) = driver_id {
                group.driver == *target_driver_id
            } else {
                false
            }
        })
        .map(|group| DriverUiLaunchScanGroup {
            id: group.id,
            driver: group.driver,
            scan_rate_ms: group.scan_rate_ms,
            schema: group.schema,
            table: group.table,
            timestamp_column: group.timestamp_column,
            node: group.node,
        })
        .collect();

    let filtered_tags: Vec<DriverUiLaunchTag> = tags
        .into_iter()
        .filter(|tag| {
            if let Some(ref target_driver_id) = driver_id {
                tag.driver_id == *target_driver_id
            } else {
                false
            }
        })
        .map(|tag| DriverUiLaunchTag {
            id: tag.id.0,
            name: tag.name,
            data_type: tag.data_type.as_str().to_string(),
            driver_id: tag.driver_id,
            scan_group_id: tag.scan_group_id,
            enabled: true,
            driver_spec: tag.driver_spec,
            metadata: tag.metadata,
        })
        .collect();

    Ok(DriverUiLaunchContext {
        schema_version: 1,
        request_id: format!("req-{}", session_id),
        generated_at: Utc::now().to_rfc3339(),
        direction: "host-to-driver".to_string(),
        session: DriverUiLaunchSession {
            session_id: session_id.to_string(),
            mode: "create-or-edit".to_string(),
            output_json_path,
        },
        driver: DriverUiLaunchDriver {
            driver_type,
            driver_id,
        },
        context: DriverUiLaunchData {
            scan_groups: filtered_scan_groups,
            tags: filtered_tags,
        },
    })
}

#[tauri::command]
pub async fn check_driver_ui_result(
    req: CheckDriverUiResultRequest,
) -> Result<CheckDriverUiResultResponse, ErrorResponse> {
    if req.output_json_path.trim().is_empty() {
        return Err(ErrorResponse {
            error: "output_json_path cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    Ok(CheckDriverUiResultResponse {
        ready: std::path::Path::new(&req.output_json_path).exists(),
    })
}

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

    if let Some(request_driver_id) = req.driver_id.as_ref().and_then(|value| normalize_optional_string(Some(value.clone()))) {
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

fn resolve_driver_ui_path(config: &DriverConfig) -> Option<String> {
    let explicit_path = config
        .settings
        .get("registration_ui_path")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            config
                .settings
                .get("driver_ui_path")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        });

    if let Some(path) = explicit_path {
        if let Some(resolved) = resolve_candidate_path(&path) {
            return Some(resolved);
        }
    }

    find_default_driver_ui_path(&config.driver_type)
}

fn resolve_candidate_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw = PathBuf::from(trimmed);
    if raw.is_absolute() && raw.exists() {
        return Some(path_to_string(raw));
    }
    if raw.exists() {
        return Some(path_to_string(raw));
    }

    for root in app_root_candidates() {
        let candidate = root.join(trimmed);
        if candidate.exists() {
            return Some(path_to_string(candidate));
        }
    }

    None
}

fn find_default_driver_ui_path(driver_type: &str) -> Option<String> {
    if driver_type.trim().is_empty() {
        return None;
    }

    let file_names = [
        format!("{}-registration-ui.exe", driver_type),
        "registration-ui.exe".to_string(),
        "driver-ui.exe".to_string(),
        format!("{}-registration-ui", driver_type),
        "registration-ui".to_string(),
        "driver-ui".to_string(),
    ];

    for root in app_root_candidates() {
        for file_name in &file_names {
            let candidate = root.join("driver-ui").join(driver_type).join(file_name);
            if candidate.exists() {
                return Some(path_to_string(candidate));
            }
        }
    }

    None
}

fn app_root_candidates() -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            roots.push(exe_dir.to_path_buf());
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }

    let mut unique = Vec::<PathBuf>::new();
    for path in roots {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

fn path_to_string(path: PathBuf) -> String {
    match path.canonicalize() {
        Ok(canonical) => canonical.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

#[derive(Serialize)]
struct TagsTomlFile {
    #[serde(rename = "scan_group")]
    scan_group: Vec<ScanGroupConfig>,
    #[serde(rename = "tag")]
    tag: Vec<TagConfig>,
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
    session: &crate::app_state::DriverUiSessionState,
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
                error: format!(
                    "Tag {} references unknown scanGroup {}",
                    t.id, scan_group_id
                ),
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
        if key != "id" && key != "driverType" && key != "driverKind" && key != "enabled" {
            merged.insert(key, value);
        }
    }

    serde_json::Value::Object(merged)
}

async fn sync_driver_runtime(
    state: &tauri::State<'_, AppState>,
    config: &DriverConfig,
) -> Result<(), ErrorResponse> {
    if config.driver_type != "postgres" {
        return Ok(());
    }

    let scan_groups: Vec<ScanGroupConfig> = state
        .scan_groups
        .read()
        .await
        .iter()
        .filter(|g| g.driver == config.id)
        .cloned()
        .collect();

    let runtime_running = state.runtime_status.read().await.drivers_running;
    let mut manager = state.drivers.write().await;
    if manager.contains(&config.id) {
        let _ = manager.stop_driver(&config.id).await;
    }
    manager.register(Box::new(PostgresDriver::new(config.clone(), scan_groups)));

    if runtime_running && config.enabled.unwrap_or(true) {
        manager
            .start_driver(&config.id, &state.registry, &state.tag_bus)
            .await
            .map_err(ErrorResponse::from)?;
    }

    Ok(())
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

#[derive(Serialize)]
struct DriversTomlFile {
    #[serde(rename = "driver")]
    driver: Vec<DriverConfig>,
}

fn write_drivers_toml_atomic(drivers: &[DriverConfig]) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    let drivers_path = config_dir.join("drivers.toml");
    let tmp_path = config_dir.join("drivers.toml.tmp");

    let toml_text = toml::to_string_pretty(&DriversTomlFile {
        driver: drivers.to_vec(),
    })
    .map_err(|e| ErrorResponse {
        error: format!("Failed to serialize drivers.toml: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write drivers.toml.tmp: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    replace_file_atomically(&tmp_path, &drivers_path, &toml_text, "drivers.toml")
}

fn replace_file_atomically(
    tmp_path: &Path,
    target_path: &Path,
    content: &str,
    label: &str,
) -> Result<(), ErrorResponse> {
    if target_path.exists() {
        let _ = std::fs::remove_file(target_path);
    }

    if let Err(rename_error) = std::fs::rename(tmp_path, target_path) {
        std::fs::write(target_path, content).map_err(|write_error| ErrorResponse {
            error: format!(
                "Failed to replace {} (rename: {}; write fallback: {})",
                label, rename_error, write_error
            ),
            code: "IO_ERROR".to_string(),
        })?;
        let _ = std::fs::remove_file(tmp_path);
    }

    Ok(())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn resolve_driver_ui_path_for_type(configs: &[DriverConfig], driver_type: &str) -> Option<String> {
    configs
        .iter()
        .filter(|cfg| cfg.driver_type == driver_type)
        .find_map(resolve_driver_ui_path)
}
