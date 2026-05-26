use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tracing::{debug, info};

use crate::joywatcher_bridge::{BridgeConnectionSettings, JoyWatcherBridgeProcess};

const DEFAULT_PRIMARY_SERVER_PROCESS: &str = "JoyWSrv2.exe";
const DEFAULT_JOYWNET2_PROCESS: &str = "JoyWNet2.exe";
const DEFAULT_JOYWNET2_WAIT_TIMEOUT_MS: u64 = 5_000;
const JOYWNET2_WAIT_POLL_MS: u64 = 200;

#[derive(Debug, Clone)]
pub struct JoyWatcherServerGuard {
    pub primary_server_process_name: String,
    pub wait_for_joywnet2: bool,
    pub joywnet2_process_name: String,
    pub joywnet2_wait_timeout: Duration,
}

impl JoyWatcherServerGuard {
    pub fn from_connection_settings(settings: Option<&HashMap<String, String>>) -> Self {
        let primary_server_process_name = get_string_setting(
            settings,
            &["primary_server_process_name", "primaryServerProcessName"],
            DEFAULT_PRIMARY_SERVER_PROCESS,
        );
        let wait_for_joywnet2 = get_bool_setting(
            settings,
            &["wait_for_joywnet2", "waitForJoyWNet2"],
            false,
        );
        let joywnet2_process_name = get_string_setting(
            settings,
            &["joywnet2_process_name", "joyWNet2ProcessName"],
            DEFAULT_JOYWNET2_PROCESS,
        );
        let joywnet2_wait_timeout_ms = get_u64_setting(
            settings,
            &["joywnet2_wait_timeout_ms", "joyWNet2WaitTimeoutMs"],
            DEFAULT_JOYWNET2_WAIT_TIMEOUT_MS,
        );

        Self {
            primary_server_process_name,
            wait_for_joywnet2,
            joywnet2_process_name,
            joywnet2_wait_timeout: Duration::from_millis(joywnet2_wait_timeout_ms),
        }
    }

    pub fn ensure_primary_server_ready(&self) -> Result<()> {
        let running = is_process_running(&self.primary_server_process_name)?;
        if running {
            debug!(
                process = %self.primary_server_process_name,
                "JoyWatcher primary server process is running"
            );
            return Ok(());
        }

        Err(anyhow!(
            "JoyWatcher primary server process is not running: {}",
            self.primary_server_process_name
        ))
    }

    pub fn ensure_bridge_connection_ready(
        &self,
        bridge: &mut JoyWatcherBridgeProcess,
        connection: &BridgeConnectionSettings,
    ) -> Result<()> {
        bridge.ensure_connection(connection)?;

        if !self.wait_for_joywnet2 {
            return Ok(());
        }

        self.wait_for_joywnet2_process()
    }

    fn wait_for_joywnet2_process(&self) -> Result<()> {
        let start = Instant::now();
        loop {
            if is_process_running(&self.joywnet2_process_name)? {
                info!(
                    process = %self.joywnet2_process_name,
                    elapsed_ms = start.elapsed().as_millis() as u64,
                    "JoyWatcher JoyWNet2 process became ready"
                );
                return Ok(());
            }

            if start.elapsed() >= self.joywnet2_wait_timeout {
                return Err(anyhow!(
                    "JoyWatcher JoyWNet2 process wait timed out: process={} timeout_ms={}",
                    self.joywnet2_process_name,
                    self.joywnet2_wait_timeout.as_millis()
                ));
            }

            std::thread::sleep(Duration::from_millis(JOYWNET2_WAIT_POLL_MS));
        }
    }
}

