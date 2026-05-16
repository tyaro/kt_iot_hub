use super::dto::{ErrorResponse, RuntimeStatusDto};
use crate::app_state::AppState;
use crate::grpc;

const GRPC_ADDR: &str = grpc::tag_registration::DEFAULT_GRPC_ADDR;

#[tauri::command]
pub async fn get_runtime_status(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    Ok(read_runtime_status(&state).await)
}

#[tauri::command]
pub async fn start_runtime_services(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    clear_last_error(&state).await;
    if let Err(e) = start_drivers(&state).await {
        return Err(e);
    }
    if let Err(e) = start_publishers(&state).await {
        let _ = stop_drivers(&state).await;
        return Err(e);
    }
    Ok(read_runtime_status(&state).await)
}

#[tauri::command]
pub async fn stop_runtime_services(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeStatusDto, ErrorResponse> {
    stop_publishers(&state).await?;
    stop_drivers(&state).await?;
    stop_grpc_server(&state).await;
    Ok(read_runtime_status(&state).await)
}

async fn start_drivers(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
    let driver_ids: Vec<String> = state
        .driver_configs
        .read()
        .await
        .iter()
        .filter(|cfg| cfg.enabled.unwrap_or(true))
        .map(|cfg| cfg.id.clone())
        .collect();

    let mut manager = state.drivers.write().await;
    for driver_id in driver_ids {
        manager
            .start_driver(&driver_id, &state.registry, &state.tag_bus)
            .await
            .map_err(ErrorResponse::from)?;
    }

    state.runtime_status.write().await.drivers_running = true;
    Ok(())
}

async fn stop_drivers(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
    let mut manager = state.drivers.write().await;
    manager.stop_all().await.map_err(ErrorResponse::from)?;
    state.runtime_status.write().await.drivers_running = false;
    Ok(())
}

async fn start_publishers(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
    let publisher_ids: Vec<String> = state
        .publisher_configs
        .read()
        .await
        .iter()
        .filter(|cfg| cfg.enabled.unwrap_or(true))
        .map(|cfg| cfg.id.clone())
        .collect();

    let mut manager = state.publishers.write().await;
    for publisher_id in publisher_ids {
        manager
            .start_publisher(&publisher_id, &state.registry, &state.tag_bus)
            .await
            .map_err(ErrorResponse::from)?;
    }

    state.runtime_status.write().await.publishers_running = true;
    Ok(())
}

async fn stop_publishers(state: &tauri::State<'_, AppState>) -> Result<(), ErrorResponse> {
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

async fn stop_grpc_server(state: &tauri::State<'_, AppState>) {
    if let Some(shutdown_tx) = state.grpc_shutdown_tx.write().await.take() {
        let _ = shutdown_tx.send(());
    }
    state.runtime_status.write().await.grpc_running = false;
}

async fn clear_last_error(state: &tauri::State<'_, AppState>) {
    state.runtime_status.write().await.last_error = None;
}

async fn read_runtime_status(state: &tauri::State<'_, AppState>) -> RuntimeStatusDto {
    let runtime_status = state.runtime_status.read().await.clone();
    RuntimeStatusDto {
        drivers_running: runtime_status.drivers_running,
        publishers_running: runtime_status.publishers_running,
        grpc_running: runtime_status.grpc_running,
        last_error: runtime_status.last_error,
    }
}
