use crate::app_state::RuntimeMetricsCacheState;
use crate::commands::dto::ErrorResponse;
use sysinfo::System;

pub(super) fn get_process_metrics(
    _cache: &mut RuntimeMetricsCacheState,
    _logical_cpu_count: usize,
) -> Result<(Option<f32>, Option<u64>), ErrorResponse> {
    Ok((None, None))
}

pub(super) fn get_system_cpu_percent(
    _cache: &mut RuntimeMetricsCacheState,
    system: &System,
) -> Result<Option<f32>, ErrorResponse> {
    Ok(Some(system.global_cpu_usage().clamp(0.0, 100.0)))
}
