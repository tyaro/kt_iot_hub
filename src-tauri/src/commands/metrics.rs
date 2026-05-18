use super::dto::{AppMetricsDto, DriverMetricsDto, ErrorResponse};
use crate::app_state::{AppCpuSampleState, AppState, DriverIoSampleState, RuntimeMetricsCacheState};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use sysinfo::{Networks, Pid, ProcessesToUpdate, System};
#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{
    ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
    Threading::{GetCurrentProcess, GetProcessTimes},
    },
};

#[cfg(target_os = "windows")]
fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

#[cfg(target_os = "windows")]
fn get_windows_process_metrics(
    cache: &mut RuntimeMetricsCacheState,
    logical_cpu_count: usize,
) -> Result<(Option<f32>, Option<u64>), ErrorResponse> {
    unsafe {
        let process_handle = GetCurrentProcess();

        let mut creation_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut process_kernel_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut process_user_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };

        if GetProcessTimes(
            process_handle,
            &mut creation_time,
            &mut exit_time,
            &mut process_kernel_time,
            &mut process_user_time,
        ) == 0
        {
            return Err(ErrorResponse {
                error: "GetProcessTimes failed".to_string(),
                code: "METRICS_PROCESS_TIMES_ERROR".to_string(),
            });
        }

        let current_sample = AppCpuSampleState {
            process_kernel_time: filetime_to_u64(process_kernel_time),
            process_user_time: filetime_to_u64(process_user_time),
            sampled_at: Utc::now(),
        };

        let process_cpu_percent = cache.last_app_cpu_sample.as_ref().and_then(|previous| {
            let process_delta = current_sample
                .process_kernel_time
                .saturating_sub(previous.process_kernel_time)
                + current_sample
                    .process_user_time
                    .saturating_sub(previous.process_user_time);
            let elapsed_100ns = current_sample
                .sampled_at
                .signed_duration_since(previous.sampled_at)
                .num_nanoseconds()
                .map(|ns| (ns / 100) as u64)
                .unwrap_or(0);

            if elapsed_100ns == 0 || logical_cpu_count == 0 {
                None
            } else {
                let normalized = (process_delta as f64)
                    / (elapsed_100ns as f64 * logical_cpu_count as f64)
                    * 100.0;
                Some(normalized.clamp(0.0, 100.0) as f32)
            }
        });

        cache.last_app_cpu_sample = Some(current_sample);

        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            PageFaultCount: 0,
            PeakWorkingSetSize: 0,
            WorkingSetSize: 0,
            QuotaPeakPagedPoolUsage: 0,
            QuotaPagedPoolUsage: 0,
            QuotaPeakNonPagedPoolUsage: 0,
            QuotaNonPagedPoolUsage: 0,
            PagefileUsage: 0,
            PeakPagefileUsage: 0,
        };

        if GetProcessMemoryInfo(
            process_handle,
            &mut counters as *mut PROCESS_MEMORY_COUNTERS,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) == 0
        {
            return Err(ErrorResponse {
                error: "GetProcessMemoryInfo failed".to_string(),
                code: "METRICS_PROCESS_MEMORY_ERROR".to_string(),
            });
        }

        Ok((process_cpu_percent, Some(counters.WorkingSetSize as u64)))
    }
}

#[cfg(not(target_os = "windows"))]
fn get_windows_process_metrics(
    _cache: &mut RuntimeMetricsCacheState,
    _logical_cpu_count: usize,
) -> Result<(Option<f32>, Option<u64>), ErrorResponse> {
    Ok((None, None))
}

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
        get_windows_process_metrics(&mut cache, logical_cpu_count)?
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

        let rates = if let (
            Some(last_rx),
            Some(last_tx),
            Some(last_sampled_at),
        ) = (
            cache.last_network_rx_bytes,
            cache.last_network_tx_bytes,
            cache.last_sampled_at,
        ) {
            let elapsed_sec = sampled_at
                .signed_duration_since(last_sampled_at)
                .num_milliseconds() as f64
                / 1000.0;

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

    let pids: Vec<Pid> = running
        .iter()
        .map(|(_, pid)| Pid::from_u32(*pid))
        .collect();

    let mut system = System::new_all();
    system.refresh_cpu_usage();
    let _ = system.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    let logical_cpu_count = system.cpus().len().max(1) as f32;

    let sampled_at = Utc::now();

    let process_stats: Vec<(String, u32, Option<f32>, Option<u64>, Option<u64>, Option<u64>)> =
        running
            .iter()
            .map(|(driver_id, pid_u32)| {
                let process = system.process(Pid::from_u32(*pid_u32));
                let io = process.map(|p| p.disk_usage());
                (
                    driver_id.clone(),
                    *pid_u32,
                    process.map(|p| (p.cpu_usage() / logical_cpu_count).clamp(0.0, 100.0)),
                    process.map(|p| p.memory()),
                    io.map(|usage| usage.total_read_bytes),
                    io.map(|usage| usage.total_written_bytes),
                )
            })
            .collect();

    let mut cache = state.runtime_metrics_cache.write().await;
    let active_pids: HashSet<u32> = process_stats.iter().map(|(_, pid, _, _, _, _)| *pid).collect();
    cache
        .last_driver_io_samples
        .retain(|pid, _| active_pids.contains(pid));

    let metrics = process_stats
        .into_iter()
        .map(|(driver_id, pid_u32, cpu_percent, memory_bytes, total_read_bytes, total_written_bytes)| {
            let (network_rx_bytes_per_sec, network_tx_bytes_per_sec) = if let (
                Some(read_now),
                Some(write_now),
            ) = (total_read_bytes, total_written_bytes) {
                let rates = cache
                    .last_driver_io_samples
                    .get(&pid_u32)
                    .and_then(|prev| {
                        let elapsed_sec = sampled_at
                            .signed_duration_since(prev.sampled_at)
                            .num_milliseconds() as f64
                            / 1000.0;
                        if elapsed_sec <= 0.0 {
                            return None;
                        }
                        let rx = (read_now.saturating_sub(prev.read_bytes) as f64 / elapsed_sec).max(0.0);
                        let tx = (write_now.saturating_sub(prev.write_bytes) as f64 / elapsed_sec).max(0.0);
                        Some((rx, tx))
                    });

                cache.last_driver_io_samples.insert(
                    pid_u32,
                    DriverIoSampleState {
                        read_bytes: read_now,
                        write_bytes: write_now,
                        sampled_at,
                    },
                );

                match rates {
                    Some((rx, tx)) => (Some(rx), Some(tx)),
                    None => (None, None),
                }
            } else {
                (None, None)
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
                network_rx_bytes_per_sec,
                network_tx_bytes_per_sec,
            }
        })
        .collect();

    Ok(metrics)
}
