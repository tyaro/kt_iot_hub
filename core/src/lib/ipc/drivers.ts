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

export interface DiscoverDriverPackagesRequest {
  driver_ui_base_dir: string;
}

export interface DiscoveredDriverPackageDto {
  driver_type: string;
  display_name: string;
  manifest_path: string;
  registration_ui_path: string;
  runtime_path: string;
  version?: string | null;
  vendor?: string | null;
  capabilities: string[];
}

export interface InvalidDriverPackageDto {
  manifest_path: string;
  status_code: string;
  status_message: string;
  driver_type_hint?: string | null;
}

export interface DiscoverDriverPackagesResponse {
  available_count: number;
  invalid_count: number;
  available: DiscoveredDriverPackageDto[];
  invalid: InvalidDriverPackageDto[];
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

/**
 * ドライバマニフェストを走査して利用可能なドライバ一覧を取得する
 */
export async function discoverDriverPackages(
  req: DiscoverDriverPackagesRequest,
): Promise<DiscoverDriverPackagesResponse> {
  return ipcInvoke('discover_driver_packages', {
    req: {
      driverUiBaseDir: req.driver_ui_base_dir,
    },
  });
}
