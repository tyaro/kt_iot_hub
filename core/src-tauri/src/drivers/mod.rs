// ドライバプロセスマネージャー
// 各ドライバは独立したプロセスとして起動・管理される
// 本体はプロセスのライフサイクルのみを担当し、通信処理はドライバプロセス側が実装する

pub mod manifest;
pub mod manifest_discovery;
mod path_resolver;

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

        let mut cmd = tokio::process::Command::new(&exe_path);
        cmd.arg("--driver-id")
            .arg(driver_id)
            .arg("--driver-kind")
            .arg(driver_type)
            .arg("--grpc-addr")
            .arg(&self.grpc_addr)
            .kill_on_drop(true);
        // Windows: 子プロセスがコンソールサブシステムであっても
        // コンソールウィンドウを生成しないようフラグを設定する。
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let child = cmd.spawn()
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
    /// 優先順位: manifest > driver-uiベースパス同居配置 > DRIVER_BIN_DIR > 本体と同じディレクトリ > 実行名そのまま
    fn resolve_exe_path(
        &self,
        driver_type: &str,
        exe_name: &str,
        driver_ui_base_dir: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        for root in path_resolver::app_root_candidates(driver_ui_base_dir) {
            if let Some(manifest_runtime_path) =
                path_resolver::resolve_manifest_runtime_path_for_root(driver_type, &root)
                ?
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
}
