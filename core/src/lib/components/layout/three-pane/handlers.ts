import type { DriverDto, RuntimeStatusDto, ScanGroupDto, TagDto } from '$lib/ipc';

export type SelectionState = {
  selectedTag: TagDto | null;
  selectedDriver: DriverDto | null;
  selectedScanGroup: ScanGroupDto | null;
  tagMode: 'detail' | 'new' | 'edit';
  tagActionMessage: string;
  editorDriverId: string | null;
};

export function resetSelectionForPage(state: SelectionState): void {
  state.selectedTag = null;
  state.selectedScanGroup = null;
  state.selectedDriver = null;
  state.tagMode = 'detail';
  state.tagActionMessage = '';
  state.editorDriverId = null;
}

export function handleTagSelect(
  state: SelectionState,
  tag: TagDto | null,
  drivers: DriverDto[],
  scanGroups: ScanGroupDto[],
): void {
  state.selectedTag = tag;
  if (tag) {
    state.selectedDriver = drivers.find((item) => item.id === tag.driver_id) ?? null;
    state.selectedScanGroup = scanGroups.find((item) => item.id === tag.scan_group_id) ?? null;
  }
  state.tagMode = 'detail';
  state.tagActionMessage = '';
}

export function handleDriverSelect(state: SelectionState, driver: DriverDto | null): void {
  state.selectedDriver = driver;
  state.selectedTag = null;
  state.selectedScanGroup = null;
  state.tagMode = 'detail';
  state.tagActionMessage = '';
}

export function handleScanGroupSelect(
  state: SelectionState,
  scanGroup: ScanGroupDto | null,
  drivers: DriverDto[],
): void {
  state.selectedScanGroup = scanGroup;
  state.selectedTag = null;
  if (scanGroup) {
    state.selectedDriver = drivers.find((item) => item.id === scanGroup.driver_id) ?? null;
  }
  state.tagMode = 'detail';
  state.tagActionMessage = '';
}

export function openManualTagEditor(
  state: SelectionState,
  driverId: string,
  mode: 'new' | 'edit',
  tag?: TagDto | null,
): void {
  state.selectedTag = tag ?? null;
  state.editorDriverId = driverId;
  state.tagMode = mode;
  state.tagActionMessage =
    mode === 'new'
      ? '手動タグ登録モードです。'
      : '外部ドライバUI未設定のため手動編集モードに切り替えました。';
}

export function closeTagEditor(state: SelectionState): void {
  state.tagMode = 'detail';
  if (!state.selectedTag) {
    state.editorDriverId = null;
  }
}

export async function onTagEditorDone(
  state: SelectionState,
  reloadTags: () => Promise<void>,
): Promise<void> {
  await reloadTags();
  state.tagMode = 'detail';
  state.tagActionMessage = 'タグ定義を保存しました。';
}

export function saveDriverUiBaseDir(
  inputValue: string,
  normalize: (value: string) => string | null,
  saveToStorage: (value: string | null) => void,
): { saved: string | null; message: string } {
  const normalized = normalize(inputValue);
  saveToStorage(normalized);
  return {
    saved: normalized,
    message: normalized
      ? `ドライバUI設置ベースパスを保存しました: ${normalized}`
      : 'ドライバUI設置ベースパス設定をクリアしました。',
  };
}

export async function pickDriverUiBaseDir(
  pickFolder: () => Promise<string | null>,
): Promise<{ selected: string | null; message: string | null }> {
  const selected = await pickFolder();
  if (!selected) {
    return { selected: null, message: null };
  }
  return {
    selected,
    message: `フォルダを選択しました: ${selected}`,
  };
}

export async function refreshRuntimeStatus(
  getRuntimeStatus: () => Promise<RuntimeStatusDto>,
): Promise<{ status?: RuntimeStatusDto; message?: string }> {
  try {
    return { status: await getRuntimeStatus() };
  } catch (error) {
    return {
      message: error instanceof Error ? error.message : 'ランタイム状態の取得に失敗しました',
    };
  }
}

export async function runRuntimeServiceAction(
  execute: () => Promise<RuntimeStatusDto>,
  successMessage: string,
  extractErrorMessage: (error: unknown, fallback: string) => string,
  failureMessage: string,
): Promise<{ status?: RuntimeStatusDto; message: string }> {
  try {
    return {
      status: await execute(),
      message: successMessage,
    };
  } catch (error) {
    return {
      message: extractErrorMessage(error, failureMessage),
    };
  }
}
