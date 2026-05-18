//! ドライバ設定の変更後に DriverProcessManager の状態を同期する処理。

use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::DriverConfig;
use tracing::info;

pub(super) async fn sync_driver_runtime(
    state: &tauri::State<'_, AppState>,
    config: &DriverConfig,
) -> Result<(), ErrorResponse> {
    let runtime_status = state.runtime_status.read().await.clone();
    let should_keep_driver_running = runtime_status.drivers_running || runtime_status.publishers_running;
    let driver_ui_base_dir = state.driver_ui_base_dir.read().await.clone();
    let mut manager = state.drivers.write().await;
    let was_running = manager.is_running(&config.id);

    if was_running {
        info!("Driver config changed while runtime active; restarting driver: {}", config.id);
        let _ = manager.stop_driver(&config.id).await;
    }

    if should_keep_driver_running && config.enabled.unwrap_or(true) {
        manager
            .start_driver(&config.id, &config.driver_type, driver_ui_base_dir.as_deref())
            .await
            .map_err(ErrorResponse::from)?;

        state.runtime_status.write().await.drivers_running = true;
    }

    Ok(())
}
