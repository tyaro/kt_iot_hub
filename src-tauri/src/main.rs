// Tauri 設定・ウィンドウ管理
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_logs;
mod app_state;
mod commands;
mod config;
mod core;
mod drivers;
mod grpc;
mod publishers;
mod subscribers;

use app_state::AppState;
use config::AppConfig;
use core::{DataType, Tag, TagBus, TagId, TagRegistry};
use drivers::DriverProcessManager;
use publishers::mqtt::MqttPublisher;
use publishers::PublisherManager;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tracing::info;

static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn is_driver_ui_process() -> bool {
    std::env::args().any(|arg| arg == "--driver-ui-mode")
}

fn main() {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .with_writer(app_logs::AppLogMakeWriter)
        .init();

    info!("kt_iot_hub starting...");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::tag::create_tag,
            commands::tag::list_tags,
            commands::tag::list_scan_groups,
            commands::tag::delete_tag,
            commands::driver::crud::list_drivers,
            commands::driver::crud::save_driver,
            commands::driver::crud::delete_driver,
            commands::driver::ui_launcher::launch_driver_ui,
            commands::driver::ui_launcher::check_driver_ui_available,
            commands::driver::ui_launcher::check_driver_ui_result,
            commands::driver::import::import_driver_ui_result,
            commands::publisher::crud::list_publishers,
            commands::publisher::crud::save_publisher,
            commands::subscriber::monitor::open_mqtt_monitor_window,
            commands::subscriber::monitor::list_mqtt_monitor_publishers,
            commands::subscriber::monitor::get_mqtt_monitor_status,
            commands::subscriber::monitor::list_mqtt_monitor_messages,
            commands::subscriber::monitor::get_mqtt_monitor_tree,
            commands::subscriber::monitor::get_mqtt_monitor_topic_detail,
            commands::subscriber::monitor::clear_mqtt_monitor_messages,
            commands::subscriber::monitor::start_mqtt_monitor,
            commands::subscriber::monitor::stop_mqtt_monitor,
            kt_driver_ui_host::bridge::get_driver_ui_launch_context,
            kt_driver_ui_host::bridge::save_driver_ui_output,
            kt_driver_ui_host::postgres::postgres_test_connection,
            kt_driver_ui_host::postgres::postgres_list_tables,
            kt_driver_ui_host::postgres::postgres_list_columns,
            commands::runtime::get_runtime_status,
            commands::runtime::start_runtime_services,
            commands::runtime::stop_runtime_services,
            commands::metrics::get_app_metrics,
            commands::metrics::get_driver_metrics,
            commands::logs::list_app_logs,
            commands::logs::clear_app_logs,
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

            // gRPCアドレスは tag_registration と同じポートを使用
            let grpc_addr = grpc::tag_registration::DEFAULT_GRPC_ADDR;
            let driver_process_manager = DriverProcessManager::new(grpc_addr);

            let mut publisher_manager = PublisherManager::new();
            for p in &config.publishers {
                if p.publisher_type == "mqtt" {
                    publisher_manager.register(Box::new(MqttPublisher::new(p.clone())));
                }
            }

            let app_state = AppState::new(
                registry,
                tag_bus,
                driver_process_manager,
                publisher_manager,
                config.drivers,
                config.publishers,
                config.scan_groups,
            );

            app.manage(app_state.clone());

            if !is_driver_ui_process() {
                tauri::async_runtime::block_on(async {
                    commands::runtime::start_grpc_server(&app_state)
                        .await
                        .map_err(|e| std::io::Error::other(e.error.clone()))?;

                    if let Err(e) = commands::runtime::auto_start_runtime_services(&app_state).await
                    {
                        tracing::error!("Failed to auto start runtime services: {}", e.error);
                        app_state.runtime_status.write().await.last_error =
                            Some(format!("Auto start failed: {}", e.error));
                    }

                    Ok::<(), std::io::Error>(())
                })
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            } else {
                info!("Driver UI mode detected: skip gRPC startup");
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
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
            let mut monitor = state.mqtt_monitor.lock().await;
            if let Err(e) = monitor.stop(state.mqtt_monitor_status.clone()).await {
                tracing::warn!("Failed to stop MQTT monitor cleanly: {}", e);
            }
        }

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
