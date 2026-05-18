import { ipcInvoke } from './_invoke';

interface DriverCoreFields {
  id: string;
  driver_type: string;
  enabled: boolean;
  host: string;
  port: number;
  database: string;
  username: string;
}

export interface DriverDto extends DriverCoreFields {
  registration_ui_available: boolean;
}

export interface SaveDriverRequest extends DriverCoreFields {
  original_id?: string | null;
  password: string;
}

/**
 * すべてのドライバ設定を取得する
 */
export async function listDrivers(): Promise<DriverDto[]> {
  return ipcInvoke('list_drivers');
}

/**
 * ドライバ設定を保存する
 */
export async function saveDriver(req: SaveDriverRequest): Promise<void> {
  return ipcInvoke('save_driver', { req });
}

/**
 * ドライバ設定を削除する
 */
export async function deleteDriver(driverId: string): Promise<void> {
  return ipcInvoke('delete_driver', { driverId });
}
