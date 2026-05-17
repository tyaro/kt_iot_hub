mod connection;
mod dll_api;
mod mock_api;
mod protocol;
mod service;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use dll_api::JoyWatcherDllApi;
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
}

fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(error) = run() {
        error!("joywatcher-bridge-x86 terminated with error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let api: Box<dyn connection::JoyWatcherBridgeApi> = match args.mode.as_str() {
        "mock" => {
            info!("joywatcher-bridge-x86 starting in mock mode");
            Box::new(MockJoyWatcherApi::default())
        }
        "dll" => {
            let api = JoyWatcherDllApi::new(args.dll_path.clone())?;
            info!(
                "joywatcher-bridge-x86 starting in dll mode: dll={}",
                api.dll_path().display()
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

fn write_response(stdout: &mut dyn Write, response: &BridgeResponse) -> Result<()> {
    serde_json::to_writer(&mut *stdout, response).context("failed to serialize response")?;
    writeln!(stdout).context("failed to write newline")?;
    stdout.flush().context("failed to flush stdout")?;
    Ok(())
}
