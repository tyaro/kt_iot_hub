mod grpc_client;
mod joywatcher_artifacts;
mod joywatcher_connection;
mod joywatcher_ffi;

use anyhow::Result;
use clap::Parser;
use grpc_client::DriverRuntimeGrpcClient;
use joywatcher_artifacts::JoyWatcherArtifacts;
use joywatcher_connection::JoyWatcherConnectionPlan;
use joywatcher_ffi::observed_calling_conventions;
use std::time::Duration;
use tracing::{error, info, warn};

pub mod proto {
    tonic::include_proto!("kt_iot_hub.driver_runtime");
}

#[derive(Debug, Parser)]
#[command(author, version, about = "JoyWatcher runtime driver process")]
struct Args {
    #[arg(long)]
    driver_id: String,

    #[arg(long)]
    driver_kind: String,

    #[arg(long, default_value = "127.0.0.1:55051")]
    grpc_addr: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = run().await {
        error!("driver-joywatcher terminated with error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();
    info!(
        "driver-joywatcher starting: driver_id={} kind={} grpc={}",
        args.driver_id, args.driver_kind, args.grpc_addr
    );

    let connection_plan = JoyWatcherConnectionPlan::default();
    info!(
        "JoyWatcher connection plan: {}",
        connection_plan.summary()
    );
    info!(
        "JoyWatcher planned FFI symbols: {}",
        connection_plan.required_symbols().join(", ")
    );
    info!(
        "JoyWatcher observed calling conventions: {}",
        observed_calling_conventions()
            .iter()
            .map(|cc| cc.summary())
            .collect::<Vec<_>>()
            .join(" / ")
    );

    let artifacts = JoyWatcherArtifacts::inspect();
    info!("{}", artifacts.status_message());
    if let Some(message) = artifacts.missing_dll_warning() {
        warn!("{}", message);
    }

    // JoyWatcher DLL 実装時の接続ライフサイクルメモ:
    // - `ConnectNet()` は複数回呼んでもよいが、呼んだ回数ぶん `DisconnectNet()` を実行する
    // - 通常終了時は `DisconnectNet()` で参照カウントを戻す
    // - 異常終了や参照カウント不整合時の最終退避として `DisconnectNetForce()` も候補に含める
    // - したがって FFI の最初の実装対象は `ConnectNet` / `DisconnectNet` /
    //   `DisconnectNetForce` / `JWGetTagIDS2` / `JWRead` のセットで考える

    if cfg!(all(target_os = "windows", target_pointer_width = "64")) {
        warn!(
            "JoyWatcher DLL may be 32-bit only. Current runtime build is 64-bit, so this scaffold intentionally avoids DLL loading until an x86 build path is prepared."
        );
    }

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
            Ok(definition) => definition,
            Err(e) => {
                warn!("GetDriverDefinition failed: {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        info!(
            "JoyWatcher definition loaded: scan_groups={} tags={}",
            definition.scan_groups.len(),
            definition.tags.len()
        );

        if definition.scan_groups.is_empty() {
            warn!("JoyWatcher definition has no scan groups yet");
        }
        if definition.tags.is_empty() {
            warn!("JoyWatcher definition has no tags yet");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
