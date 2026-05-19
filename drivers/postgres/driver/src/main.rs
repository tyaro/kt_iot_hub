#[path = "../../../common/driver_runtime_grpc_client.rs"]
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
