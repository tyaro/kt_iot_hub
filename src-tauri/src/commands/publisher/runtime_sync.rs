use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use crate::config::PublisherConfig;
use crate::publishers::mqtt::MqttPublisher;

pub(super) async fn sync_publisher_runtime(
    state: &tauri::State<'_, AppState>,
    config: &PublisherConfig,
) -> Result<(), ErrorResponse> {
    let runtime_running = state.runtime_status.read().await.publishers_running;
    let mut manager = state.publishers.write().await;

    if manager.contains(&config.id) {
        let _ = manager.stop_publisher(&config.id).await;
        let _ = manager.unregister(&config.id);
    }

    match config.publisher_type.as_str() {
        "mqtt" => {
            manager.register(Box::new(MqttPublisher::new(config.clone())));
        }
        other => {
            return Err(ErrorResponse::invalid_input(format!(
                "Unsupported publisher_type: {}",
                other
            )));
        }
    }

    if runtime_running {
        manager
            .start_publisher(&config.id, &state.registry, &state.tag_bus)
            .await
            .map_err(ErrorResponse::from)?;
    }

    Ok(())
}
