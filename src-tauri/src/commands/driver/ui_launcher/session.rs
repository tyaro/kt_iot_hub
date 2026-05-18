use super::paths::{resolve_driver_ui_path_for_type, resolve_driver_ui_path_with_base};
use crate::app_state::{AppState, DriverUiSessionState};
use crate::commands::dto::ErrorResponse;
use crate::config::DriverConfig;

pub(super) struct ResolvedLaunchTarget {
    pub driver_id: Option<String>,
    pub driver_type: String,
    pub executable_path: String,
}

pub(super) fn resolve_launch_target(
    requested_driver_id: Option<String>,
    requested_driver_type: Option<String>,
    requested_driver_ui_base_dir: Option<String>,
    driver_configs: &[DriverConfig],
) -> Result<ResolvedLaunchTarget, ErrorResponse> {
    if requested_driver_id.is_none() && requested_driver_type.is_none() {
        return Err(ErrorResponse::invalid_input(
            "driver_id or driver_type is required",
        ));
    }

    if let Some(driver_id) = requested_driver_id {
        let driver_config = driver_configs
            .iter()
            .find(|cfg| cfg.id == driver_id)
            .cloned()
            .ok_or(ErrorResponse::not_found(format!(
                "Driver not found: {}",
                driver_id
            )))?;

        let executable_path = resolve_driver_ui_path_with_base(
            &driver_config,
            requested_driver_ui_base_dir.as_deref(),
        )
        .ok_or(ErrorResponse::new(
            "NOT_CONFIGURED",
            format!(
                "Driver UI executable not found for driver {} (place it under driver-ui/{}/registration-ui(.exe))",
                driver_id, driver_config.driver_type
            ),
        ))?;

        return Ok(ResolvedLaunchTarget {
            driver_id: Some(driver_id),
            driver_type: driver_config.driver_type,
            executable_path,
        });
    }

    let driver_type = requested_driver_type.ok_or(ErrorResponse::invalid_input(
        "driver_type is required when driver_id is omitted",
    ))?;

    let executable_path = resolve_driver_ui_path_for_type(
        driver_configs,
        &driver_type,
        requested_driver_ui_base_dir.as_deref(),
    )
    .ok_or(ErrorResponse::new(
        "NOT_CONFIGURED",
        format!(
            "Driver UI executable not found for driver type {} (place it under driver-ui/{}/registration-ui(.exe))",
            driver_type, driver_type
        ),
    ))?;

    Ok(ResolvedLaunchTarget {
        driver_id: None,
        driver_type,
        executable_path,
    })
}

pub(super) async fn register_active_session(
    state: &tauri::State<'_, AppState>,
    session_id: &str,
    driver_id: Option<String>,
    driver_type: String,
) -> Result<(), ErrorResponse> {
    let mut sessions = state.active_driver_ui_sessions.write().await;
    if let Some(existing_driver_id) = driver_id.as_ref() {
        if sessions.values().any(|session| {
            session.process_active && session.target_driver_id.as_ref() == Some(existing_driver_id)
        }) {
            return Err(ErrorResponse::new(
                "ALREADY_RUNNING",
                format!(
                    "Driver UI is already active for driver {}",
                    existing_driver_id
                ),
            ));
        }
    }

    sessions.insert(
        session_id.to_string(),
        DriverUiSessionState {
            target_driver_id: driver_id,
            driver_type,
            process_active: true,
        },
    );

    Ok(())
}
