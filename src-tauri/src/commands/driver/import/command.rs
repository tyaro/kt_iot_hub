use super::apply::apply_driver_import;
use super::validate::{validate_and_convert_payload, validate_import_request};
use super::super::ui_launcher::paths::normalize_optional_string;
use crate::app_state::AppState;
use crate::commands::dto::{ErrorResponse, ImportDriverUiResultRequest, ImportDriverUiResultResponse};
use kt_driver_ui_protocol::DriverUiImportPayload;

#[tauri::command]
pub async fn import_driver_ui_result(
    state: tauri::State<'_, AppState>,
    req: ImportDriverUiResultRequest,
) -> Result<ImportDriverUiResultResponse, ErrorResponse> {
    validate_import_request(&req)?;

    {
        let imported = state.imported_driver_ui_sessions.read().await;
        if imported.contains(&req.session_id) {
            return Err(ErrorResponse::new(
                "DUPLICATE_IMPORT",
                format!("Session already imported: {}", req.session_id),
            ));
        }
    }

    let session = {
        let sessions = state.active_driver_ui_sessions.read().await;
        sessions
            .get(&req.session_id)
            .cloned()
            .ok_or(ErrorResponse::new(
                "NO_ACTIVE_SESSION",
                format!("No active driver UI session: {}", req.session_id),
            ))?
    };

    if let Some(request_driver_id) = req
        .driver_id
        .as_ref()
        .and_then(|value| normalize_optional_string(Some(value.clone())))
    {
        if let Some(session_driver_id) = session.target_driver_id.as_ref() {
            if request_driver_id != *session_driver_id {
                return Err(ErrorResponse::new(
                    "SESSION_MISMATCH",
                    format!(
                        "Session mismatch for driver {} (expected={}, got={})",
                        request_driver_id, session_driver_id, request_driver_id
                    ),
                ));
            }
        }
    }

    let output_path = std::path::PathBuf::from(&req.output_json_path);
    if !output_path.exists() {
        return Err(ErrorResponse::new(
            "RESULT_NOT_READY",
            format!("Result file not found: {}", req.output_json_path),
        ));
    }

    let json_text = std::fs::read_to_string(&output_path)
        .map_err(|e| ErrorResponse::io_error(format!("Failed to read result file: {}", e)))?;

    let payload: DriverUiImportPayload = serde_json::from_str(&json_text).map_err(|e| {
        ErrorResponse::new(
            "INVALID_JSON",
            format!("Failed to parse driver UI JSON: {}", e),
        )
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
