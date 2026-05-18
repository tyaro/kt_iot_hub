use super::dto::{AppMetricsDto, ErrorResponse};
use crate::app_state::AppState;
use chrono::Utc;
use sysinfo::{Networks, Pid, ProcessesToUpdate, System};

#[tauri::command]
pub async fn get_app_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<AppMetricsDto, ErrorResponse> {
    let mut system = System::new_all();
    let current_pid: Pid = sysinfo::get_current_pid().map_err(|e| ErrorResponse {
        error: format!("Failed to get current pid: {}", e),
        code: "METRICS_PID_ERROR".to_string(),
    })?;

    system.refresh_cpu_usage();
    let _ = system.refresh_processes(ProcessesToUpdate::Some(&[current_pid]), true);
    system.refresh_memory();

    let process = system.process(current_pid);
    let process_cpu_percent = process.map(|p| p.cpu_usage());
    let process_memory_bytes = process.map(|p| p.memory());

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
