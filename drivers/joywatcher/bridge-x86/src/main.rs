// リリースビルドでコンソールウィンドウが表示されないよう Windows サブシステムとして宣言。
// AllocConsole() は run() 内で明示的に呼び出して隠しコンソールを確保する。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod connection;
mod dll_api;
mod dll_ffi;
mod dll_symbols;
mod mock_api;
#[allow(dead_code)]
#[path = "../../common/path_utils.rs"]
mod path_utils;
mod protocol;
mod service;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use dll_api::{JoyWatcherConnectConvention, JoyWatcherDllApi};
use mock_api::MockJoyWatcherApi;
use protocol::{BridgeRequest, BridgeResponse};
use service::JoyWatcherBridgeService;
use tracing::{error, info, warn};

#[derive(Debug, Parser)]
#[command(author, version, about = "JoyWatcher x86 bridge process")]
struct Args {
    #[arg(long, default_value = "mock")]
    mode: String,

    #[arg(long)]
    dll_path: Option<PathBuf>,

    #[arg(long, default_value = "cdecl")]
    connect_convention: String,
}

fn main() {
    init_tracing();

    if let Err(error) = run() {
        error!("joywatcher-bridge-x86 terminated with error: {}", error);
        std::process::exit(1);
    }
}

/// tracing-subscriber を stderr + ファイルへ分岐。
/// 親が windows-subsystem だと stderr は破棄されるため、診断にはファイルが必須。
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let log_path = resolve_log_path();
    if let Some(path) = &log_path {
        rotate_log_if_needed(path);
    }
    let file_writer = log_path.as_ref().and_then(|path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });

    let builder = tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(env_filter);

    match file_writer {
        Some(file) => {
            let writer = std::io::stderr.and(std::sync::Mutex::new(file));
            builder.with_writer(writer).init();
            if let Some(path) = log_path {
                tracing::info!(log_file = %path.display(), "bridge-x86 tracing initialized");
            }
        }
        None => {
            builder.with_writer(std::io::stderr).init();
            tracing::warn!("bridge-x86 file log not available; using stderr only");
        }
    }
}

/// ログファイルが MAX_LOG_SIZE を超えていたら .1 にリネームしてローテーション（1世代保持）。
fn rotate_log_if_needed(path: &PathBuf) {
    const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() >= MAX_LOG_SIZE {
            let rotated = path.with_extension("log.1");
            // 失敗しても起動を止めない（古い .1 が残っていても上書き）
            let _ = std::fs::rename(path, &rotated);
        }
    }
}

fn resolve_log_path() -> Option<PathBuf> {
    // 優先: %LOCALAPPDATA%\kt_iot_hub\logs\joywatcher-bridge-x86.log
    // フォールバック: %TEMP%\kt_iot_hub-joywatcher-bridge-x86.log
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let mut dir = PathBuf::from(local);
        dir.push("kt_iot_hub");
        dir.push("logs");
        if std::fs::create_dir_all(&dir).is_ok() {
            return Some(dir.join("joywatcher-bridge-x86.log"));
        }
    }
    if let Some(tmp) = std::env::var_os("TEMP") {
        return Some(PathBuf::from(tmp).join("kt_iot_hub-joywatcher-bridge-x86.log"));
    }
    None
}

fn run() -> Result<()> {
    // GUI subsystem の親（release driver-ui exe）から起動された場合、
    // 子の console プロセスはコンソールを継承しない。
    // JoyWaApi の TagSel2 ダイアログはオーナーHWND探索に GetConsoleWindow() を
    // 参照する実装になっており、コンソール未割当だとダイアログが表示されない。
    // ここで隠しコンソールを確保し、オーナーHWNDを取得可能にする。
    #[cfg(windows)]
    ensure_hidden_console();

    let args = Args::parse();
    let connect_convention = parse_connect_convention(&args.connect_convention)?;
    let api: Box<dyn connection::JoyWatcherBridgeApi> = match args.mode.as_str() {
        "mock" => {
            info!("joywatcher-bridge-x86 starting in mock mode");
            Box::new(MockJoyWatcherApi::default())
        }
        "dll" => {
            let api = JoyWatcherDllApi::new(args.dll_path.clone(), connect_convention)?;
            info!(
                "joywatcher-bridge-x86 starting in dll mode: dll={}, connect_convention={}",
                api.dll_path().display(),
                connect_convention.as_str()
            );
            Box::new(api)
        }
        other => {
            warn!("unsupported mode '{}'; falling back to mock", other);
            Box::new(MockJoyWatcherApi::default())
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut service = JoyWatcherBridgeService::new(api);

    for line in stdin.lock().lines() {
        let line = line.context("failed to read stdin line")?;
        if line.trim().is_empty() {
            continue;
        }

        let request: BridgeRequest = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = BridgeResponse::error("INVALID_REQUEST", error.to_string());
                write_response(&mut stdout, &response)?;
                continue;
            }
        };

        let response = service.handle_request(request);
        write_response(&mut stdout, &response)?;
    }

    Ok(())
}

fn parse_connect_convention(raw: &str) -> Result<JoyWatcherConnectConvention> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "cdecl" => Ok(JoyWatcherConnectConvention::Cdecl),
        "stdcall" => Ok(JoyWatcherConnectConvention::Stdcall),
        other => Err(anyhow!(
            "unsupported connect convention: {} (expected cdecl or stdcall)",
            other
        )),
    }
}

fn write_response(stdout: &mut dyn Write, response: &BridgeResponse) -> Result<()> {
    serde_json::to_writer(&mut *stdout, response).context("failed to serialize response")?;
    writeln!(stdout).context("failed to write newline")?;
    stdout.flush().context("failed to flush stdout")?;
    Ok(())
}

#[cfg(windows)]
fn ensure_hidden_console() {
    // kernel32.dll: AllocConsole / GetConsoleWindow
    // user32.dll  : ShowWindow
    #[link(name = "kernel32")]
    extern "system" {
        fn AllocConsole() -> i32;
        fn GetConsoleWindow() -> *mut core::ffi::c_void;
    }
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: *mut core::ffi::c_void, n_cmd_show: i32) -> i32;
    }
    const SW_HIDE: i32 = 0;

    unsafe {
        if GetConsoleWindow().is_null() {
            // 既にコンソールがある場合は何もしない（dev時等）。
            // 失敗しても TagSel2 以外の動作には影響しないため戻り値は無視する。
            if AllocConsole() != 0 {
                let hwnd = GetConsoleWindow();
                if !hwnd.is_null() {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    info!("AllocConsole succeeded; hidden console attached for owner HWND");
                } else {
                    warn!("AllocConsole succeeded but GetConsoleWindow returned null");
                }
            } else {
                warn!("AllocConsole failed; TagSel2 dialog may not appear under GUI parent");
            }
        }
    }
}
