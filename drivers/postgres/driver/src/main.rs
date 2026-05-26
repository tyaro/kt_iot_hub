// リリースビルドでコンソールウィンドウが表示されないよう Windows サブシステムとして宣言。
// ドライバは gRPC バックグラウンドサービスのためコンソールは不要。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../../../apps/common/driver_runtime_grpc_client.rs"]
mod grpc_client;
mod postgres_poller;

use anyhow::Result;
use clap::Parser;
use grpc_client::DriverRuntimeGrpcClient;
use postgres_poller::PostgresPoller;
use std::time::Duration;
use tracing::{error, info, warn};

pub mod proto {
    tonic::include_proto!("kt_iot_hub.driver_runtime");
}

#[derive(Debug, Parser)]
#[command(author, version, about = "PostgreSQL runtime driver process")]
struct Args {
    #[arg(long)]
    driver_id: String,

    #[arg(long)]
    driver_kind: String,

    #[arg(long, default_value = "127.0.0.1:55051")]
    grpc_addr: String,

    #[arg(long)]
    parent_pid: Option<u32>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = run().await {
        error!("driver-postgres terminated with error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();
    start_parent_exit_watcher(args.parent_pid);
    info!(
        "driver-postgres starting: driver_id={} kind={} grpc={}",
        args.driver_id, args.driver_kind, args.grpc_addr
    );

    loop {
        let mut client = match DriverRuntimeGrpcClient::connect(
            &args.grpc_addr,
            args.driver_id.clone(),
            args.driver_kind.clone(),
        )
        .await
        {
            Ok(client) => client,
            Err(e) => {
                warn!("gRPC connect failed: {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let definition = match client.get_driver_definition().await {
            Ok(def) => def,
            Err(e) => {
                warn!("GetDriverDefinition failed: {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let poller = match PostgresPoller::from_definition(definition) {
            Ok(poller) => poller,
            Err(e) => {
                warn!("Invalid driver definition: {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let stream_task = tokio::spawn(async move { client.stream_tag_values(rx).await });

        let poll_result = poller.run(tx).await;
        if let Err(e) = poll_result {
            warn!("poller loop ended with error: {}", e);
        }

        stream_task.abort();
        let _ = stream_task.await;

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

#[cfg(windows)]
fn start_parent_exit_watcher(parent_pid: Option<u32>) {
    let Some(parent_pid) = parent_pid else {
        return;
    };

    std::thread::spawn(move || {
        const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
        const INFINITE: u32 = 0xFFFF_FFFF;
        const WAIT_OBJECT_0: u32 = 0;

        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(
                desired_access: u32,
                inherit_handle: i32,
                process_id: u32,
            ) -> *mut core::ffi::c_void;
            fn WaitForSingleObject(handle: *mut core::ffi::c_void, milliseconds: u32) -> u32;
            fn CloseHandle(handle: *mut core::ffi::c_void) -> i32;
        }

        // SAFETY: Win32 API 呼び出しは null チェック済みハンドルに限定する。
        unsafe {
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, parent_pid);
            if handle.is_null() {
                warn!(
                    parent_pid,
                    "failed to open parent process handle; parent watcher disabled"
                );
                return;
            }

            let wait_result = WaitForSingleObject(handle, INFINITE);
            let _ = CloseHandle(handle);

            if wait_result == WAIT_OBJECT_0 {
                warn!(
                    parent_pid,
                    "parent process exited; terminating driver-postgres"
                );
                std::process::exit(0);
            }
        }
    });
}

#[cfg(not(windows))]
fn start_parent_exit_watcher(_parent_pid: Option<u32>) {}
