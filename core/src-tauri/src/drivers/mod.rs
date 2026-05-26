// ドライバプロセスマネージャー
// 各ドライバは独立したプロセスとして起動・管理される
// 本体はプロセスのライフサイクルのみを担当し、通信処理はドライバプロセス側が実装する

pub mod manifest;
pub mod manifest_discovery;
mod path_resolver;

use crate::app_state::RuntimeStatusState;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::process::{Child, Command as TokioCommand};
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, warn};

type Shared<T> = Arc<tokio::sync::RwLock<T>>;

const DRIVER_RESTART_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DRIVER_RESTART_WINDOW: StdDuration = StdDuration::from_secs(60);
const DRIVER_RESTART_BACKOFF_BASE_MS: u64 = 500;
const DRIVER_RESTART_BACKOFF_MAX_MS: u64 = 30_000;

/// ドライバ種別からプロセス実行ファイル名を決定する
/// 例: "postgres" → "driver-postgres.exe" (Windows)
fn executable_name(driver_type: &str) -> String {
    format!("driver-{}{}", driver_type, std::env::consts::EXE_SUFFIX)
}

struct ManagedDriverProcess {
    child: Option<Child>,
    auto_restart: bool,
    max_restart_per_minute: Option<u32>,
}

/// ドライバプロセスマネージャー
/// 各ドライバを子プロセスとして起動・停止・監視する
pub struct DriverProcessManager {
    processes: HashMap<String, ManagedDriverProcess>,
    restart_history: HashMap<String, VecDeque<Instant>>,
    /// 本体の gRPC アドレス（ドライバプロセスへの引数に渡す）
    grpc_addr: String,
}

impl DriverProcessManager {
    pub fn new(grpc_addr: impl Into<String>) -> Self {
        Self {
            processes: HashMap::new(),
            restart_history: HashMap::new(),
            grpc_addr: grpc_addr.into(),
        }
    }

    /// 指定ドライバのプロセスを起動する
    /// executable は本体と同じディレクトリから解決する
    /// 開発時は DRIVER_BIN_DIR 環境変数で上書き可能
    pub async fn start_driver(
        &mut self,
        manager_handle: Shared<DriverProcessManager>,
        runtime_status: Shared<RuntimeStatusState>,
        driver_id: &str,
        driver_type: &str,
        driver_ui_base_dir: Option<&str>,
        auto_restart: bool,
        max_restart_per_minute: Option<u32>,
    ) -> Result<()> {
        if self.processes.contains_key(driver_id) {
            warn!("Driver process already running: {}", driver_id);
            return Ok(());
        }

        let exe_name = executable_name(driver_type);
        let exe_path = self
            .resolve_exe_path(driver_type, &exe_name, driver_ui_base_dir)?
            .ok_or_else(|| {
                anyhow!(
                    "Driver executable not found for type '{}'. Looked for '{}' near the configured driver-ui base path, DRIVER_BIN_DIR, and the app directory. Build/install the runtime driver beside the registration UI (for dev, use `npm run driver-runtime:dev`).",
                    driver_type,
                    exe_name
                )
            })?;

        info!(
            "Starting driver process: id={} exe={}",
            driver_id,
            exe_path.display()
        );

        let child = Self::spawn_driver_process(&exe_path, driver_id, driver_type, &self.grpc_addr)?;

        self.processes.insert(
            driver_id.to_string(),
            ManagedDriverProcess {
                child: Some(child),
                auto_restart,
                max_restart_per_minute,
            },
        );
        self.restart_history
            .entry(driver_id.to_string())
            .or_default();

        let monitor_driver_id = driver_id.to_string();
        let monitor_driver_type = driver_type.to_string();
        let monitor_exe_path = exe_path.clone();
        let grpc_addr = self.grpc_addr.clone();
        tauri::async_runtime::spawn(async move {
            supervise_driver_process(
                manager_handle,
                runtime_status,
                monitor_driver_id,
                monitor_driver_type,
                monitor_exe_path,
                grpc_addr,
            )
            .await;
        });

        info!("Driver process started: {}", driver_id);
        Ok(())
    }

