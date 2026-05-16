import { invoke } from '@tauri-apps/api/core';

export interface RuntimeStatusDto {
  drivers_running: boolean;
  publishers_running: boolean;
  grpc_running: boolean;
  last_error?: string | null;
}

/**
 * ランタイム状態を取得する
 */
export async function getRuntimeStatus(): Promise<RuntimeStatusDto> {
  return invoke('get_runtime_status');
}

/**
 * バックグラウンドサービスを起動する
 */
export async function startRuntimeServices(): Promise<RuntimeStatusDto> {
  return invoke('start_runtime_services');
}

/**
 * バックグラウンドサービスを停止する
 */
export async function stopRuntimeServices(): Promise<RuntimeStatusDto> {
  return invoke('stop_runtime_services');
}
