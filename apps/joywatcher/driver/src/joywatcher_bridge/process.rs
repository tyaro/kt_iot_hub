use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

use super::{BridgeMode, JoyWatcherBridgeProcess};
use crate::joywatcher_artifacts::JoyWatcherArtifacts;
use crate::path_utils::{bridge_exe_candidates, BRIDGE_EXE_NAME};

impl JoyWatcherBridgeProcess {
    pub fn start(artifacts: &JoyWatcherArtifacts) -> Result<Self> {
        let exe_path = resolve_bridge_exe_path()?.ok_or_else(|| {
            anyhow!(
                "JoyWatcher bridge executable not found. Looked for '{}' in known bridge locations.",
                BRIDGE_EXE_NAME
            )
        })?;

        let mode = if artifacts.dll_path.is_some() {
            BridgeMode::Dll
        } else {
            BridgeMode::Mock
        };

        let mut command = Command::new(&exe_path);
        command
            .arg("--mode")
            .arg(mode.as_arg())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        if let (BridgeMode::Dll, Some(dll_path)) = (mode, artifacts.dll_path.as_ref()) {
            command.arg("--dll-path").arg(dll_path);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn JoyWatcher bridge: {}", exe_path.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture bridge stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture bridge stdout"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: false,
            connection: None,
            mode,
            exe_path,
        })
    }
}

impl Drop for JoyWatcherBridgeProcess {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.disconnect();
        }

        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resolve_bridge_exe_path() -> Result<Option<PathBuf>> {
    for candidate in bridge_exe_candidates() {
        if candidate.exists() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn bridge_candidates_prioritize_driver_ui_and_x86_target_paths() {
        let candidates = bridge_exe_candidates();

        let driver_ui_index = candidates
            .iter()
            .position(|path| {
                path.ends_with(Path::new(r"driver-ui\joywatcher\joywatcher-bridge-x86.exe"))
            })
            .expect("driver-ui candidate should exist");
        let x86_debug_index = candidates
            .iter()
            .position(|path| {
                path.ends_with(Path::new(
                    r"target\i686-pc-windows-msvc\debug\joywatcher-bridge-x86.exe",
                ))
            })
            .expect("x86 target candidate should exist");
        let x64_debug_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"target\debug\joywatcher-bridge-x86.exe")));

        assert!(
            driver_ui_index < x86_debug_index
                || driver_ui_index == x86_debug_index.saturating_sub(1)
        );
        if let Some(x64_debug_index) = x64_debug_index {
            assert!(x86_debug_index < x64_debug_index);
        }
    }
}
