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

type RuntimeStatusDtoRaw = {
  driversRunning: boolean;
  publishersRunning: boolean;
  grpcRunning: boolean;
  lastError?: string | null;
};

function normalizeRuntimeStatus(raw: RuntimeStatusDtoRaw): RuntimeStatusDto {
  return {
    drivers_running: raw.driversRunning,
    publishers_running: raw.publishersRunning,
    grpc_running: raw.grpcRunning,
    last_error: raw.lastError ?? null,
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
