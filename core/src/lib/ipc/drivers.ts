import { ipcInvoke } from './_invoke';

interface DriverCoreFields {
  id: string;
  driver_type: string;
  enabled: boolean;
  host: string;
  port: number;
  database: string;
  username: string;
  password_key?: string | null;
  tls_enabled: boolean;
  tls_ca_path?: string | null;
  tls_client_cert_path?: string | null;
  tls_client_key_path?: string | null;
  connect_timeout_ms?: number | null;
  statement_timeout_ms?: number | null;
  auto_restart: boolean;
  max_restart_per_minute?: number | null;
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

type ApiDiscoveredDriverPackageDto = {
  driverType: string;
  displayName: string;
  manifestPath: string;
  registrationUiPath: string;
  runtimePath: string;
  version?: string | null;
  vendor?: string | null;
  capabilities: string[];
};

type ApiInvalidDriverPackageDto = {
  manifestPath: string;
  statusCode: string;
  statusMessage: string;
  driverTypeHint?: string | null;
};

type ApiDiscoverDriverPackagesResponse = {
  availableCount: number;
  invalidCount: number;
  available: ApiDiscoveredDriverPackageDto[];
  invalid: ApiInvalidDriverPackageDto[];
};

function mapDiscoveredDriverPackageFromApi(
  api: ApiDiscoveredDriverPackageDto,
): DiscoveredDriverPackageDto {
  return {
    driver_type: api.driverType,
    display_name: api.displayName,
    manifest_path: api.manifestPath,
    registration_ui_path: api.registrationUiPath,
    runtime_path: api.runtimePath,
    version: api.version,
    vendor: api.vendor,
    capabilities: api.capabilities,
  };
}

function mapInvalidDriverPackageFromApi(api: ApiInvalidDriverPackageDto): InvalidDriverPackageDto {
  return {
    manifest_path: api.manifestPath,
    status_code: api.statusCode,
    status_message: api.statusMessage,
    driver_type_hint: api.driverTypeHint,
  };
}

function mapDiscoverDriverPackagesResponseFromApi(
  api: ApiDiscoverDriverPackagesResponse,
): DiscoverDriverPackagesResponse {
  return {
    available_count: api.availableCount,
    invalid_count: api.invalidCount,
    available: api.available.map(mapDiscoveredDriverPackageFromApi),
    invalid: api.invalid.map(mapInvalidDriverPackageFromApi),
  };
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
  const api = await ipcInvoke<ApiDiscoverDriverPackagesResponse>('discover_driver_packages', {
    req: {
      driverUiBaseDir: req.driver_ui_base_dir,
    },
  });

  return mapDiscoverDriverPackagesResponseFromApi(api);
}
