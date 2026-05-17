import type {
  DriverDto,
  LaunchDriverUiResponse,
  ScanGroupDto,
  TagDto,
} from '$lib/ipc';
import { DRIVER_UI_IMPORT_BUSY_MESSAGE } from './helpers';

export type DeleteTagFlowDeps = {
  tag: TagDto;
  deletingTag: boolean;
  confirmAction: (message: string) => boolean;
  setDeletingTag: (value: boolean) => void;
  deleteTagApi: (tagId: string) => Promise<void>;
  reloadTags: () => Promise<void>;
  isSelectedTag: (tagId: string) => boolean;
  clearSelectedTag: () => void;
  setMessage: (message: string) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export async function runDeleteTagFlow({
  tag,
  deletingTag,
  confirmAction,
  setDeletingTag,
  deleteTagApi,
  reloadTags,
  isSelectedTag,
  clearSelectedTag,
  setMessage,
  extractErrorMessage,
  notify,
}: DeleteTagFlowDeps): Promise<void> {
  if (deletingTag) {
    return;
  }
  if (!confirmAction(`タグ「${tag.id}」を削除しますか？`)) {
    return;
  }

  setDeletingTag(true);
  try {
    await deleteTagApi(tag.id);
    await reloadTags();
    if (isSelectedTag(tag.id)) {
      clearSelectedTag();
    }
    setMessage(`タグ「${tag.id}」を削除しました`);
  } catch (error) {
    const message = extractErrorMessage(error, 'タグ削除に失敗しました');
    setMessage(message);
    notify(message);
  } finally {
    setDeletingTag(false);
  }
}

export type DeleteDriverFlowDeps = {
  driverId: string;
  driverUiPolling: boolean;
  setMessage: (message: string) => void;
  confirmAction: (message: string) => boolean;
  deleteDriverApi: (driverId: string) => Promise<void>;
  reloadDrivers: () => Promise<void>;
  reloadScanGroups: () => Promise<void>;
  reloadTags: () => Promise<void>;
  selectedDriver: DriverDto | null;
  selectedTag: TagDto | null;
  selectedScanGroup: ScanGroupDto | null;
  clearSelectedDriver: () => void;
  clearSelectedTag: () => void;
  clearSelectedScanGroup: () => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export async function runDeleteDriverFlow({
  driverId,
  driverUiPolling,
  setMessage,
  confirmAction,
  deleteDriverApi,
  reloadDrivers,
  reloadScanGroups,
  reloadTags,
  selectedDriver,
  selectedTag,
  selectedScanGroup,
  clearSelectedDriver,
  clearSelectedTag,
  clearSelectedScanGroup,
  extractErrorMessage,
  notify,
}: DeleteDriverFlowDeps): Promise<void> {
  if (driverUiPolling) {
    setMessage(DRIVER_UI_IMPORT_BUSY_MESSAGE);
    return;
  }

  if (!confirmAction(`接続先「${driverId}」を削除しますか？\n配下のScanグループとタグも削除されます。`)) {
    return;
  }

  try {
    await deleteDriverApi(driverId);
    await reloadDrivers();
    await reloadScanGroups();
    await reloadTags();

    if (selectedDriver?.id === driverId) {
      clearSelectedDriver();
    }
    if (selectedTag?.driver_id === driverId) {
      clearSelectedTag();
    }
    if (selectedScanGroup?.driver_id === driverId) {
      clearSelectedScanGroup();
    }

    setMessage(`接続先「${driverId}」を削除しました。`);
  } catch (error) {
    const message = extractErrorMessage(error, '接続先削除に失敗しました');
    setMessage(message);
    notify(message);
  }
}

export type OpenDriverUiForTypeFlowDeps = {
  driverType: string;
  driverUiBaseDirSaved: string | null;
  launchDriverUiApi: (req: {
    driver_type: string;
    driver_ui_base_dir?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  setMessage: (message: string) => void;
  monitorAndImport: (result: LaunchDriverUiResponse) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export async function runOpenDriverUiForTypeFlow({
  driverType,
  driverUiBaseDirSaved,
  launchDriverUiApi,
  setMessage,
  monitorAndImport,
  extractErrorMessage,
  notify,
}: OpenDriverUiForTypeFlowDeps): Promise<void> {
  try {
    const result = await launchDriverUiApi({
      driver_type: driverType,
      driver_ui_base_dir: driverUiBaseDirSaved,
    });
    setMessage(`${driverType} 用のドライバUIを起動しました。接続先・Scanグループ・タグを外部画面で登録してください。`);
    monitorAndImport(result);
  } catch (error) {
    const message = extractErrorMessage(error, 'ドライバUI起動に失敗しました');
    setMessage(message);
    notify(message);
  }
}

export type OpenDriverUiForDriverFlowDeps = {
  driverId: string;
  actionLabel: '新規' | '編集';
  editingTagId?: string;
  driverUiPolling: boolean;
  driverUiBaseDirSaved: string | null;
  launchDriverUiApi: (req: {
    driver_id: string;
    driver_ui_base_dir?: string | null;
    editing_tag_id?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  setMessage: (message: string) => void;
  monitorAndImport: (result: LaunchDriverUiResponse) => void;
  notify: (message: string) => void;
};

export async function runOpenDriverUiForDriverFlow({
  driverId,
  actionLabel,
  editingTagId,
  driverUiPolling,
  driverUiBaseDirSaved,
  launchDriverUiApi,
  setMessage,
  monitorAndImport,
  notify,
}: OpenDriverUiForDriverFlowDeps): Promise<void> {
  if (driverUiPolling) {
    setMessage(DRIVER_UI_IMPORT_BUSY_MESSAGE);
    return;
  }

  try {
    const result = await launchDriverUiApi({
      driver_id: driverId,
      driver_ui_base_dir: driverUiBaseDirSaved,
      editing_tag_id: editingTagId,
    });
    setMessage(
      `${actionLabel}用ドライバUIを起動しました (driver_id=${result.driver_id}, session_id=${result.session_id})`,
    );
    monitorAndImport(result);
  } catch (error) {
    const message = error instanceof Error ? error.message : 'ドライバUI起動に失敗しました';
    setMessage(message);
    notify(message);
  }
}

export async function ensureDriversLoaded(
  driversCount: number,
  driversLoading: boolean,
  reloadDrivers: () => Promise<void>,
): Promise<void> {
  if (driversCount === 0 && !driversLoading) {
    await reloadDrivers();
  }
}

export function runRequestEditTagFlow(
  tag: TagDto,
  canUseDriverUi: (driverId: string) => boolean,
  setSelectedTag: (tag: TagDto) => void,
  openDriverUi: (driverId: string, actionLabel: '編集', editingTagId?: string) => void,
  openManualTagEditor: (driverId: string, mode: 'edit', tag?: TagDto | null) => void,
): void {
  if (canUseDriverUi(tag.driver_id)) {
    setSelectedTag(tag);
    openDriverUi(tag.driver_id, '編集', tag.id);
    return;
  }
  openManualTagEditor(tag.driver_id, 'edit', tag);
}

export function runRequestNewTagForDriverFlow(
  driverId: string,
  canUseDriverUi: (driverId: string) => boolean,
  openDriverUi: (driverId: string, actionLabel: '新規') => void,
  openManualTagEditor: (driverId: string, mode: 'new') => void,
): void {
  if (canUseDriverUi(driverId)) {
    openDriverUi(driverId, '新規');
    return;
  }
  openManualTagEditor(driverId, 'new');
}

export function runRequestEditDriverFlow(
  driverId: string,
  canUseDriverUi: (driverId: string) => boolean,
  openDriverUi: (driverId: string, actionLabel: '編集') => void,
  notify: (message: string) => void,
): void {
  if (canUseDriverUi(driverId)) {
    openDriverUi(driverId, '編集');
    return;
  }

  notify(`接続先「${driverId}」の外部ドライバUIが見つからないため、編集を開けません。`);
}
