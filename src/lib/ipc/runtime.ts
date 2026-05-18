import { invoke } from '@tauri-apps/api/core';

export interface StartRuntimeServicesRequest {
  driver_ui_base_dir?: string | null;
}

export interface RuntimeStatusDto {
  drivers_running: boolean;
  publishers_running: boolean;
  grpc_running: boolean;
  last_error?: string | null;
}

export interface AppMetricsDto {
  process_cpu_percent?: number | null;
  process_memory_bytes?: number | null;
  system_cpu_percent?: number | null;
  system_memory_used_bytes?: number | null;
  system_memory_total_bytes?: number | null;
  network_rx_bytes_per_sec?: number | null;
  network_tx_bytes_per_sec?: number | null;
  sampled_at: string;
}

export interface DriverMetricsDto {
  driver_id: string;
  driver_type: string;
  pid: number;
  cpu_percent?: number | null;
  memory_bytes?: number | null;
  network_rx_bytes_per_sec?: number | null;
  network_tx_bytes_per_sec?: number | null;
}

type RuntimeStatusDtoRaw = {
  driversRunning: boolean;
  publishersRunning: boolean;
  grpcRunning: boolean;
  lastError?: string | null;
};

type AppMetricsDtoRaw = {
  processCpuPercent?: number | null;
  processMemoryBytes?: number | null;
  systemCpuPercent?: number | null;
  systemMemoryUsedBytes?: number | null;
  systemMemoryTotalBytes?: number | null;
  networkRxBytesPerSec?: number | null;
  networkTxBytesPerSec?: number | null;
  sampledAt: string;
};

type DriverMetricsDtoRaw = {
  driverId: string;
  driverType: string;
  pid: number;
  cpuPercent?: number | null;
  memoryBytes?: number | null;
  networkRxBytesPerSec?: number | null;
  networkTxBytesPerSec?: number | null;
};

function normalizeRuntimeStatus(raw: RuntimeStatusDtoRaw): RuntimeStatusDto {
  return {
    drivers_running: raw.driversRunning,
    publishers_running: raw.publishersRunning,
    grpc_running: raw.grpcRunning,
    last_error: raw.lastError ?? null,
  };
}

function normalizeAppMetrics(raw: AppMetricsDtoRaw): AppMetricsDto {
  return {
    process_cpu_percent: raw.processCpuPercent ?? null,
    process_memory_bytes: raw.processMemoryBytes ?? null,
    system_cpu_percent: raw.systemCpuPercent ?? null,
    system_memory_used_bytes: raw.systemMemoryUsedBytes ?? null,
    system_memory_total_bytes: raw.systemMemoryTotalBytes ?? null,
    network_rx_bytes_per_sec: raw.networkRxBytesPerSec ?? null,
    network_tx_bytes_per_sec: raw.networkTxBytesPerSec ?? null,
    sampled_at: raw.sampledAt,
  };
}

function normalizeDriverMetrics(raw: DriverMetricsDtoRaw): DriverMetricsDto {
  return {
    driver_id: raw.driverId,
    driver_type: raw.driverType,
    pid: raw.pid,
    cpu_percent: raw.cpuPercent ?? null,
    memory_bytes: raw.memoryBytes ?? null,
    network_rx_bytes_per_sec: raw.networkRxBytesPerSec ?? null,
    network_tx_bytes_per_sec: raw.networkTxBytesPerSec ?? null,
  };
}

/**
 * ランタイム状態を取得する
 */
export async function getRuntimeStatus(): Promise<RuntimeStatusDto> {
  const raw = await invoke<RuntimeStatusDtoRaw>('get_runtime_status');
  return normalizeRuntimeStatus(raw);
}

/**
 * バックグラウンドサービスを起動する
 */
export async function startRuntimeServices(
  req?: StartRuntimeServicesRequest,
): Promise<RuntimeStatusDto> {
  const raw = await invoke<RuntimeStatusDtoRaw>('start_runtime_services', {
    req: req
      ? {
          driverUiBaseDir: req.driver_ui_base_dir ?? null,
        }
      : null,
  });
  return normalizeRuntimeStatus(raw);
}

/**
 * バックグラウンドサービスを停止する
 */
export async function stopRuntimeServices(): Promise<RuntimeStatusDto> {
  const raw = await invoke<RuntimeStatusDtoRaw>('stop_runtime_services');
  return normalizeRuntimeStatus(raw);
}

/**
 * アプリメトリクスを取得する
 */
export async function getAppMetrics(): Promise<AppMetricsDto> {
  const raw = await invoke<AppMetricsDtoRaw>('get_app_metrics');
  return normalizeAppMetrics(raw);
}

/**
 * ドライバ別メトリクスを取得する
 */
export async function getDriverMetrics(): Promise<DriverMetricsDto[]> {
  const raw = await invoke<DriverMetricsDtoRaw[]>('get_driver_metrics');
  return raw.map(normalizeDriverMetrics);
}
