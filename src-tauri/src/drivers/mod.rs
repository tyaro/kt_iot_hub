// ドライバプロセスマネージャー
// 各ドライバは独立したプロセスとして起動・管理される
// 本体はプロセスのライフサイクルのみを担当し、通信処理はドライバプロセス側が実装する

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Child;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

/// ドライバ種別からプロセス実行ファイル名を決定する
/// 例: "postgres" → "driver-postgres.exe" (Windows)
fn executable_name(driver_type: &str) -> String {
    format!("driver-{}{}", driver_type, std::env::consts::EXE_SUFFIX)
}

/// ドライバプロセスマネージャー
/// 各ドライバを子プロセスとして起動・停止・監視する
pub struct DriverProcessManager {
    processes: HashMap<String, Child>,
    /// 本体の gRPC アドレス（ドライバプロセスへの引数に渡す）
    grpc_addr: String,
}

impl DriverProcessManager {
    pub fn new(grpc_addr: impl Into<String>) -> Self {
        Self {
            processes: HashMap::new(),
            grpc_addr: grpc_addr.into(),
        }
    }

    /// 指定ドライバのプロセスを起動する
    /// executable は本体と同じディレクトリから解決する
    /// 開発時は DRIVER_BIN_DIR 環境変数で上書き可能
    pub async fn start_driver(
        &mut self,
        driver_id: &str,
        driver_type: &str,
        driver_ui_base_dir: Option<&str>,
    ) -> Result<()> {
        if self.processes.contains_key(driver_id) {
            warn!("Driver process already running: {}", driver_id);
            return Ok(());
        }

        let exe_name = executable_name(driver_type);
        let exe_path = self
            .resolve_exe_path(driver_type, &exe_name, driver_ui_base_dir)
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

        let child = tokio::process::Command::new(&exe_path)
            .arg("--driver-id")
            .arg(driver_id)
            .arg("--driver-kind")
            .arg(driver_type)
            .arg("--grpc-addr")
            .arg(&self.grpc_addr)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                anyhow!(
                    "Failed to spawn driver process '{}': {}",
                    exe_path.display(),
                    e
                )
            })?;

        self.processes.insert(driver_id.to_string(), child);
        info!("Driver process started: {}", driver_id);
        Ok(())
    }

    /// 指定ドライバのプロセスを停止する
    pub async fn stop_driver(&mut self, driver_id: &str) -> Result<()> {
        if let Some(mut child) = self.processes.remove(driver_id) {
            info!("Stopping driver process: {}", driver_id);
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
                }
                Err(_) => {
                    warn!(
                        "Timed out waiting driver process {} to stop; forcing kill",
                        driver_id
                    );
                    if let Err(e) = child.kill().await {
                        warn!("Forced kill failed for driver process {}: {}", driver_id, e);
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
        self.processes.keys().map(|s| s.as_str()).collect()
    }

    /// 起動中ドライバの (driver_id, pid) 一覧を返す
    pub fn running_driver_processes(&self) -> Vec<(String, u32)> {
        self.processes
            .iter()
            .filter_map(|(driver_id, child)| child.id().map(|pid| (driver_id.clone(), pid)))
            .collect()
    }

    /// ドライバプロセスが起動中か確認する
    pub fn is_running(&self, driver_id: &str) -> bool {
        self.processes.contains_key(driver_id)
    }

    /// executable のパスを解決する
    /// 優先順位: driver-uiベースパス同居配置 > DRIVER_BIN_DIR > 本体と同じディレクトリ > 実行名そのまま
    fn resolve_exe_path(
        &self,
        driver_type: &str,
        exe_name: &str,
        driver_ui_base_dir: Option<&str>,
    ) -> Option<PathBuf> {
        for candidate in colocated_runtime_candidates(driver_type, exe_name, driver_ui_base_dir) {
            if candidate.exists() {
                return Some(candidate);
            }
        }

        if let Ok(bin_dir) = std::env::var("DRIVER_BIN_DIR") {
            let candidate = PathBuf::from(bin_dir).join(exe_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join(exe_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        Some(PathBuf::from(exe_name))
    }
}

fn colocated_runtime_candidates(
    driver_type: &str,
    exe_name: &str,
    driver_ui_base_dir: Option<&str>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for root in app_root_candidates(driver_ui_base_dir) {
        candidates.push(root.join("driver-ui").join(driver_type).join(exe_name));
        candidates.push(root.join(driver_type).join(exe_name));
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }

    unique
}

fn app_root_candidates(driver_ui_base_dir: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();

    if let Some(base_dir) = driver_ui_base_dir {
        let trimmed = base_dir.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let mut cursor = Some(current_dir.as_path());
        while let Some(path) = cursor {
            roots.push(path.to_path_buf());
            cursor = path.parent();
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let mut cursor = Some(exe_dir);
            while let Some(path) = cursor {
                roots.push(path.to_path_buf());
                cursor = path.parent();
            }
        }
    }

    let mut unique = Vec::<PathBuf>::new();
    for root in roots {
        if !unique.contains(&root) {
            unique.push(root);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colocated_candidates_support_repo_root_base() {
        let candidates = colocated_runtime_candidates(
            "postgres",
            "driver-postgres.exe",
            Some(r"D:\develop\kt_iot_hub"),
        );

        assert_eq!(
            candidates[0],
            PathBuf::from(r"D:\develop\kt_iot_hub")
                .join("driver-ui")
                .join("postgres")
                .join("driver-postgres.exe")
        );
    }

    #[test]
    fn colocated_candidates_support_driver_ui_root_base() {
        let candidates = colocated_runtime_candidates(
            "postgres",
            "driver-postgres.exe",
            Some(r"D:\develop\kt_iot_hub\driver-ui"),
        );

        assert_eq!(
            candidates[1],
            PathBuf::from(r"D:\develop\kt_iot_hub\driver-ui")
                .join("postgres")
                .join("driver-postgres.exe")
        );
    }
}
