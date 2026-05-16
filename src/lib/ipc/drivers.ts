import { invoke } from '@tauri-apps/api/core';

export interface DriverDto {
  id: string;
  driver_type: string;
  enabled: boolean;
  registration_ui_available: boolean;
  host: string;
  port: number;
  database: string;
  username: string;
}

export interface SaveDriverRequest {
  id: string;
  driver_type: string;
  enabled: boolean;
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
}

/**
 * すべてのドライバ設定を取得する
 */
export async function listDrivers(): Promise<DriverDto[]> {
  return invoke('list_drivers');
}

/**
 * ドライバ設定を保存する
 */
export async function saveDriver(req: SaveDriverRequest): Promise<void> {
  return invoke('save_driver', { req });
}

/**
 * ドライバ設定を削除する
 */
export async function deleteDriver(driverId: string): Promise<void> {
  return invoke('delete_driver', { driverId });
}
