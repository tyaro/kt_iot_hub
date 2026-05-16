import { invoke } from '@tauri-apps/api/core';

export interface TagDto {
  id: string;
  name: string;
  data_type: string;
  driver_id: string;
  scan_group_id: string;
  driver_spec: Record<string, unknown>;
}

export interface ScanGroupDto {
  id: string;
  driver_id: string;
  table?: string;
  timestamp_column?: string;
  scan_rate_ms?: number;
}

export interface CreateTagRequest {
  id: string;
  name: string;
  data_type: string;
  driver_id: string;
  scan_group_id: string;
  driver_spec: Record<string, unknown>;
}

/**
 * タグを作成する
 */
export async function createTag(req: CreateTagRequest): Promise<TagDto> {
  return invoke('create_tag', { req });
}

/**
 * すべてのタグを取得する
 */
export async function listTags(): Promise<TagDto[]> {
  return invoke('list_tags');
}

/**
 * スキャングループ一覧を取得する
 */
export async function listScanGroups(driverId?: string): Promise<ScanGroupDto[]> {
  return invoke('list_scan_groups', { driverId: driverId ?? null });
}

/**
 * タグを削除する
 */
export async function deleteTag(tagId: string): Promise<void> {
  return invoke('delete_tag', { tagId });
}