    /// 指定ドライバのプロセスを停止する
    pub async fn stop_driver(&mut self, driver_id: &str) -> Result<()> {
        if let Some(mut process) = self.processes.remove(driver_id) {
            info!("Stopping driver process: {}", driver_id);
            self.restart_history.remove(driver_id);

            if let Some(mut child) = process.child.take() {
                if let Err(e) = child.start_kill() {
                    warn!(
                        "Failed to send kill signal to driver process {}: {}",
                        driver_id, e
                    );
                }

                match timeout(Duration::from_secs(3), child.wait()).await {
                    Ok(Ok(status)) => {
                        info!("Driver process stopped: {} (status={})", driver_id, status);
                    }
                    Ok(Err(e)) => {
                        warn!("Failed while waiting driver process {}: {}", driver_id, e);
                        force_kill_driver_process(&mut child, driver_id).await;
                    }
                    Err(_) => {
                        warn!(
                            "Timed out waiting driver process {} to stop; forcing kill",
                            driver_id
                        );
                        force_kill_driver_process(&mut child, driver_id).await;
                    }
                }
            }
        } else {
            warn!("Driver process not found: {}", driver_id);
        }
        Ok(())
    }

    /// 全ドライバプロセスを停止する
    pub async fn stop_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.processes.keys().cloned().collect();
        for id in ids {
            if let Err(e) = self.stop_driver(&id).await {
                error!("Failed to stop driver {}: {}", id, e);
            }
        }
        Ok(())
    }

    /// 起動中のドライバID一覧を返す
    pub fn running_driver_ids(&self) -> Vec<&str> {
        self.processes
            .iter()
            .filter(|(_, process)| process.child.is_some())
            .map(|(driver_id, _)| driver_id.as_str())
            .collect()
    }

    /// 起動中ドライバの (driver_id, pid) 一覧を返す
    pub fn running_driver_processes(&self) -> Vec<(String, u32)> {
        self.processes
            .iter()
            .filter_map(|(driver_id, process)| {
                process
                    .child
                    .as_ref()
                    .and_then(|child| child.id().map(|pid| (driver_id.clone(), pid)))
            })
            .collect()
    }

    /// ドライバプロセスが起動中か確認する
    pub fn is_running(&self, driver_id: &str) -> bool {
        self.processes
            .get(driver_id)
            .and_then(|process| process.child.as_ref())
            .is_some()
    }

    fn spawn_driver_process(
        exe_path: &Path,
        driver_id: &str,
        driver_type: &str,
        grpc_addr: &str,
    ) -> Result<Child> {
        let mut cmd = tokio::process::Command::new(exe_path);
        cmd.arg("--driver-id")
            .arg(driver_id)
            .arg("--driver-kind")
            .arg(driver_type)
            .arg("--grpc-addr")
            .arg(grpc_addr)
            .arg("--parent-pid")
            .arg(std::process::id().to_string())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.spawn().map_err(|e| {
            anyhow!(
                "Failed to spawn driver process '{}': {}",
                exe_path.display(),
                e
            )
        })
    }

    fn record_restart_attempt(&mut self, driver_id: &str, now: Instant) -> usize {
        let history = self
            .restart_history
            .entry(driver_id.to_string())
            .or_default();
        while let Some(front) = history.front() {
            if now.duration_since(*front) > DRIVER_RESTART_WINDOW {
                history.pop_front();
            } else {
                break;
            }
        }
        history.push_back(now);
        history.len()
    }

    fn clear_restart_history(&mut self, driver_id: &str) {
        self.restart_history.remove(driver_id);
    }

    fn restart_backoff(attempts: usize) -> Duration {
        let exponent = attempts.saturating_sub(1).min(6) as u32;
        let delay_ms =
            (DRIVER_RESTART_BACKOFF_BASE_MS << exponent).min(DRIVER_RESTART_BACKOFF_MAX_MS);
        Duration::from_millis(delay_ms)
    }

    /// executable のパスを解決する
    /// 優先順位: manifest > driver-uiベースパス同居配置 > DRIVER_BIN_DIR > 本体と同じディレクトリ > 実行名そのまま
    fn resolve_exe_path(
        &self,
        driver_type: &str,
        exe_name: &str,
        driver_ui_base_dir: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        for root in path_resolver::app_root_candidates(driver_ui_base_dir) {
            if let Some(manifest_runtime_path) =
                path_resolver::resolve_manifest_runtime_path_for_root(driver_type, &root)?
            {
                return Ok(Some(manifest_runtime_path));
            }

            for candidate in
                path_resolver::colocated_runtime_candidates_for_root(driver_type, exe_name, &root)
            {
                if candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
        }

        if let Ok(bin_dir) = std::env::var("DRIVER_BIN_DIR") {
            let candidate = PathBuf::from(bin_dir).join(exe_name);
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join(exe_name);
                if candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
        }

        Ok(Some(PathBuf::from(exe_name)))
    }
}

