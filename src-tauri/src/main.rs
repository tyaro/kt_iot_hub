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
use std::sync::atomic::{AtomicBool, Ordering};
use std::str::FromStr;
use config::ScanGroupConfig;
use tauri::Manager;
use tracing::info;

static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

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
            commands::tag::list_scan_groups,
            commands::tag::delete_tag,
            commands::driver::list_drivers,
            commands::driver::save_driver,
            commands::driver::delete_driver,
            commands::driver::launch_driver_ui,
            commands::driver::check_driver_ui_result,
            commands::driver::import_driver_ui_result,
            commands::driver_ui_bridge::get_driver_ui_launch_context,
            commands::driver_ui_bridge::save_driver_ui_output,
            commands::postgres_registration::postgres_test_connection,
            commands::postgres_registration::postgres_list_tables,
            commands::postgres_registration::postgres_list_columns,
            commands::runtime::get_runtime_status,
            commands::runtime::start_runtime_services,
            commands::runtime::stop_runtime_services,
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
                }
            }

            let mut publisher_manager = PublisherManager::new();
            for p in &config.publishers {
                if p.publisher_type == "mqtt" {
                    publisher_manager.register(Box::new(MqttPublisher::new(p.clone())));
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

            app.manage(app_state.clone());

            let grpc_state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::runtime::start_grpc_server(&grpc_state).await {
                    tracing::error!("Failed to initialize gRPC server: {}", e.error);
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if SHUTDOWN_IN_PROGRESS.load(Ordering::SeqCst) {
                    return;
                }
                api.prevent_exit();
                SHUTDOWN_IN_PROGRESS.store(true, Ordering::SeqCst);

                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    graceful_shutdown(app_handle).await;
                });
            }
            _ => {}
        }
    });
}

async fn graceful_shutdown(app_handle: tauri::AppHandle) {
    info!("Graceful shutdown requested");

    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Some(shutdown_tx) = state.grpc_shutdown_tx.write().await.take() {
            let _ = shutdown_tx.send(());
        }
        state.runtime_status.write().await.grpc_running = false;

        {
            let mut publishers = state.publishers.write().await;
            if let Err(e) = publishers.stop_all().await {
                tracing::warn!("Failed to stop publishers cleanly: {}", e);
            }
        }
        state.runtime_status.write().await.publishers_running = false;

        {
            let mut drivers = state.drivers.write().await;
            if let Err(e) = drivers.stop_all().await {
                tracing::warn!("Failed to stop drivers cleanly: {}", e);
            }
        }
        state.runtime_status.write().await.drivers_running = false;
    } else {
        tracing::warn!("AppState not available during shutdown");
    }

    info!("Exiting application");
    app_handle.exit(0);
}
