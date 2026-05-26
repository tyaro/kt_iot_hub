// リリースビルドでコンソールウィンドウが表示されないよう Windows サブシステムとして宣言。
// ドライバは gRPC バックグラウンドサービスのためコンソールは不要。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../../../apps/common/driver_runtime_grpc_client.rs"]
mod grpc_client;
mod joywatcher_artifacts;
mod joywatcher_bridge;
mod joywatcher_connection;
mod joywatcher_ffi;
mod joywatcher_runtime;
mod server_guard;
#[allow(dead_code)]
#[path = "../../common/path_utils.rs"]
mod path_utils;

use anyhow::{anyhow, Result};
use clap::Parser;
use grpc_client::DriverRuntimeGrpcClient;
use joywatcher_artifacts::JoyWatcherArtifacts;
use joywatcher_bridge::{BridgeMode, JoyWatcherBridgeProcess};
use joywatcher_connection::JoyWatcherConnectionPlan;
use joywatcher_ffi::observed_calling_conventions;
use joywatcher_runtime::JoyWatcherPollPlan;
use server_guard::JoyWatcherServerGuard;
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
        error!("driver-joywatcher terminated with error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();
    start_parent_exit_watcher(args.parent_pid);
    let instance_name = format!(
        "Global\\kt_iot_hub.driver_joywatcher.{}",
        sanitize_instance_token(&args.driver_id)
    );
    let _instance_guard = acquire_single_instance(&instance_name).map_err(|error| {
        anyhow!(
            "driver-joywatcher single-instance guard rejected startup (driver_id={}): {}",
            args.driver_id,
            error
        )
    })?;
    info!(
        "driver-joywatcher starting: driver_id={} kind={} grpc={}",
        args.driver_id, args.driver_kind, args.grpc_addr
    );

    let connection_plan = JoyWatcherConnectionPlan::default();
    info!("JoyWatcher connection plan: {}", connection_plan.summary());
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

    let mut bridge = None;

    // JoyWatcher DLL 実装時の接続ライフサイクルメモ:
    // - `ConnectNet()` は複数回呼んでもよいが、呼んだ回数ぶん `DisconnectNet()` を実行する
    // - 通常終了時は `DisconnectNet()` で参照カウントを戻す
    // - 異常終了や参照カウント不整合時の最終退避として `DisconnectNetForce()` も候補に含める
    // - したがって FFI の最初の実装対象は `ConnectNet` / `DisconnectNet` /
    //   `DisconnectNetForce` / `JWGetTagIDS2` / `JWRead` のセットで考える

    if cfg!(all(target_os = "windows", target_pointer_width = "64")) {
        warn!(
            "JoyWatcher runtime build is 64-bit. Native DLL loading stays in the x86 bridge process, while this runtime only orchestrates bridge startup and gRPC."
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

        let poll_plan = JoyWatcherPollPlan::from_definition(&definition);
        let server_guard = JoyWatcherServerGuard::from_connection_settings(
            definition.connection.as_ref().map(|connection| &connection.settings),
        );

        if let Err(error) = server_guard.ensure_primary_server_ready() {
            warn!(
                "JoyWatcher primary server guard blocked polling start: {}",
                error
            );
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        if poll_plan.groups.is_empty() {
            warn!("JoyWatcher definition has no scan groups with enabled nativeTagId tags");
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        ensure_bridge_started(&artifacts, &mut bridge);
        let heartbeat_ok = match bridge.as_mut() {
            Some(active_bridge) => match active_bridge.ping() {
                Ok(response) => {
                    info!("JoyWatcher bridge heartbeat ok: {}", response);
                    true
                }
                Err(error) => {
                    warn!("JoyWatcher bridge heartbeat failed: {}", error);
                    active_bridge.disconnect_best_effort("heartbeat failed");
                    false
                }
            },
            None => false,
        };
        if !heartbeat_ok {
            if bridge.take().is_some() {
                info!("JoyWatcher bridge process dropped after heartbeat failure");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        let Some(active_bridge) = bridge.as_mut() else {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };

        if let Err(error) =
            server_guard.ensure_bridge_connection_ready(active_bridge, &poll_plan.connection)
        {
            warn!(
                "JoyWatcher bridge readiness guard blocked polling start: {}",
                error
            );
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let stream_task = tokio::spawn(async move { client.stream_tag_values(rx).await });

        let poll_result = poll_plan.run(active_bridge, tx).await;
        let mut should_drop_bridge = false;
        if let Err(error) = poll_result {
            warn!("JoyWatcher poller loop ended with error: {}", error);
            active_bridge.disconnect_best_effort("poll loop error");
            should_drop_bridge = true;
        }

        stream_task.abort();
        let _ = stream_task.await;

        if should_drop_bridge && bridge.take().is_some() {
            info!("JoyWatcher bridge process dropped after poll loop error");
        }

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
                    "parent process exited; terminating driver-joywatcher"
                );
                std::process::exit(0);
            }
        }
    });
}

#[cfg(not(windows))]
fn start_parent_exit_watcher(_parent_pid: Option<u32>) {}

fn ensure_bridge_started(
    artifacts: &JoyWatcherArtifacts,
    bridge: &mut Option<JoyWatcherBridgeProcess>,
) {
    if bridge.is_some() {
        return;
    }

    *bridge = restart_bridge(artifacts);
}

fn restart_bridge(artifacts: &JoyWatcherArtifacts) -> Option<JoyWatcherBridgeProcess> {
    match JoyWatcherBridgeProcess::start(artifacts) {
        Ok(mut bridge) => {
            info!(
                "JoyWatcher bridge started: mode={:?} exe= {}",
                bridge.mode(),
                bridge.exe_path().display()
            );

            match bridge.ping() {
                Ok(response) => info!("JoyWatcher bridge ping ok: {}", response),
                Err(error) => warn!("JoyWatcher bridge ping failed: {}", error),
            }

            if matches!(bridge.mode(), BridgeMode::Dll) {
                info!(
                    "JoyWatcher bridge connect will be deferred until driver settings are loaded"
                );
            }

            Some(bridge)
        }
        Err(error) => {
            warn!("JoyWatcher bridge start failed: {}", error);
            None
        }
    }
}

#[cfg(windows)]
struct SingleInstanceGuard {
    handle: *mut core::ffi::c_void,
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[link(name = "kernel32")]
        extern "system" {
            fn CloseHandle(handle: *mut core::ffi::c_void) -> i32;
        }

        // SAFETY: CreateMutexW で取得した有効ハンドルのみを保持する。
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
fn acquire_single_instance(name: &str) -> Result<SingleInstanceGuard> {
    const ERROR_ALREADY_EXISTS: u32 = 183;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            mutex_attributes: *mut core::ffi::c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: *mut core::ffi::c_void) -> i32;
    }

    let mut wide_name = name.encode_utf16().collect::<Vec<u16>>();
    wide_name.push(0);

    // SAFETY: Null終端 UTF-16 名称を渡し、戻りハンドルは null 判定して扱う。
    unsafe {
        let handle = CreateMutexW(std::ptr::null_mut(), 0, wide_name.as_ptr());
        if handle.is_null() {
            return Err(anyhow!("CreateMutexW returned null"));
        }

        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            return Err(anyhow!("mutex already exists: {}", name));
        }

        Ok(SingleInstanceGuard { handle })
    }
}

#[cfg(not(windows))]
struct SingleInstanceGuard;

#[cfg(not(windows))]
fn acquire_single_instance(_name: &str) -> Result<SingleInstanceGuard> {
    Ok(SingleInstanceGuard)
}

fn sanitize_instance_token(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "default".to_string()
    } else {
        sanitized
    }
}
