import { invoke } from '@tauri-apps/api/core';

export interface LaunchDriverUiRequest {
  driver_id?: string | null;
  driver_type?: string | null;
  driver_ui_base_dir?: string | null;
  editing_tag_id?: string | null;
}

export interface LaunchDriverUiResponse {
  session_id: string;
  output_json_path: string;
  driver_id?: string | null;
  driver_type: string;
}

export interface CheckDriverUiResultRequest {
  session_id?: string | null;
  output_json_path: string;
}

export interface CheckDriverUiResultResponse {
  ready: boolean;
  process_active: boolean;
}

type CheckDriverUiResultResponseRaw = {
  ready: boolean;
  processActive: boolean;
};

function normalizeCheckDriverUiResult(
  raw: CheckDriverUiResultResponseRaw,
): CheckDriverUiResultResponse {
  return {
    ready: raw.ready,
    process_active: raw.processActive,
  };
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

export interface DriverUiLaunchContextDto {
  launchedAsDriverUi: boolean;
  sessionId?: string | null;
  driverType?: string | null;
  driverId?: string | null;
  inputJsonPath?: string | null;
  outputJsonPath?: string | null;
  requestId?: string | null;
  editingTagId?: string | null;
  context?: {
    scanGroups?: Array<{
      id: string;
      driver: string;
      scanRateMs: number;
      schema?: string | null;
      table?: string | null;
      timestampColumn?: string | null;
      node?: string | null;
      tags?: Array<{
        id: string;
        name: string;
        dataType: string;
        driverId: string;
        scanGroupId: string;
        enabled: boolean;
        driverSpec: Record<string, unknown>;
        metadata?: Record<string, unknown> | null;
      }>;
    }>;
    existingDriverIds?: string[];
    driverSettings?: Record<string, unknown> | null;
  } | null;
}

export interface SaveDriverUiOutputRequest {
  outputJsonPath?: string | null;
  payload: Record<string, unknown>;
}

/**
 * ドライバUIを外部プロセスとして起動する
 */
export async function launchDriverUi(req: LaunchDriverUiRequest): Promise<LaunchDriverUiResponse> {
  return invoke('launch_driver_ui', {
    req: {
      driverId: req.driver_id ?? null,
      driverType: req.driver_type ?? null,
      driverUiBaseDir: req.driver_ui_base_dir ?? null,
      editingTagId: req.editing_tag_id ?? null,
    },
  });
}

/**
 * ドライバUI結果JSONの生成有無を確認する
 */
export async function checkDriverUiResult(
  req: CheckDriverUiResultRequest,
): Promise<CheckDriverUiResultResponse> {
  const raw = await invoke<CheckDriverUiResultResponseRaw>('check_driver_ui_result', {
    req: {
      sessionId: req.session_id ?? null,
      outputJsonPath: req.output_json_path,
    },
  });
  return normalizeCheckDriverUiResult(raw);
}

/**
 * ドライバUI結果JSONを検証して本体へ取り込む
 */
export async function importDriverUiResult(
  req: ImportDriverUiResultRequest,
): Promise<ImportDriverUiResultResponse> {
  const raw = await invoke<{
    driverId: string;
    sessionId: string;
    importedTagCount: number;
    importedScanGroupCount: number;
  }>('import_driver_ui_result', {
    req: {
      sessionId: req.session_id,
      driverId: req.driver_id ?? null,
      outputJsonPath: req.output_json_path,
    },
  });

  return {
    driver_id: raw.driverId,
    session_id: raw.sessionId,
    imported_tag_count: raw.importedTagCount,
    imported_scan_group_count: raw.importedScanGroupCount,
  };
}

/**
 * ドライバUI起動コンテキストを取得する
 */
export async function getDriverUiLaunchContext(): Promise<DriverUiLaunchContextDto> {
  return invoke('get_driver_ui_launch_context');
}

/**
 * ドライバUIの確定結果JSONを output-json へ保存する
 */
export async function saveDriverUiOutput(req: SaveDriverUiOutputRequest): Promise<string> {
  return invoke('save_driver_ui_output', { req });
}

/**
 * 指定ドライバタイプの登録UI実行ファイルが利用可能かチェックする
 * （既存ドライバ設定がなくても確認できる）
 */
export async function checkDriverUiAvailable(
  driverType: string,
  driverUiBaseDir?: string | null,
): Promise<boolean> {
  return invoke('check_driver_ui_available', {
    driverType,
    driverUiBaseDir: driverUiBaseDir ?? null,
  });
}
