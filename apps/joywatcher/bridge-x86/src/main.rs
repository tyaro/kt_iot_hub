mod connection;
mod mock_api;
mod protocol;
mod service;

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use clap::Parser;
use protocol::{BridgeRequest, BridgeResponse};
use service::JoyWatcherBridgeService;
use tracing::{error, info, warn};

#[derive(Debug, Parser)]
#[command(author, version, about = "JoyWatcher x86 bridge process")]
struct Args {
    #[arg(long, default_value = "mock")]
    mode: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(error) = run() {
        error!("joywatcher-bridge-x86 terminated with error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    if args.mode != "mock" {
        warn!("unsupported mode '{}'; falling back to mock", args.mode);
    }

    info!("joywatcher-bridge-x86 starting in mock mode");

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut service = JoyWatcherBridgeService::default();

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
