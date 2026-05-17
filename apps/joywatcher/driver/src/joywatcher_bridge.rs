use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{anyhow, Context, Result};

use crate::joywatcher_artifacts::JoyWatcherArtifacts;

const BRIDGE_EXE_NAME: &str = "joywatcher-bridge-x86.exe";

pub struct JoyWatcherBridgeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    connected: bool,
    mode: BridgeMode,
    exe_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMode {
    Mock,
    Dll,
}

impl BridgeMode {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Dll => "dll",
        }
    }
}

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
            mode,
            exe_path,
        })
    }

    pub fn exe_path(&self) -> &Path {
        &self.exe_path
    }

    pub fn mode(&self) -> BridgeMode {
        self.mode
    }

    pub fn ping(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(anyhow!("unexpected bridge ping response: {}", response));
        }
        Ok(response)
    }

    pub fn connect(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"connect"}"#)?;
        if !response.contains(r#""type":"connected""#) {
            return Err(anyhow!("unexpected bridge connect response: {}", response));
        }
        self.connected = true;
        Ok(response)
    }

    pub fn disconnect(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(anyhow!("unexpected bridge disconnect response: {}", response));
        }
        self.connected = false;
        Ok(response)
    }

    fn send_request(&mut self, request: &str) -> Result<String> {
        writeln!(self.stdin, "{}", request).context("failed to write bridge request")?;
        self.stdin.flush().context("failed to flush bridge stdin")?;

        loop {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .context("failed to read bridge response")?;

            if bytes_read == 0 {
                return Err(anyhow!("bridge returned EOF before responding"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if !trimmed.starts_with('{') {
                continue;
            }

            return Ok(trimmed.to_string());
        }
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

fn bridge_exe_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(explicit_path) = std::env::var("JOYWATCHER_BRIDGE_EXE") {
        push_unique(&mut candidates, PathBuf::from(explicit_path));
    }

    if let Ok(bridge_dir) = std::env::var("JOYWATCHER_BRIDGE_DIR") {
        push_unique(&mut candidates, PathBuf::from(bridge_dir).join(BRIDGE_EXE_NAME));
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(
            &mut candidates,
            repo_root.join("driver-ui").join("joywatcher").join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("debug")
                .join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("release")
                .join(BRIDGE_EXE_NAME),
        );
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join(BRIDGE_EXE_NAME));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join(BRIDGE_EXE_NAME));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
            }
        }
    }

    candidates
}

fn find_repo_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_ancestor_with(&current_dir, |dir| dir.join("Cargo.toml").exists())
}

fn find_ancestor_with(start: &Path, predicate: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut cursor = Some(start);
    while let Some(path) = cursor {
        if predicate(path) {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }
    None
}

fn push_unique(vec: &mut Vec<PathBuf>, path: PathBuf) {
    if !vec.contains(&path) {
        vec.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_candidates_prioritize_driver_ui_and_x86_target_paths() {
        let candidates = bridge_exe_candidates();

        let driver_ui_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"driver-ui\joywatcher\joywatcher-bridge-x86.exe")))
            .expect("driver-ui candidate should exist");
        let x86_debug_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"target\i686-pc-windows-msvc\debug\joywatcher-bridge-x86.exe")))
            .expect("x86 target candidate should exist");
        let x64_debug_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"target\debug\joywatcher-bridge-x86.exe")));

        assert!(driver_ui_index < x86_debug_index || driver_ui_index == x86_debug_index.saturating_sub(1));
        if let Some(x64_debug_index) = x64_debug_index {
            assert!(x86_debug_index < x64_debug_index);
        }
    }
}
