use crate::app_state::{AppCpuSampleState, RuntimeMetricsCacheState};
use crate::commands::dto::ErrorResponse;
use chrono::Utc;
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetCurrentProcess, GetProcessTimes},
    },
};

fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

pub(super) fn get_process_metrics(
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
            return Err(ErrorResponse::new(
                "METRICS_PROCESS_TIMES_ERROR",
                "GetProcessTimes failed",
            ));
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
                let normalized =
                    (process_delta as f64) / (elapsed_100ns as f64 * logical_cpu_count as f64) * 100.0;
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
            return Err(ErrorResponse::new(
                "METRICS_PROCESS_MEMORY_ERROR",
                "GetProcessMemoryInfo failed",
            ));
        }

        Ok((process_cpu_percent, Some(counters.WorkingSetSize as u64)))
    }
}
