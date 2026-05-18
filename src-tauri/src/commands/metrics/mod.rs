use super::dto::{AppMetricsDto, DriverMetricsDto, ErrorResponse};
use crate::app_state::{AppState, DriverIoSampleState};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use sysinfo::{Networks, Pid, ProcessesToUpdate, System};

#[cfg(not(target_os = "windows"))]
mod fallback;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
use fallback::get_process_metrics;
#[cfg(target_os = "windows")]
use windows::get_process_metrics;

#[tauri::command]
pub async fn get_app_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<AppMetricsDto, ErrorResponse> {
    let mut system = System::new_all();
    system.refresh_cpu_usage();
    system.refresh_memory();
    let logical_cpu_count = system.cpus().len().max(1);

    let (process_cpu_percent, process_memory_bytes) = {
        let mut cache = state.runtime_metrics_cache.write().await;
        get_process_metrics(&mut cache, logical_cpu_count)?
    };

    let system_cpu_percent = Some(system.global_cpu_usage());
    let system_memory_total_bytes = Some(system.total_memory());
    let system_memory_used_bytes = Some(system.used_memory());

    let mut networks = Networks::new_with_refreshed_list();
    networks.refresh(true);

    let total_rx: u64 = networks.values().map(|net| net.total_received()).sum();
    let total_tx: u64 = networks.values().map(|net| net.total_transmitted()).sum();

    let sampled_at = Utc::now();
    let (network_rx_bytes_per_sec, network_tx_bytes_per_sec) = {
        let mut cache = state.runtime_metrics_cache.write().await;

        let rates = if let (Some(last_rx), Some(last_tx), Some(last_sampled_at)) = (
            cache.last_network_rx_bytes,
            cache.last_network_tx_bytes,
            cache.last_sampled_at,
        ) {
            let elapsed_sec =
                sampled_at.signed_duration_since(last_sampled_at).num_milliseconds() as f64 / 1000.0;

            if elapsed_sec > 0.0 {
                let rx_bps = (total_rx.saturating_sub(last_rx) as f64) / elapsed_sec;
                let tx_bps = (total_tx.saturating_sub(last_tx) as f64) / elapsed_sec;
                (Some(rx_bps.max(0.0)), Some(tx_bps.max(0.0)))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        cache.last_network_rx_bytes = Some(total_rx);
        cache.last_network_tx_bytes = Some(total_tx);
        cache.last_sampled_at = Some(sampled_at);

        rates
    };

    Ok(AppMetricsDto {
        process_cpu_percent,
        process_memory_bytes,
        system_cpu_percent,
        system_memory_used_bytes,
        system_memory_total_bytes,
        network_rx_bytes_per_sec,
        network_tx_bytes_per_sec,
        sampled_at: sampled_at.to_rfc3339(),
    })
}

#[tauri::command]
pub async fn get_driver_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DriverMetricsDto>, ErrorResponse> {
    let running = {
        let drivers = state.drivers.read().await;
        drivers.running_driver_processes()
    };

    if running.is_empty() {
        return Ok(Vec::new());
    }

    let driver_types: HashMap<String, String> = state
        .driver_configs
        .read()
        .await
        .iter()
        .map(|cfg| (cfg.id.clone(), cfg.driver_type.clone()))
        .collect();

    let pids: Vec<Pid> = running.iter().map(|(_, pid)| Pid::from_u32(*pid)).collect();

    let mut system = System::new_all();
    system.refresh_cpu_usage();
    let _ = system.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    let logical_cpu_count = system.cpus().len().max(1) as f32;

    let process_stats: Vec<(String, u32, Option<f32>, Option<u64>)> = running
        .iter()
        .map(|(driver_id, pid_u32)| {
            let process = system.process(Pid::from_u32(*pid_u32));
            (
                driver_id.clone(),
                *pid_u32,
                process.map(|p| (p.cpu_usage() / logical_cpu_count).clamp(0.0, 100.0)),
                process.map(|p| p.memory()),
            )
        })
        .collect();

    let mut cache = state.runtime_metrics_cache.write().await;
    let active_driver_ids: HashSet<String> = process_stats
        .iter()
        .map(|(driver_id, _, _, _)| driver_id.clone())
        .collect();
    cache
        .last_driver_reported_io_samples
        .retain(|driver_id, _| active_driver_ids.contains(driver_id));
    cache
        .last_driver_reported_io_totals
        .retain(|driver_id, _| active_driver_ids.contains(driver_id));

    let metrics = process_stats
        .into_iter()
        .map(|(driver_id, pid_u32, cpu_percent, memory_bytes)| {
            let (network_rx_bytes_per_sec, network_tx_bytes_per_sec) =
                if let Some((current_rx_total, current_tx_total, current_sampled_at)) = cache
                    .last_driver_reported_io_totals
                    .get(&driver_id)
                    .map(|state| (state.rx_bytes_total, state.tx_bytes_total, state.sampled_at))
                {
                    let rates = cache
                        .last_driver_reported_io_samples
                        .get(&driver_id)
                        .and_then(|prev| {
                            let elapsed_sec = current_sampled_at
                                .signed_duration_since(prev.sampled_at)
                                .num_milliseconds() as f64
                                / 1000.0;
                            if elapsed_sec <= 0.0 {
                                return None;
                            }

                            let rx_delta = if current_rx_total >= prev.read_bytes {
                                current_rx_total - prev.read_bytes
                            } else {
                                current_rx_total
                            };
                            let tx_delta = if current_tx_total >= prev.write_bytes {
                                current_tx_total - prev.write_bytes
                            } else {
                                current_tx_total
                            };

                            let rx = (rx_delta as f64 / elapsed_sec).max(0.0);
                            let tx = (tx_delta as f64 / elapsed_sec).max(0.0);
                            Some((rx, tx))
                        });

                    cache.last_driver_reported_io_samples.insert(
                        driver_id.clone(),
                        DriverIoSampleState {
                            read_bytes: current_rx_total,
                            write_bytes: current_tx_total,
                            sampled_at: current_sampled_at,
                        },
                    );

                    rates.unwrap_or((0.0, 0.0))
                } else {
                    (0.0, 0.0)
                };

            DriverMetricsDto {
                driver_id: driver_id.clone(),
                driver_type: driver_types
                    .get(&driver_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
                pid: pid_u32,
                cpu_percent,
                memory_bytes,
                network_rx_bytes_per_sec: Some(network_rx_bytes_per_sec),
                network_tx_bytes_per_sec: Some(network_tx_bytes_per_sec),
            }
        })
        .collect();

    Ok(metrics)
}
