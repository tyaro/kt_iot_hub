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

export interface TagManagementSettingsTransferResponse {
  path: string;
  driver_count: number;
  scan_group_count: number;
  tag_count: number;
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

/**
 * タグ管理設定を JSON へエクスポートする
 */
export async function exportTagManagementSettings(
  path: string,
): Promise<TagManagementSettingsTransferResponse> {
  return ipcInvoke('export_tag_management_settings', { req: { path } });
}

/**
 * タグ管理設定を JSON からインポートする
 */
export async function importTagManagementSettings(
  path: string,
): Promise<TagManagementSettingsTransferResponse> {
  return ipcInvoke('import_tag_management_settings', { req: { path } });
}

/**
 * ドライバ配置の既定ベースパスを取得する
 */
export async function getDefaultDriverUiBaseDir(): Promise<string | null> {
  return ipcInvoke('get_default_driver_ui_base_dir');
}
