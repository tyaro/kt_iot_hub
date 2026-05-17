//! ドライバ設定の変更後に DriverProcessManager の状態を同期する処理。

use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::DriverConfig;

pub(super) async fn sync_driver_runtime(
    state: &tauri::State<'_, AppState>,
    config: &DriverConfig,
) -> Result<(), ErrorResponse> {
    let runtime_running = state.runtime_status.read().await.drivers_running;
    let mut manager = state.drivers.write().await;

    if manager.is_running(&config.id) {
        let _ = manager.stop_driver(&config.id).await;
    }

    if runtime_running && config.enabled.unwrap_or(true) {
        manager
            .start_driver(&config.id, &config.driver_type)
            .await
            .map_err(ErrorResponse::from)?;
    }

    Ok(())
}
