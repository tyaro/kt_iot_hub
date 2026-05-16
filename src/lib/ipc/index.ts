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

export interface LaunchDriverUiRequest {
  driver_id?: string | null;
  driver_type?: string | null;
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

export interface LaunchDriverUiResponse {
  session_id: string;
  output_json_path: string;
  driver_id?: string | null;
  driver_type: string;
}

export interface CheckDriverUiResultRequest {
  output_json_path: string;
}

export interface CheckDriverUiResultResponse {
  ready: boolean;
}

export interface ImportDriverUiResultRequest {
  session_id: string;
  driver_id?: string | null;
  output_json_path: string;
}

export interface ImportDriverUiResultResponse {
  driver_id: string;
  session_id: string;
  imported_tag_count: number;
  imported_scan_group_count: number;
}

export interface RuntimeStatusDto {
  drivers_running: boolean;
  publishers_running: boolean;
  grpc_running: boolean;
  last_error?: string | null;
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

/**
 * ドライバUIを外部プロセスとして起動する
 */
export async function launchDriverUi(req: LaunchDriverUiRequest): Promise<LaunchDriverUiResponse> {
  return invoke('launch_driver_ui', { req });
}

/**
 * ドライバUI結果JSONの生成有無を確認する
 */
export async function checkDriverUiResult(
  req: CheckDriverUiResultRequest,
): Promise<CheckDriverUiResultResponse> {
  return invoke('check_driver_ui_result', { req });
}

/**
 * ドライバUI結果JSONを検証して本体へ取り込む
 */
export async function importDriverUiResult(
  req: ImportDriverUiResultRequest,
): Promise<ImportDriverUiResultResponse> {
  return invoke('import_driver_ui_result', { req });
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
