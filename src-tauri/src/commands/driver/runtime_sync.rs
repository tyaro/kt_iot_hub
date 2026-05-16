//! ドライバ設定の変更後に DriverManager を最新化する処理。

use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::{DriverConfig, ScanGroupConfig};
use crate::drivers::postgres::PostgresDriver;

/// 現状は PostgreSQL ドライバのみリアルタイム反映する。
/// 他ドライバが追加されたらここに分岐を増やす。
pub(super) async fn sync_driver_runtime(
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