async fn force_kill_driver_process(child: &mut Child, driver_id: &str) {
    let pid = child.id();

    if let Err(e) = child.kill().await {
        warn!("Forced kill failed for driver process {}: {}", driver_id, e);
    }

    match timeout(Duration::from_secs(2), child.wait()).await {
        Ok(Ok(status)) => {
            info!(
                "Driver process force-stopped: {} (status={})",
                driver_id, status
            );
            return;
        }
        Ok(Err(e)) => {
            warn!(
                "Failed while waiting force-killed driver process {}: {}",
                driver_id, e
            );
        }
        Err(_) => {
            warn!(
                "Timed out waiting force-killed driver process {} to exit",
                driver_id
            );
        }
    }

    #[cfg(windows)]
    {
        if let Some(pid) = pid {
            match TokioCommand::new("taskkill")
                .args([
                    "/PID",
                    &pid.to_string(),
                    "/T",
                    "/F",
                ])
                .output()
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        info!(
                            "taskkill fallback succeeded for driver process {} (pid={})",
                            driver_id, pid
                        );
                    } else {
                        warn!(
                            "taskkill fallback failed for driver process {} (pid={} status={}): {}",
                            driver_id,
                            pid,
                            output.status,
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "taskkill fallback command failed for driver process {} (pid={}): {}",
                        driver_id, pid, e
                    );
                }
            }

            match timeout(Duration::from_secs(2), child.wait()).await {
                Ok(Ok(status)) => {
                    info!(
                        "Driver process exited after taskkill fallback: {} (status={})",
                        driver_id, status
                    );
                }
                Ok(Err(e)) => {
                    warn!(
                        "Failed while waiting driver process {} after taskkill fallback: {}",
                        driver_id, e
                    );
                }
                Err(_) => {
                    warn!(
                        "Driver process {} still running after taskkill fallback timeout",
                        driver_id
                    );
                }
            }
        }
    }
}

async fn supervise_driver_process(
    manager_handle: Shared<DriverProcessManager>,
    runtime_status: Shared<RuntimeStatusState>,
    driver_id: String,
    driver_type: String,
    exe_path: PathBuf,
    grpc_addr: String,
) {
    loop {
        let exit_status = {
            let mut manager = manager_handle.write().await;
            let Some(process) = manager.processes.get_mut(&driver_id) else {
                return;
            };

            let Some(child) = process.child.as_mut() else {
                return;
            };

            match child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => None,
                Err(e) => {
                    error!("Failed to inspect driver process {}: {}", driver_id, e);
                    let mut runtime = runtime_status.write().await;
                    runtime.last_error = Some(format!(
                        "Failed to inspect driver process '{}' while monitoring",
                        driver_id
                    ));
                    manager.processes.remove(&driver_id);
                    manager.clear_restart_history(&driver_id);
                    drop(manager);
                    refresh_driver_running_status(&manager_handle, &runtime_status).await;
                    return;
                }
            }
        };

        let Some(status) = exit_status else {
            sleep(DRIVER_RESTART_POLL_INTERVAL).await;
            continue;
        };

        info!("Driver process exited: {} (status={})", driver_id, status);

        let (auto_restart, max_restart_per_minute) = {
            let mut manager = manager_handle.write().await;
            let Some(process) = manager.processes.get_mut(&driver_id) else {
                return;
            };

            process.child = None;
            (process.auto_restart, process.max_restart_per_minute)
        };

        {
            let mut runtime = runtime_status.write().await;
            runtime.last_error = Some(format!(
                "Driver process '{}' exited with status {}",
                driver_id, status
            ));
        }

        if !auto_restart {
            let mut manager = manager_handle.write().await;
            manager.processes.remove(&driver_id);
            manager.clear_restart_history(&driver_id);
            drop(manager);
            refresh_driver_running_status(&manager_handle, &runtime_status).await;
            return;
        }

        let restart_attempts = {
            let mut manager = manager_handle.write().await;
            manager.record_restart_attempt(&driver_id, Instant::now())
        };

        if let Some(limit) = max_restart_per_minute {
            if restart_attempts > limit as usize {
                let message = format!(
                    "Driver process '{}' restart limit exceeded ({} per minute)",
                    driver_id, limit
                );
                warn!("{}", message);
                let mut runtime = runtime_status.write().await;
                runtime.last_error = Some(message);
                let mut manager = manager_handle.write().await;
                manager.processes.remove(&driver_id);
                manager.clear_restart_history(&driver_id);
                drop(manager);
                refresh_driver_running_status(&manager_handle, &runtime_status).await;
                return;
            }
        }

        let backoff = DriverProcessManager::restart_backoff(restart_attempts);
        info!(
            "Restarting driver process {} after {:?} (attempt #{})",
            driver_id, backoff, restart_attempts
        );
        sleep(backoff).await;

        let should_restart = {
            let manager = manager_handle.read().await;
            manager.processes.contains_key(&driver_id)
        };

        if !should_restart {
            refresh_driver_running_status(&manager_handle, &runtime_status).await;
            return;
        }

        let new_child = match DriverProcessManager::spawn_driver_process(
            &exe_path,
            &driver_id,
            &driver_type,
            &grpc_addr,
        ) {
            Ok(child) => child,
            Err(e) => {
                let message = format!(
                    "Failed to restart driver process '{}' (exe='{}'): {}",
                    driver_id,
                    exe_path.display(),
                    e
                );
                error!("{}", message);
                let mut runtime = runtime_status.write().await;
                runtime.last_error = Some(message);
                let mut manager = manager_handle.write().await;
                manager.processes.remove(&driver_id);
                manager.clear_restart_history(&driver_id);
                drop(manager);
                refresh_driver_running_status(&manager_handle, &runtime_status).await;
                return;
            }
        };

        {
            let mut manager = manager_handle.write().await;
            let Some(process) = manager.processes.get_mut(&driver_id) else {
                let mut child = new_child;
                if let Err(e) = child.start_kill() {
                    warn!(
                        "Failed to stop restarted driver process {}: {}",
                        driver_id, e
                    );
                }
                let _ = timeout(Duration::from_secs(3), child.wait()).await;
                return;
            };

            process.child = Some(new_child);
        }

        info!("Driver process restarted: {}", driver_id);
        refresh_driver_running_status(&manager_handle, &runtime_status).await;
    }
}

