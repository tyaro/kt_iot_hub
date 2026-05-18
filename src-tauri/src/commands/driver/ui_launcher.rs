//! ドライバUI 起動・結果ファイル待ち受け関連のコマンド群。

use super::ui_paths::{
    find_default_driver_ui_path, normalize_optional_string, resolve_driver_ui_path_for_type,
    resolve_driver_ui_path_with_base,
};
use crate::app_state::AppState;
use crate::commands::dto::{
    CheckDriverUiResultRequest, CheckDriverUiResultResponse, ErrorResponse, LaunchDriverUiRequest,
    LaunchDriverUiResponse,
};
use kt_driver_ui_protocol::{
    DriverUiLaunchContext, DriverUiLaunchData, DriverUiLaunchDriver, DriverUiLaunchScanGroup,
    DriverUiLaunchSession, DriverUiLaunchTag,
};
use chrono::Utc;
use tracing::{info, warn};
use uuid::Uuid;

#[tauri::command]
pub async fn launch_driver_ui(
    state: tauri::State<'_, AppState>,
    req: LaunchDriverUiRequest,
) -> Result<LaunchDriverUiResponse, ErrorResponse> {
    let requested_driver_id = normalize_optional_string(req.driver_id);
    let requested_driver_type = normalize_optional_string(req.driver_type);
    let requested_driver_ui_base_dir = normalize_optional_string(req.driver_ui_base_dir);

    if let Some(driver_ui_base_dir) = requested_driver_ui_base_dir.clone() {
        *state.driver_ui_base_dir.write().await = Some(driver_ui_base_dir);
    }

    if requested_driver_id.is_none() && requested_driver_type.is_none() {
        return Err(ErrorResponse {
            error: "driver_id or driver_type is required".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let driver_configs = state.driver_configs.read().await.clone();

    // driver_id 指定があれば既存ドライバの種別から、なければ driver_type から解決。
    let (driver_id, driver_type, executable_path) = if let Some(driver_id) = requested_driver_id.clone() {
        let driver_config = driver_configs
            .iter()
            .find(|cfg| cfg.id == driver_id)
            .cloned()
            .ok_or(ErrorResponse {
                error: format!("Driver not found: {}", driver_id),
                code: "NOT_FOUND".to_string(),
            })?;

        let executable_path = resolve_driver_ui_path_with_base(
            &driver_config,
            requested_driver_ui_base_dir.as_deref(),
        )
        .ok_or(ErrorResponse {
            error: format!(
                "Driver UI executable not found for driver {} (place it under driver-ui/{}/registration-ui(.exe))",
                driver_id, driver_config.driver_type
            ),
            code: "NOT_CONFIGURED".to_string(),
        })?;

        (Some(driver_id), driver_config.driver_type, executable_path)
    } else {
        let driver_type = requested_driver_type.clone().ok_or(ErrorResponse {
            error: "driver_type is required when driver_id is omitted".to_string(),
            code: "INVALID_INPUT".to_string(),
        })?;

        let executable_path = resolve_driver_ui_path_for_type(
            &driver_configs,
            &driver_type,
            requested_driver_ui_base_dir.as_deref(),
        )
        .ok_or(ErrorResponse {
            error: format!(
                "Driver UI executable not found for driver type {} (place it under driver-ui/{}/registration-ui(.exe))",
                driver_type, driver_type
            ),
            code: "NOT_CONFIGURED".to_string(),
        })?;

        (None, driver_type, executable_path)
    };

    let session_id = Uuid::new_v4().to_string();

    // 同一ドライバに対する多重起動を防ぐ。
    {
        let mut sessions = state.active_driver_ui_sessions.write().await;
        if let Some(existing_driver_id) = driver_id.as_ref() {
            if sessions.values().any(|session| {
                session.process_active
                    && session.target_driver_id.as_ref() == Some(existing_driver_id)
            }) {
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
                process_active: true,
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
        req.editing_tag_id.clone(),
    )
    .await?;

    let launch_context_text =
        serde_json::to_string_pretty(&launch_context).map_err(|e| ErrorResponse {
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
    command
        .arg("--driver-ui-mode")
        .arg("--session-id")
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

    info!(
        "Launching driver UI: session_id={} driver_type={} driver_id={} executable={}",
        session_id,
        driver_type,
        driver_id.clone().unwrap_or_else(|| "<new>".to_string()),
        executable_path
    );

    let state_for_spawn = state.clone();
    let mut child = command.spawn().map_err(|e| {
        let session_id_clone = session_id.clone();
        tauri::async_runtime::block_on(async move {
            let mut sessions = state_for_spawn.active_driver_ui_sessions.write().await;
            sessions.remove(&session_id_clone);
        });
        let _ = std::fs::remove_file(&input_json_path);

        ErrorResponse {
            error: format!(
                "Failed to launch driver UI: {} (path={})",
                e, executable_path
            ),
            code: "PROCESS_LAUNCH_FAILED".to_string(),
        }
    })?;

    let app_state_for_wait = state.inner().clone();
    let session_id_for_wait = session_id.clone();
    let output_json_path_for_wait = output_json_path.clone();
    std::thread::spawn(move || {
        let wait_result = child.wait();
        let session_id_for_log = session_id_for_wait.clone();
        let output_exists = std::path::Path::new(&output_json_path_for_wait).exists();
        let wait_status_text = wait_result
            .as_ref()
            .map(|status| status.to_string())
            .unwrap_or_else(|error| format!("wait-failed:{}", error));

        tauri::async_runtime::block_on(async move {
            let mut sessions = app_state_for_wait.active_driver_ui_sessions.write().await;
            if output_exists {
                if let Some(session) = sessions.get_mut(&session_id_for_wait) {
                    session.process_active = false;
                }
            } else {
                sessions.remove(&session_id_for_wait);
            }
        });

        if let Err(error) = wait_result {
            warn!(
                "driver ui process wait failed: session_id={}, error={}",
                session_id_for_log, error
            );
        } else {
            info!(
                "Driver UI process exited: session_id={} status={} output_exists={}",
                session_id_for_log,
                wait_status_text,
                output_exists
            );
        }
    });

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
    editing_tag_id: Option<String>,
) -> Result<DriverUiLaunchContext, ErrorResponse> {
    let scan_groups = state.scan_groups.read().await.clone();
    let tags = state.registry.list_all().await;
    let driver_configs = state.driver_configs.read().await.clone();

    let existing_driver_ids: Vec<String> = driver_configs
        .iter()
        .filter(|cfg| cfg.driver_type == driver_type)
        .map(|cfg| cfg.id.clone())
        .collect();

    let driver_settings = driver_id
        .as_ref()
        .and_then(|target_driver_id| {
            driver_configs
                .iter()
                .find(|cfg| cfg.id == *target_driver_id)
                .map(|cfg| cfg.settings.clone())
        });

    // ドライバ配下のタグを scanGroupId でグループ化
    let tags_by_scan_group: std::collections::HashMap<String, Vec<_>> = tags
        .iter()
        .filter(|tag| {
            if let Some(ref target_driver_id) = driver_id {
                tag.driver_id == *target_driver_id
            } else {
                false
            }
        })
        .fold(
            std::collections::HashMap::new(),
            |mut map, tag| {
                map.entry(tag.scan_group_id.clone())
                    .or_insert_with(Vec::new)
                    .push(tag);
                map
            },
        );

    // scanGroups にネストされたタグ構造を生成
    let filtered_scan_groups: Vec<DriverUiLaunchScanGroup> = scan_groups
        .into_iter()
        .filter(|group| {
            if let Some(ref target_driver_id) = driver_id {
                group.driver == *target_driver_id
            } else {
                false
            }
        })
        .map(|group| {
            let group_tags = tags_by_scan_group
                .get(&group.id)
                .map(|tag_refs| {
                    tag_refs
                        .iter()
                        .map(|tag| DriverUiLaunchTag {
                            id: tag.id.0.clone(),
                            name: tag.name.clone(),
                            data_type: tag.data_type.as_str().to_string(),
                            driver_id: tag.driver_id.clone(),
                            scan_group_id: tag.scan_group_id.clone(),
                            enabled: true,
                            driver_spec: tag.driver_spec.clone(),
                            metadata: tag.metadata.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            DriverUiLaunchScanGroup {
                id: group.id,
                driver: group.driver,
                scan_rate_ms: group.scan_rate_ms,
                schema: group.schema,
                table: group.table,
                timestamp_column: group.timestamp_column,
                node: group.node,
                tags: group_tags,
            }
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
            editing_tag_id,
        },
        driver: DriverUiLaunchDriver {
            driver_type,
            driver_id,
        },
        context: DriverUiLaunchData {
            scan_groups: filtered_scan_groups,
            existing_driver_ids,
            driver_settings,
        },
    })
}

/// 指定ドライバタイプの登録UI 実行ファイルが利用可能かをチェックする。
/// 既存ドライバ設定がなくても確認できる。
#[tauri::command]
pub async fn check_driver_ui_available(
    driver_type: String,
    driver_ui_base_dir: Option<String>,
) -> bool {
    find_default_driver_ui_path(
        driver_type.trim(),
        driver_ui_base_dir.as_deref().and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
    )
    .is_some()
}

#[tauri::command]
pub async fn check_driver_ui_result(
    state: tauri::State<'_, AppState>,
    req: CheckDriverUiResultRequest,
) -> Result<CheckDriverUiResultResponse, ErrorResponse> {
    if req.output_json_path.trim().is_empty() {
        return Err(ErrorResponse {
            error: "output_json_path cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let process_active = if let Some(session_id) = req.session_id.as_ref() {
        let sessions = state.active_driver_ui_sessions.read().await;
        sessions
            .get(session_id)
            .map(|session| session.process_active)
            .unwrap_or(false)
    } else {
        false
    };

    Ok(CheckDriverUiResultResponse {
        ready: std::path::Path::new(&req.output_json_path).exists(),
        process_active,
    })
}
