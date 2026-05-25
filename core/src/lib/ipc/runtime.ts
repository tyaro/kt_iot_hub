import { ipcInvoke } from './_invoke';

export interface StartRuntimeServicesRequest {
  driver_ui_base_dir?: string | null;
}

export interface RuntimeStatusDto {
  drivers_running: boolean;
  publishers_running: boolean;
  grpc_running: boolean;
  last_error?: string | null;
}

export interface RuntimeStartupConfigDto {
  auto_start_runtime_services: boolean;
}

export interface SetRuntimeStartupConfigRequest {
  auto_start_runtime_services: boolean;
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

type RuntimeStartupConfigDtoRaw = {
  autoStartRuntimeServices: boolean;
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

function normalizeRuntimeStartupConfig(
  raw: RuntimeStartupConfigDtoRaw,
): RuntimeStartupConfigDto {
  return {
    auto_start_runtime_services: raw.autoStartRuntimeServices,
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
  const raw = await ipcInvoke<RuntimeStatusDtoRaw>('get_runtime_status');
  return normalizeRuntimeStatus(raw);
}

/**
 * バックグラウンドサービスを起動する
 */
export async function startRuntimeServices(
  req?: StartRuntimeServicesRequest,
): Promise<RuntimeStatusDto> {
  const raw = await ipcInvoke<RuntimeStatusDtoRaw>('start_runtime_services', {
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
  const raw = await ipcInvoke<RuntimeStatusDtoRaw>('stop_runtime_services');
  return normalizeRuntimeStatus(raw);
}

/**
 * 起動時のランタイム自動開始設定を取得する
 */
export async function getRuntimeStartupConfig(): Promise<RuntimeStartupConfigDto> {
  const raw = await ipcInvoke<RuntimeStartupConfigDtoRaw>('get_runtime_startup_config');
  return normalizeRuntimeStartupConfig(raw);
}

/**
 * 起動時のランタイム自動開始設定を保存する
 */
export async function setRuntimeStartupConfig(
  req: SetRuntimeStartupConfigRequest,
): Promise<RuntimeStartupConfigDto> {
  const raw = await ipcInvoke<RuntimeStartupConfigDtoRaw>('set_runtime_startup_config', {
    req: {
      autoStartRuntimeServices: req.auto_start_runtime_services,
    },
  });
  return normalizeRuntimeStartupConfig(raw);
}

/**
 * アプリメトリクスを取得する
 */
export async function getAppMetrics(): Promise<AppMetricsDto> {
  const raw = await ipcInvoke<AppMetricsDtoRaw>('get_app_metrics');
  return normalizeAppMetrics(raw);
}

/**
 * ドライバ別メトリクスを取得する
 */
export async function getDriverMetrics(): Promise<DriverMetricsDto[]> {
  const raw = await ipcInvoke<DriverMetricsDtoRaw[]>('get_driver_metrics');
  return raw.map(normalizeDriverMetrics);
}