fn get_string_setting(
    settings: Option<&HashMap<String, String>>,
    keys: &[&str],
    default_value: &str,
) -> String {
    settings
        .and_then(|map| {
            keys.iter().find_map(|key| {
                map.get(*key)
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
        })
        .unwrap_or_else(|| default_value.to_string())
}

fn get_bool_setting(settings: Option<&HashMap<String, String>>, keys: &[&str], default_value: bool) -> bool {
    settings
        .and_then(|map| {
            keys.iter().find_map(|key| {
                map.get(*key)
                    .map(|value| value.trim().to_ascii_lowercase())
                    .and_then(|value| match value.as_str() {
                        "true" | "1" | "yes" | "on" => Some(true),
                        "false" | "0" | "no" | "off" => Some(false),
                        _ => None,
                    })
            })
        })
        .unwrap_or(default_value)
}

fn get_u64_setting(settings: Option<&HashMap<String, String>>, keys: &[&str], default_value: u64) -> u64 {
    settings
        .and_then(|map| {
            keys.iter()
                .find_map(|key| map.get(*key).and_then(|value| value.trim().parse::<u64>().ok()))
        })
        .unwrap_or(default_value)
}

#[cfg(windows)]
fn is_process_running(process_name: &str) -> Result<bool> {
    use std::ffi::c_void;

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

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
        fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    let target = process_name.trim().to_ascii_lowercase();

    // SAFETY: Toolhelp API の規約に従い、初期化済み構造体と有効ハンドルのみを扱う。
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
            return Err(anyhow!("CreateToolhelp32Snapshot failed"));
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

        let mut found = false;
        if Process32FirstW(snapshot, &mut entry as *mut ProcessEntry32W) != 0 {
            loop {
                let exe_name_end = entry
                    .sz_exe_file
                    .iter()
                    .position(|ch| *ch == 0)
                    .unwrap_or(entry.sz_exe_file.len());
                let exe_name = String::from_utf16_lossy(&entry.sz_exe_file[..exe_name_end])
                    .to_ascii_lowercase();

                if exe_name == target {
                    found = true;
                    break;
                }

                if Process32NextW(snapshot, &mut entry as *mut ProcessEntry32W) == 0 {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        Ok(found)
    }
}

#[cfg(not(windows))]
fn is_process_running(_process_name: &str) -> Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_connection_settings_uses_defaults_when_missing() {
        let guard = JoyWatcherServerGuard::from_connection_settings(None);

        assert_eq!(guard.primary_server_process_name, "JoyWSrv2.exe");
        assert!(!guard.wait_for_joywnet2);
        assert_eq!(guard.joywnet2_process_name, "JoyWNet2.exe");
        assert_eq!(guard.joywnet2_wait_timeout, Duration::from_millis(5_000));
    }

    #[test]
    fn from_connection_settings_accepts_snake_case_keys() {
        let mut settings = HashMap::new();
        settings.insert(
            "primary_server_process_name".to_string(),
            "  JoyWSrv2Custom.exe  ".to_string(),
        );
        settings.insert("wait_for_joywnet2".to_string(), "true".to_string());
        settings.insert(
            "joywnet2_process_name".to_string(),
            "JoyWNet2Custom.exe".to_string(),
        );
        settings.insert(
            "joywnet2_wait_timeout_ms".to_string(),
            "12345".to_string(),
        );

        let guard = JoyWatcherServerGuard::from_connection_settings(Some(&settings));

        assert_eq!(guard.primary_server_process_name, "JoyWSrv2Custom.exe");
        assert!(guard.wait_for_joywnet2);
        assert_eq!(guard.joywnet2_process_name, "JoyWNet2Custom.exe");
        assert_eq!(guard.joywnet2_wait_timeout, Duration::from_millis(12_345));
    }

    #[test]
    fn from_connection_settings_accepts_camel_case_keys() {
        let mut settings = HashMap::new();
        settings.insert(
            "primaryServerProcessName".to_string(),
            "JoyWSrv2Camel.exe".to_string(),
        );
        settings.insert("waitForJoyWNet2".to_string(), "yes".to_string());
        settings.insert(
            "joyWNet2ProcessName".to_string(),
            "JoyWNet2Camel.exe".to_string(),
        );
        settings.insert(
            "joyWNet2WaitTimeoutMs".to_string(),
            "777".to_string(),
        );

        let guard = JoyWatcherServerGuard::from_connection_settings(Some(&settings));

        assert_eq!(guard.primary_server_process_name, "JoyWSrv2Camel.exe");
        assert!(guard.wait_for_joywnet2);
        assert_eq!(guard.joywnet2_process_name, "JoyWNet2Camel.exe");
        assert_eq!(guard.joywnet2_wait_timeout, Duration::from_millis(777));
    }

    #[test]
    fn from_connection_settings_falls_back_on_invalid_values() {
        let mut settings = HashMap::new();
        settings.insert("wait_for_joywnet2".to_string(), "maybe".to_string());
        settings.insert("joywnet2_wait_timeout_ms".to_string(), "oops".to_string());
        settings.insert("primary_server_process_name".to_string(), "   ".to_string());

        let guard = JoyWatcherServerGuard::from_connection_settings(Some(&settings));

        assert_eq!(guard.primary_server_process_name, "JoyWSrv2.exe");
        assert!(!guard.wait_for_joywnet2);
        assert_eq!(guard.joywnet2_wait_timeout, Duration::from_millis(5_000));
    }
}
