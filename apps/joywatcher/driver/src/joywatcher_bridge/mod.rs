use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout};

mod commands;
mod process;
mod protocol;

pub struct JoyWatcherBridgeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    connected: bool,
    connection: Option<BridgeConnectionSettings>,
    mode: BridgeMode,
    exe_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BridgeConnectionSettings {
    pub endpoint: Option<String>,
    pub user_id: i32,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeReadValue {
    pub native_tag_id: i32,
    pub quality: String,
    pub value: BridgeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeValue {
    Bool(bool),
    Number(f64),
    String(String),
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
    pub fn exe_path(&self) -> &Path {
        &self.exe_path
    }

    pub fn mode(&self) -> BridgeMode {
        self.mode
    }
}
