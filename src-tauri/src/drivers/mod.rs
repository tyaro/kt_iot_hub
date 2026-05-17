// ドライバプロセスマネージャー
// 各ドライバは独立したプロセスとして起動・管理される
// 本体はプロセスのライフサイクルのみを担当し、通信処理はドライバプロセス側が実装する

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::process::Child;
use tracing::{error, info, warn};

/// ドライバ種別からプロセス実行ファイル名を決定する
/// 例: "postgres" → "driver-postgres" (.exe は OS が補完)
fn executable_name(driver_type: &str) -> String {
    format!("driver-{}", driver_type)
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
    pub async fn start_driver(&mut self, driver_id: &str, driver_type: &str) -> Result<()> {
        if self.processes.contains_key(driver_id) {
            warn!("Driver process already running: {}", driver_id);
            return Ok(());
        }

        let exe_name = executable_name(driver_type);
        let exe_path = self.resolve_exe_path(&exe_name);

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
            if let Err(e) = child.kill().await {
                warn!("Failed to kill driver process {}: {}", driver_id, e);
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

    /// ドライバプロセスが起動中か確認する
    pub fn is_running(&self, driver_id: &str) -> bool {
        self.processes.contains_key(driver_id)
    }

    /// executable のパスを解決する
    /// 優先順位: DRIVER_BIN_DIR 環境変数 > 本体と同じディレクトリ
    fn resolve_exe_path(&self, exe_name: &str) -> std::path::PathBuf {
        if let Ok(bin_dir) = std::env::var("DRIVER_BIN_DIR") {
            return std::path::PathBuf::from(bin_dir).join(exe_name);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join(exe_name);
            }
        }
        std::path::PathBuf::from(exe_name)
    }
}
