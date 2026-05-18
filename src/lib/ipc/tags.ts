import { invoke } from '@tauri-apps/api/core';

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

/**
 * タグを作成する
 */
export async function createTag(req: CreateTagRequest): Promise<TagDto> {
  const api = await invoke<ApiTagDto>('create_tag', {
    req: mapCreateTagToApi(req),
  });
  return mapTagFromApi(api);
}

/**
 * すべてのタグを取得する
 */
export async function listTags(): Promise<TagDto[]> {
  const api = await invoke<ApiTagDto[]>('list_tags');
  return api.map(mapTagFromApi);
}

/**
 * スキャングループ一覧を取得する
 */
export async function listScanGroups(driverId?: string): Promise<ScanGroupDto[]> {
  const api = await invoke<ApiScanGroupDto[]>('list_scan_groups', {
    driverId: driverId ?? null,
  });
  return api.map(mapScanGroupFromApi);
}

/**
 * タグを削除する
 */
export async function deleteTag(tagId: string): Promise<void> {
  return invoke('delete_tag', { tagId });
}
