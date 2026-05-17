use super::dto::{ErrorResponse, RuntimeStatusDto, StartRuntimeServicesRequest};
use crate::app_state::AppState;
use crate::grpc;

const GRPC_ADDR: &str = grpc::tag_registration::DEFAULT_GRPC_ADDR;

#[tauri::command]
pub async fn get_runtime_status(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    Ok(read_runtime_status(state.inner()).await)
}

#[tauri::command]
pub async fn start_runtime_services(
    state: tauri::State<'_, AppState>,
    req: Option<StartRuntimeServicesRequest>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    clear_last_error(state.inner()).await;
    let driver_ui_base_dir = req.and_then(|req| normalize_optional_string(req.driver_ui_base_dir));

    if let Some(driver_ui_base_dir) = driver_ui_base_dir.clone() {
        *state.driver_ui_base_dir.write().await = Some(driver_ui_base_dir);
    }

    if let Err(e) = start_drivers(state.inner(), driver_ui_base_dir.as_deref()).await {
        return Err(e);
    }
    if let Err(e) = start_publishers(state.inner(), false).await {
        let _ = stop_drivers(state.inner()).await;
        return Err(e);
    }
    Ok(read_runtime_status(state.inner()).await)
}

#[tauri::command]
pub async fn stop_runtime_services(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    stop_publishers(state.inner()).await?;
    stop_drivers(state.inner()).await?;
    Ok(read_runtime_status(state.inner()).await)
}

pub async fn auto_start_runtime_services(state: &AppState) -> Result<(), ErrorResponse> {
    let should_auto_start = state
        .publisher_configs
        .read()
        .await
        .iter()
        .any(|cfg| cfg.enabled.unwrap_or(false));

    if !should_auto_start {
        return Ok(());
    }

    clear_last_error(state).await;

    if let Err(e) = start_drivers(state, None).await {
        return Err(e);
    }
    if let Err(e) = start_publishers(state, true).await {
        let _ = stop_drivers(state).await;
        return Err(e);
    }

    Ok(())
}

async fn start_drivers(state: &AppState, driver_ui_base_dir: Option<&str>) -> Result<(), ErrorResponse> {
    let drivers: Vec<(String, String)> = state
        .driver_configs
        .read()
        .await
        .iter()
        .filter(|cfg| cfg.enabled.unwrap_or(true))
        .map(|cfg| (cfg.id.clone(), cfg.driver_type.clone()))
        .collect();

    let has_drivers = !drivers.is_empty();
    let mut manager = state.drivers.write().await;
    for (driver_id, driver_type) in drivers {
        manager
            .start_driver(&driver_id, &driver_type, driver_ui_base_dir)
            .await
            .map_err(ErrorResponse::from)?;
    }

    state.runtime_status.write().await.drivers_running = has_drivers && !manager.running_driver_ids().is_empty();
    Ok(())
}

async fn stop_drivers(state: &AppState) -> Result<(), ErrorResponse> {
    let mut manager = state.drivers.write().await;
    manager.stop_all().await.map_err(ErrorResponse::from)?;
    state.runtime_status.write().await.drivers_running = false;
    Ok(())
}

async fn start_publishers(state: &AppState, startup_only: bool) -> Result<(), ErrorResponse> {
    let publisher_ids: Vec<String> = state
        .publisher_configs
        .read()
        .await
        .iter()
        .filter(|cfg| !startup_only || cfg.enabled.unwrap_or(false))
        .map(|cfg| cfg.id.clone())
        .collect();

    let has_publishers = !publisher_ids.is_empty();
    let mut manager = state.publishers.write().await;
    for publisher_id in publisher_ids {
        manager
            .start_publisher(&publisher_id, &state.registry, &state.tag_bus)
            .await
            .map_err(ErrorResponse::from)?;
    }

    state.runtime_status.write().await.publishers_running = has_publishers;
    Ok(())
}

async fn stop_publishers(state: &AppState) -> Result<(), ErrorResponse> {
    let mut manager = state.publishers.write().await;
    manager.stop_all().await.map_err(ErrorResponse::from)?;
    state.runtime_status.write().await.publishers_running = false;
    Ok(())
}

pub async fn start_grpc_server(state: &AppState) -> Result<(), ErrorResponse> {
    if state.runtime_status.read().await.grpc_running {
        return Ok(());
    }

    let listener = match std::net::TcpListener::bind(GRPC_ADDR) {
        Ok(listener) => listener,
        Err(e) => {
            let message = format!("Failed to bind gRPC server {}: {}", GRPC_ADDR, e);
            state.runtime_status.write().await.last_error = Some(message.clone());
            return Err(ErrorResponse {
                error: message,
                code: "GRPC_BIND_FAILED".to_string(),
            });
        }
    };
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    {
        let mut grpc_shutdown = state.grpc_shutdown_tx.write().await;
        *grpc_shutdown = Some(shutdown_tx);
    }
    {
        let mut runtime_status = state.runtime_status.write().await;
        runtime_status.grpc_running = true;
        runtime_status.last_error = None;
    }

    let app_state = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = grpc::tag_registration::serve(app_state.clone(), GRPC_ADDR, shutdown_rx).await {
            tracing::error!("gRPC server stopped with error: {}", e);
            let mut runtime_status = app_state.runtime_status.write().await;
            runtime_status.last_error = Some(format!("gRPC transport error: {}", e));
            runtime_status.grpc_running = false;
        } else {
            let mut runtime_status = app_state.runtime_status.write().await;
            runtime_status.grpc_running = false;
        }

        let mut shutdown = app_state.grpc_shutdown_tx.write().await;
        *shutdown = None;
    });

    Ok(())
}

async fn clear_last_error(state: &AppState) {
    state.runtime_status.write().await.last_error = None;
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

async fn read_runtime_status(state: &AppState) -> RuntimeStatusDto {
    let runtime_status = state.runtime_status.read().await.clone();
    let grpc_running = if runtime_status.grpc_running {
        true
    } else if state.grpc_shutdown_tx.read().await.is_some() {
        true
    } else {
        tokio::net::TcpStream::connect(GRPC_ADDR).await.is_ok()
    };

    if grpc_running && !runtime_status.grpc_running {
        state.runtime_status.write().await.grpc_running = true;
    }

    RuntimeStatusDto {
        drivers_running: runtime_status.drivers_running,
        publishers_running: runtime_status.publishers_running,
        grpc_running,
        last_error: runtime_status.last_error,
    }
}
