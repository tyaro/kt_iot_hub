#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use tracing::{info, warn};

    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    const MAX_PATH_W: usize = 260;

    #[repr(C)]
    struct ProcessEntry32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; MAX_PATH_W],
    }

    #[derive(Debug, Clone)]
    struct ProcessInfo {
        name: String,
        pid: u32,
        ppid: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
        fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    fn utf16_to_string(buf: &[u16]) -> String {
        let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn collect_target_processes() -> Result<Vec<ProcessInfo>, String> {
        let mut result = Vec::new();
        let targets = [
            "joywatcher-bridge-x86.exe",
            "driver-joywatcher.exe",
            "joywnet2.exe",
            "joywsrv2.exe",
            "jwlauncher.exe",
        ];

        // SAFETY: Win32 API の契約に従い、初期化済み構造体と有効ハンドルのみを扱う。
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
                return Err("CreateToolhelp32Snapshot failed".to_string());
            }

            let mut entry = ProcessEntry32W {
                dw_size: std::mem::size_of::<ProcessEntry32W>() as u32,
                cnt_usage: 0,
                th32_process_id: 0,
                th32_default_heap_id: 0,
                th32_module_id: 0,
                cnt_threads: 0,
                th32_parent_process_id: 0,
                pc_pri_class_base: 0,
                dw_flags: 0,
                sz_exe_file: [0; MAX_PATH_W],
            };

            if Process32FirstW(snapshot, &mut entry as *mut ProcessEntry32W) != 0 {
                loop {
                    let name = utf16_to_string(&entry.sz_exe_file).to_ascii_lowercase();
                    if targets.contains(&name.as_str()) {
                        result.push(ProcessInfo {
                            name,
                            pid: entry.th32_process_id,
                            ppid: entry.th32_parent_process_id,
                        });
                    }

                    if Process32NextW(snapshot, &mut entry as *mut ProcessEntry32W) == 0 {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        result.sort_by_key(|p| p.pid);
        Ok(result)
    }

    pub fn log_jw_process_snapshot(stage: &str) {
        match collect_target_processes() {
            Ok(processes) => {
                if processes.is_empty() {
                    info!(stage, "JW process snapshot: no target processes found");
                    return;
                }

                let rows = processes
                    .iter()
                    .map(|p| format!("{}(pid={},ppid={})", p.name, p.pid, p.ppid))
                    .collect::<Vec<_>>()
                    .join(", ");

                info!(stage, processes = %rows, "JW process snapshot");
            }
            Err(error) => {
                warn!(stage, %error, "JW process snapshot failed");
            }
        }
    }
}

#[cfg(windows)]
pub use imp::log_jw_process_snapshot;

#[cfg(not(windows))]
pub fn log_jw_process_snapshot(_stage: &str) {}
