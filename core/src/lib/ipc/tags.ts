import { ipcInvoke } from './_invoke';

export interface TagShape {
  id: string;
  name: string;
  data_type: string;
  driver_id: string;
  scan_group_id: string;
  driver_spec: Record<string, unknown>;
}

export type TagDto = TagShape;

export interface ScanGroupDto {
  id: string;
  driver_id: string;
  table?: string;
  timestamp_column?: string;
  scan_rate_ms?: number;
  observed_cycle_ms?: number;
  observed_p95_cycle_ms?: number;
  cycle_delta_ratio?: number;
  cycle_status?: string;
}

export interface UpdateScanGroupRateRequest {
  driver_id: string;
  scan_group_id: string;
  scan_rate_ms: number;
}

export interface BulkUpdateDriverScanGroupRateRequest {
  driver_id: string;
  scan_rate_ms: number;
}

export interface BulkUpdateScanGroupsResult {
  driver_id: string;
  updated_count: number;
  scan_rate_ms: number;
}

export type CreateTagRequest = TagShape;

type ApiTagDto = {
  id: string;
  name: string;
  dataType: string;
  driverId: string;
  scanGroupId: string;
  driverSpec: Record<string, unknown>;
};

type ApiScanGroupDto = {
  id: string;
  driverId: string;
  table?: string;
  timestampColumn?: string;
  scanRateMs?: number;
  observedCycleMs?: number;
  observedP95CycleMs?: number;
  cycleDeltaRatio?: number;
  cycleStatus?: string;
};

type ApiCreateTagRequest = {
  id: string;
  name: string;
  dataType: string;
  driverId: string;
  scanGroupId: string;
  driverSpec: Record<string, unknown>;
};

type ApiUpdateScanGroupRateRequest = {
  driverId: string;
  scanGroupId: string;
  scanRateMs: number;
};

type ApiBulkUpdateDriverScanGroupRateRequest = {
  driverId: string;
  scanRateMs: number;
};

type ApiBulkUpdateScanGroupsResult = {
  driverId: string;
  updatedCount: number;
  scanRateMs: number;
};

function mapTagFromApi(api: ApiTagDto): TagDto {
  return {
    id: api.id,
    name: api.name,
    data_type: api.dataType,
    driver_id: api.driverId,
    scan_group_id: api.scanGroupId,
    driver_spec: api.driverSpec,
  };
}

function mapScanGroupFromApi(api: ApiScanGroupDto): ScanGroupDto {
  return {
    id: api.id,
    driver_id: api.driverId,
    table: api.table,
    timestamp_column: api.timestampColumn,
    scan_rate_ms: api.scanRateMs,
    observed_cycle_ms: api.observedCycleMs,
    observed_p95_cycle_ms: api.observedP95CycleMs,
    cycle_delta_ratio: api.cycleDeltaRatio,
    cycle_status: api.cycleStatus,
  };
}

function mapCreateTagToApi(req: CreateTagRequest): ApiCreateTagRequest {
  return {
    id: req.id,
    name: req.name,
    dataType: req.data_type,
    driverId: req.driver_id,
    scanGroupId: req.scan_group_id,
    driverSpec: req.driver_spec,
  };
}

function mapUpdateScanGroupRateToApi(
  req: UpdateScanGroupRateRequest,
): ApiUpdateScanGroupRateRequest {
  return {
    driverId: req.driver_id,
    scanGroupId: req.scan_group_id,
    scanRateMs: req.scan_rate_ms,
  };
}

function mapBulkUpdateDriverScanGroupRateToApi(
  req: BulkUpdateDriverScanGroupRateRequest,
): ApiBulkUpdateDriverScanGroupRateRequest {
  return {
    driverId: req.driver_id,
    scanRateMs: req.scan_rate_ms,
  };
}

function mapBulkUpdateScanGroupsResultFromApi(
  api: ApiBulkUpdateScanGroupsResult,
): BulkUpdateScanGroupsResult {
  return {
    driver_id: api.driverId,
    updated_count: api.updatedCount,
    scan_rate_ms: api.scanRateMs,
  };
}

/**
 * タグを作成する
 */
export async function createTag(req: CreateTagRequest): Promise<TagDto> {
  const api = await ipcInvoke<ApiTagDto>('create_tag', {
    req: mapCreateTagToApi(req),
  });
  return mapTagFromApi(api);
}

/**
 * すべてのタグを取得する
 */
export async function listTags(): Promise<TagDto[]> {
  const api = await ipcInvoke<ApiTagDto[]>('list_tags');
  return api.map(mapTagFromApi);
}

/**
 * スキャングループ一覧を取得する
 */
export async function listScanGroups(driverId?: string): Promise<ScanGroupDto[]> {
  const api = await ipcInvoke<ApiScanGroupDto[]>('list_scan_groups', {
    driverId: driverId ?? null,
  });
  return api.map(mapScanGroupFromApi);
}

/**
 * 1件の ScanGroup 周期を更新する
 */
export async function updateScanGroupRate(
  req: UpdateScanGroupRateRequest,
): Promise<ScanGroupDto> {
  const api = await ipcInvoke<ApiScanGroupDto>('update_scan_group_rate', {
    req: mapUpdateScanGroupRateToApi(req),
  });
  return mapScanGroupFromApi(api);
}

/**
 * 指定接続先配下の ScanGroup 周期を一括更新する
 */
export async function bulkUpdateDriverScanGroupRate(
  req: BulkUpdateDriverScanGroupRateRequest,
): Promise<BulkUpdateScanGroupsResult> {
  const api = await ipcInvoke<ApiBulkUpdateScanGroupsResult>(
    'bulk_update_driver_scan_group_rate',
    {
      req: mapBulkUpdateDriverScanGroupRateToApi(req),
    },
  );
  return mapBulkUpdateScanGroupsResultFromApi(api);
}

/**
 * タグを削除する
 */
export async function deleteTag(tagId: string): Promise<void> {
  return ipcInvoke('delete_tag', { tagId });
}
