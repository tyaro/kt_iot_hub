// Tauri IPC のラッパー
// フロントエンドから型安全にバックエンド関数を呼び出す

import { invoke } from '@tauri-apps/api/core';

export interface TagDto {
  id: string;
  name: string;
  data_type: string;
  driver_id: string;
  scan_group_id: string;
  driver_spec: Record<string, unknown>;
}

export interface CreateTagRequest {
  id: string;
  name: string;
  data_type: string;
  driver_id: string;
  scan_group_id: string;
  driver_spec: Record<string, unknown>;
}

export interface DriverDto {
  id: string;
  driver_type: string;
  enabled: boolean;
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
 * タグを削除する
 */
export async function deleteTag(tagId: string): Promise<void> {
  return invoke('delete_tag', { tag_id: tagId });
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
