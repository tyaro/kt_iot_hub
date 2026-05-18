use crate::commands::dto::ErrorResponse;
use tauri::Manager;

#[tauri::command]
pub async fn open_mqtt_monitor_window(app: tauri::AppHandle) -> Result<(), ErrorResponse> {
    if let Some(window) = app.get_webview_window("mqtt-monitor") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        "mqtt-monitor",
        tauri::WebviewUrl::App("/?view=mqtt-monitor".into()),
    )
    .title("MQTT Monitor")
    .inner_size(1180.0, 760.0)
    .min_inner_size(920.0, 620.0)
    .resizable(true)
    .build()
    .map_err(|e| ErrorResponse {
        error: format!("Failed to open MQTT monitor window: {}", e),
        code: "WINDOW_OPEN_FAILED".to_string(),
    })?;

    Ok(())
}
