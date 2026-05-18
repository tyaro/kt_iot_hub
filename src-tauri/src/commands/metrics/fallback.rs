use crate::app_state::RuntimeMetricsCacheState;
use crate::commands::dto::ErrorResponse;

pub(super) fn get_process_metrics(
    _cache: &mut RuntimeMetricsCacheState,
    _logical_cpu_count: usize,
) -> Result<(Option<f32>, Option<u64>), ErrorResponse> {
    Ok((None, None))
}
