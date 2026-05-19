use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::path_utils::{BRIDGE_EXE_NAME, DLL_FILE_NAME};

pub(super) struct JoyWatcherUiBridgeClient {
    pub(super) child: Child,
    pub(super) stdin: ChildStdin,
    pub(super) stdout: BufReader<ChildStdout>,
    pub(super) connected: bool,
}

impl JoyWatcherUiBridgeClient {
    pub(super) fn start() -> Result<Self, String> {
        let exe_path = resolve_bridge_exe_path()
            .ok_or_else(|| format!("JoyWatcher bridge executable not found: {BRIDGE_EXE_NAME}"))?;
        let dll_path = resolve_dll_path()
            .ok_or_else(|| format!("JoyWaApi.dll not found in known search roots: {DLL_FILE_NAME}"))?;

        let mut command = Command::new(&exe_path);
        command
            .arg("--mode")
            .arg("dll")
            .arg("--dll-path")
            .arg(&dll_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command
            .spawn()
            .map_err(|e| format!("failed to spawn JoyWatcher bridge '{}': {e}", exe_path.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdout".to_string())?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: false,
        })
    }

    pub(super) fn disconnect_if_connected(&mut self) {
        if self.connected {
            let _ = self.disconnect();
        }
    }
}

impl Drop for JoyWatcherUiBridgeClient {
    fn drop(&mut self) {
        self.disconnect_if_connected();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resolve_bridge_exe_path() -> Option<PathBuf> {
    crate::path_utils::bridge_exe_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn resolve_dll_path() -> Option<PathBuf> {
    crate::path_utils::dll_file_candidates()
        .into_iter()
        .find(|path| path.exists())
}
