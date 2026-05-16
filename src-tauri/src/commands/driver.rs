use super::dto::{DriverDto, ErrorResponse, SaveDriverRequest};
use crate::app_state::AppState;
use crate::config::DriverConfig;
use crate::drivers::postgres::PostgresDriver;
use crate::config::ScanGroupConfig;

#[tauri::command]
pub async fn list_drivers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DriverDto>, ErrorResponse> {
    let configs = state.driver_configs.read().await;
    Ok(configs
        .iter()
        .map(|cfg| DriverDto {
            id: cfg.id.clone(),
            driver_type: cfg.driver_type.clone(),
            enabled: cfg.enabled.unwrap_or(true),
            host: cfg
                .settings
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: cfg
                .settings
                .get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(5432) as u16,
            database: cfg
                .settings
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            username: cfg
                .settings
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn save_driver(
    state: tauri::State<'_, AppState>,
    req: SaveDriverRequest,
) -> Result<(), ErrorResponse> {
    if req.id.trim().is_empty() {
        return Err(ErrorResponse {
            error: "Driver ID cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let config = DriverConfig {
        id: req.id.clone(),
        driver_type: req.driver_type.clone(),
        enabled: Some(req.enabled),
        settings: serde_json::json!({
            "host": req.host,
            "port": req.port,
            "database": req.database,
            "username": req.username,
            "password": req.password,
        }),
    };

    {
        let mut configs = state.driver_configs.write().await;
        if let Some(existing) = configs.iter_mut().find(|d| d.id == config.id) {
            *existing = config.clone();
        } else {
            configs.push(config.clone());
        }
    }

    if config.driver_type == "postgres" {
        let mut manager = state.drivers.write().await;
        if manager.contains(&config.id) {
            let _ = manager.stop_driver(&config.id).await;
        }

        let scan_groups: Vec<ScanGroupConfig> = state.scan_groups.read().await
            .iter()
            .filter(|g| g.driver == config.id)
            .cloned()
            .collect();
        manager.register(Box::new(PostgresDriver::new(config.clone(), scan_groups)));

        if config.enabled.unwrap_or(true) {
            manager
                .start_driver(&config.id, &state.registry, &state.tag_bus)
                .await
                .map_err(ErrorResponse::from)?;
        }
    }

    Ok(())
}
