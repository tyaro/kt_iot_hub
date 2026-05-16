// Tauri 設定・ウィンドウ管理
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod config;
mod core;
mod drivers;
mod grpc;
mod publishers;

use app_state::AppState;
use config::AppConfig;
use core::{DataType, Tag, TagBus, TagId, TagRegistry};
use drivers::postgres::PostgresDriver;
use drivers::DriverManager;
use publishers::mqtt::MqttPublisher;
use publishers::PublisherManager;
use std::str::FromStr;
use config::ScanGroupConfig;
use tauri::Manager;
use tracing::info;

fn main() {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .init();

    info!("kt_iot_hub starting...");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::tag::create_tag,
            commands::tag::list_tags,
            commands::tag::delete_tag,
            commands::driver::list_drivers,
            commands::driver::save_driver,
        ])
        .setup(|app| {
            info!("Tauri setup beginning");
            let config = AppConfig::load_from_files("../config")
                .or_else(|_| AppConfig::load_from_files("config"))
                .map_err(|e| -> Box<dyn std::error::Error> {
                    Box::new(std::io::Error::other(e.to_string()))
                })?;

            let registry = TagRegistry::new();
            let tag_bus = TagBus::new();

            tauri::async_runtime::block_on(async {
                for tag_cfg in &config.tags {
                    let data_type = match DataType::from_str(&tag_cfg.data_type) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!("Skipping tag {}: {}", tag_cfg.id, e);
                            continue;
                        }
                    };
                    registry
                        .insert(Tag {
                            id: TagId(tag_cfg.id.clone()),
                            name: tag_cfg.name.clone(),
                            data_type,
                            driver_id: tag_cfg.driver.clone(),
                            scan_group_id: tag_cfg.scan_group.clone(),
                            driver_spec: tag_cfg.driver_spec.clone(),
                            metadata: tag_cfg.metadata.clone(),
                        })
                        .await;
                }
            });

            let mut driver_manager = DriverManager::new();
            for d in &config.drivers {
                if d.driver_type == "postgres" {
                    let groups: Vec<ScanGroupConfig> = config
                        .scan_groups
                        .iter()
                        .filter(|g| g.driver == d.id)
                        .cloned()
                        .collect();
                    driver_manager.register(Box::new(PostgresDriver::new(d.clone(), groups)));
                    if d.enabled.unwrap_or(true) {
                        let _ = tauri::async_runtime::block_on(async {
                            driver_manager.start_driver(&d.id, &registry, &tag_bus).await
                        });
                    }
                }
            }

            let mut publisher_manager = PublisherManager::new();
            for p in &config.publishers {
                if p.publisher_type == "mqtt" {
                    publisher_manager.register(Box::new(MqttPublisher::new(p.clone())));
                    if p.enabled.unwrap_or(true) {
                        let _ = tauri::async_runtime::block_on(async {
                            publisher_manager.start_publisher(&p.id, &registry, &tag_bus).await
                        });
                    }
                }
            }

            let app_state = AppState::new(
                registry,
                tag_bus,
                driver_manager,
                publisher_manager,
                config.drivers,
                config.publishers,
                config.scan_groups,
            );

            let grpc_state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = grpc::tag_registration::serve(grpc_state, "127.0.0.1:50051").await
                {
                    tracing::error!("gRPC server stopped with error: {}", e);
                }
            });

            app.manage(app_state);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                // グレースフルシャットダウン
                api.prevent_exit();
            }
            _ => {}
        }
    });
}