async fn refresh_driver_running_status(
    manager_handle: &Shared<DriverProcessManager>,
    runtime_status: &Shared<RuntimeStatusState>,
) {
    let manager = manager_handle.read().await;
    let mut runtime = runtime_status.write().await;
    runtime.drivers_running = !manager.running_driver_ids().is_empty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kt_iot_hub_{}_{}",
            name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn resolve_exe_path_prefers_manifest_runtime_when_valid() {
        let root = unique_temp_dir("manifest_runtime_valid");
        let manifest_dir = root.join("ops").join("driver-ui").join("postgres");
        let registration_ui_path = manifest_dir.join("registration-ui.exe");
        let runtime_path = manifest_dir.join("driver-postgres.exe");
        let manifest_path = manifest_dir.join("driver-manifest.json");

        fs::create_dir_all(&manifest_dir).unwrap();
        fs::write(&registration_ui_path, b"test").unwrap();
        fs::write(&runtime_path, b"test").unwrap();
        fs::write(
            &manifest_path,
            r#"{
  "manifestVersion": 1,
  "driverType": "postgres",
  "displayName": "PostgreSQL 接続",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-postgres.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  }
}"#,
        )
        .unwrap();

        let manager = DriverProcessManager::new("127.0.0.1:50051");
        let resolved = manager.resolve_exe_path(
            "postgres",
            "driver-postgres.exe",
            Some(root.to_string_lossy().as_ref()),
        );

        assert_eq!(resolved.unwrap(), Some(runtime_path.clone()));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_exe_path_errors_when_manifest_is_invalid() {
        let root = unique_temp_dir("manifest_runtime_fallback");
        let manifest_dir = root.join("ops").join("driver-ui").join("postgres");
        let manifest_path = manifest_dir.join("driver-manifest.json");
        let legacy_runtime_path = root.join("postgres").join("driver-postgres.exe");

        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(legacy_runtime_path.parent().unwrap()).unwrap();
        fs::write(&legacy_runtime_path, b"test").unwrap();
        fs::write(
            &manifest_path,
            r#"{
  "manifestVersion": 1,
  "driverType": "joywatcher",
  "displayName": "PostgreSQL 接続",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-postgres.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  }
}"#,
        )
        .unwrap();

        let manager = DriverProcessManager::new("127.0.0.1:50051");
        let resolved = manager.resolve_exe_path(
            "postgres",
            "driver-postgres.exe",
            Some(root.to_string_lossy().as_ref()),
        );

        assert!(resolved.is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restart_backoff_grows_with_attempts() {
        assert!(
            DriverProcessManager::restart_backoff(1) < DriverProcessManager::restart_backoff(2)
        );
        assert!(
            DriverProcessManager::restart_backoff(2) < DriverProcessManager::restart_backoff(3)
        );
    }
}
