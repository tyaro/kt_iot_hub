use super::launch_context::build_driver_ui_launch_context;
use super::paths::{find_default_driver_ui_path, normalize_optional_string};
use super::session::{register_active_session, resolve_launch_target};
use super::tempfile::build_driver_ui_io_paths;
use crate::app_state::AppState;
use crate::commands::dto::{
    CheckDriverUiResultRequest, CheckDriverUiResultResponse, ErrorResponse, LaunchDriverUiRequest,
    LaunchDriverUiResponse,
};
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

    let driver_configs = state.driver_configs.read().await.clone();
    let resolved = resolve_launch_target(
        requested_driver_id,
        requested_driver_type,
        requested_driver_ui_base_dir.clone(),
        &driver_configs,
    )?;

    let session_id = Uuid::new_v4().to_string();
    register_active_session(
        &state,
        &session_id,
        resolved.driver_id.clone(),
        resolved.driver_type.clone(),
    )
    .await?;

    let (input_json_path, output_json_path) = build_driver_ui_io_paths(
        resolved.driver_id.as_deref(),
        &resolved.driver_type,
        &session_id,
    );

    let launch_context = build_driver_ui_launch_context(
        &state,
        &session_id,
        resolved.driver_id.clone(),
        resolved.driver_type.clone(),
        output_json_path.to_string_lossy().to_string(),
        req.editing_tag_id.clone(),
    )
    .await?;

    let launch_context_text = serde_json::to_string_pretty(&launch_context).map_err(|e| {
        ErrorResponse::serialize_error(format!(
            "Failed to serialize driver UI launch context: {}",
            e
        ))
    })?;

    let state_for_input_write = state.clone();
    std::fs::write(&input_json_path, launch_context_text).map_err(|e| {
        let session_id_clone = session_id.clone();
        tauri::async_runtime::block_on(async move {
            let mut sessions = state_for_input_write
                .active_driver_ui_sessions
                .write()
                .await;
            sessions.remove(&session_id_clone);
        });
        ErrorResponse::io_error(format!("Failed to write driver UI input JSON: {}", e))
    })?;

    let mut command = std::process::Command::new(&resolved.executable_path);
    command
        .arg("--driver-ui-mode")
        .arg("--session-id")
        .arg(&session_id)
        .arg("--driver-type")
        .arg(&resolved.driver_type)
        .arg("--input-json")
        .arg(&input_json_path)
        .arg("--output-json")
        .arg(&output_json_path);

    if let Some(driver_id) = resolved.driver_id.as_ref() {
        command.arg("--driver-id").arg(driver_id);
    }

    info!(
        "Launching driver UI: session_id={} driver_type={} driver_id={} executable= {}",
        session_id,
        resolved.driver_type,
        resolved
            .driver_id
            .clone()
            .unwrap_or_else(|| "<new>".to_string()),
        resolved.executable_path
    );

    let state_for_spawn = state.clone();
    let executable_path_for_error = resolved.executable_path.clone();
    let session_id_for_spawn_error = session_id.clone();
    let input_json_path_for_spawn_error = input_json_path.clone();
    let mut child = command.spawn().map_err(|e| {
        tauri::async_runtime::block_on(async move {
            let mut sessions = state_for_spawn.active_driver_ui_sessions.write().await;
            sessions.remove(&session_id_for_spawn_error);
        });
        let _ = std::fs::remove_file(&input_json_path_for_spawn_error);

        ErrorResponse::new(
            "PROCESS_LAUNCH_FAILED",
            format!(
                "Failed to launch driver UI: {} (path={})",
                e, executable_path_for_error
            ),
        )
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
                session_id_for_log, wait_status_text, output_exists
            );
        }
    });

    Ok(LaunchDriverUiResponse {
        session_id,
        output_json_path: output_json_path.to_string_lossy().to_string(),
        driver_id: resolved.driver_id,
        driver_type: resolved.driver_type,
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
        return Err(ErrorResponse::invalid_input(
            "output_json_path cannot be empty",
        ));
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
