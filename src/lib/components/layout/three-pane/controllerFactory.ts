import type {
  DriverDto,
  ImportDriverUiResultResponse,
  LaunchDriverUiResponse,
  RuntimeStatusDto,
  ScanGroupDto,
  TagDto,
} from '$lib/ipc';
import type { SelectionState } from './handlers';
import { createDeletionController } from './deletionController';
import { createDriverUiController } from './driverUiController';
import { createDriverUiPollingController } from './driverUiPollingController';
import { createDriverUiSettingsController } from './driverUiSettingsController';
import { createRuntimeController } from './runtimeController';
import { createSelectionController } from './selectionController';
import { createTagUiController } from './tagUiController';

export type CreateThreePaneControllersDeps = {
  common: {
    notify: (message: string) => void;
    extractErrorMessage: (error: unknown, fallback: string) => string;
    confirmAction: (message: string) => boolean;
  };
  data: {
    getDrivers: () => DriverDto[];
    getScanGroups: () => ScanGroupDto[];
    reloadTags: () => Promise<void>;
    reloadDrivers: () => Promise<void>;
    reloadScanGroups: () => Promise<void>;
    reloadTagManagementData: () => Promise<void>;
  };
  selectionState: {
    getSelectionState: () => SelectionState;
    setSelectionState: (state: SelectionState) => void;
    setSelectedDriver: (driver: DriverDto | null) => void;
    setSelectedTag: (tag: TagDto) => void;
    clearSelectedTag: () => void;
    clearSelectedDriver: () => void;
    clearSelectedScanGroup: () => void;
    getSelectedTag: () => TagDto | null;
    getSelectedDriver: () => DriverDto | null;
    getSelectedScanGroup: () => ScanGroupDto | null;
  };
  runtime: {
    isRuntimeBusy: () => boolean;
    setRuntimeBusy: (busy: boolean) => void;
    setRuntimeStatus: (status: RuntimeStatusDto) => void;
    setDashboardMessage: (message: string) => void;
  };
  driverUi: {
    ensureDriverUiNotBusy: () => boolean;
    setTagActionMessage: (message: string) => void;
    setDriverUiPolling: (polling: boolean) => void;
    getDriverUiPolling: () => boolean;
    getDriverUiBaseDirSaved: () => string | null;
    getDriverUiAvailableByType: () => Record<string, boolean>;
    checkDriverUiResultApi: (req: { output_json_path: string }) => Promise<{ ready: boolean }>;
    importDriverUiResultApi: (req: {
      session_id: string;
      driver_id?: string | null;
      output_json_path: string;
    }) => Promise<ImportDriverUiResultResponse>;
    launchDriverUiForDriverApi: (req: {
      driver_id: string;
      driver_ui_base_dir?: string | null;
    }) => Promise<LaunchDriverUiResponse>;
    launchDriverUiForTypeApi: (req: {
      driver_type: string;
      driver_ui_base_dir?: string | null;
    }) => Promise<LaunchDriverUiResponse>;
  };
  tagFlow: {
    getDriversCount: () => number;
    isDriversLoading: () => boolean;
    setDriverPickerOpen: (open: boolean) => void;
    setDriverTypePickerOpen: (open: boolean) => void;
  };
  deletion: {
    getDeletingTag: () => boolean;
    setDeletingTag: (value: boolean) => void;
    deleteTagApi: (tagId: string) => Promise<void>;
    deleteDriverApi: (driverId: string) => Promise<void>;
  };
  settings: {
    getDriverUiBaseDirInput: () => string;
    setDriverUiBaseDirInput: (value: string) => void;
    setDriverUiBaseDirSaved: (value: string | null) => void;
    setSettingsMessage: (message: string) => void;
    normalizeDriverUiBaseDir: (value: string) => string | null;
    saveDriverUiBaseDirToStorage: (value: string | null) => void;
    pickFolder: (defaultPath: string | null) => Promise<string | null>;
  };
};

export function createThreePaneControllers(deps: CreateThreePaneControllersDeps) {
  const { common, data, selectionState, runtime, driverUi, tagFlow, deletion, settings } = deps;

  const driverUiPollingController = createDriverUiPollingController({
    setPolling: driverUi.setDriverUiPolling,
    setMessage: driverUi.setTagActionMessage,
    checkReady: async (outputJsonPath) => {
      const check = await driverUi.checkDriverUiResultApi({ output_json_path: outputJsonPath });
      return check.ready;
    },
    importResult: driverUi.importDriverUiResultApi,
    onImported: async (imported) => {
      await data.reloadTagManagementData();
      selectionState.setSelectedDriver(
        data.getDrivers().find((item) => item.id === imported.driver_id) ?? null,
      );
      selectionState.clearSelectedScanGroup();
      selectionState.clearSelectedTag();
      driverUi.setTagActionMessage(
        `取込完了: ${imported.imported_tag_count}件のタグ / ${imported.imported_scan_group_count}件のスキャングループを反映しました`,
      );
    },
    notify: common.notify,
  });

  const runtimeController = createRuntimeController({
    isBusy: runtime.isRuntimeBusy,
    setBusy: runtime.setRuntimeBusy,
    setStatus: runtime.setRuntimeStatus,
    setMessage: runtime.setDashboardMessage,
    notify: common.notify,
    extractErrorMessage: common.extractErrorMessage,
  });

  const driverUiController = createDriverUiController({
    getDriverUiPolling: driverUi.getDriverUiPolling,
    getDriverUiBaseDirSaved: driverUi.getDriverUiBaseDirSaved,
    launchDriverUiApi: driverUi.launchDriverUiForDriverApi,
    setMessage: driverUi.setTagActionMessage,
    monitorAndImport: (result) => {
      void driverUiPollingController.monitorAndImport(result);
    },
    notify: common.notify,
    getDrivers: data.getDrivers,
    getDriverUiAvailableByType: driverUi.getDriverUiAvailableByType,
  });

  const selectionController = createSelectionController({
    getState: selectionState.getSelectionState,
    setState: selectionState.setSelectionState,
    getDrivers: data.getDrivers,
    getScanGroups: data.getScanGroups,
    reloadTags: data.reloadTags,
  });

  const tagUiController = createTagUiController({
    ensureDriverUiNotBusy: driverUi.ensureDriverUiNotBusy,
    getDriversCount: tagFlow.getDriversCount,
    isDriversLoading: tagFlow.isDriversLoading,
    reloadDrivers: data.reloadDrivers,
    setDriverPickerOpen: tagFlow.setDriverPickerOpen,
    setDriverTypePickerOpen: tagFlow.setDriverTypePickerOpen,
    canUseDriverUi: driverUiController.canUseDriverUi,
    setSelectedTag: selectionState.setSelectedTag,
    openDriverUiForDriver: (driverId: string, actionLabel: '新規' | '編集', editingTagId?: string) => {
      void driverUiController.openForDriver(driverId, actionLabel, editingTagId);
    },
    openManualTagEditor: selectionController.openManualTagEditor,
    getDriverUiBaseDirSaved: driverUi.getDriverUiBaseDirSaved,
    launchDriverUiApi: driverUi.launchDriverUiForTypeApi,
    setMessage: driverUi.setTagActionMessage,
    monitorAndImport: (result) => {
      void driverUiPollingController.monitorAndImport(result);
    },
    extractErrorMessage: common.extractErrorMessage,
    notify: common.notify,
  });

  const deletionController = createDeletionController({
    getDeletingTag: deletion.getDeletingTag,
    setDeletingTag: deletion.setDeletingTag,
    getDriverUiPolling: driverUi.getDriverUiPolling,
    setMessage: driverUi.setTagActionMessage,
    confirmAction: common.confirmAction,
    deleteTagApi: deletion.deleteTagApi,
    deleteDriverApi: deletion.deleteDriverApi,
    reloadTags: data.reloadTags,
    reloadDrivers: data.reloadDrivers,
    reloadScanGroups: data.reloadScanGroups,
    getSelectedTag: selectionState.getSelectedTag,
    getSelectedDriver: selectionState.getSelectedDriver,
    getSelectedScanGroup: selectionState.getSelectedScanGroup,
    clearSelectedTag: selectionState.clearSelectedTag,
    clearSelectedDriver: selectionState.clearSelectedDriver,
    clearSelectedScanGroup: selectionState.clearSelectedScanGroup,
    extractErrorMessage: common.extractErrorMessage,
    notify: common.notify,
  });

  const driverUiSettingsController = createDriverUiSettingsController({
    getInput: settings.getDriverUiBaseDirInput,
    setInput: settings.setDriverUiBaseDirInput,
    getSaved: driverUi.getDriverUiBaseDirSaved,
    setSaved: settings.setDriverUiBaseDirSaved,
    setMessage: settings.setSettingsMessage,
    normalize: settings.normalizeDriverUiBaseDir,
    saveToStorage: settings.saveDriverUiBaseDirToStorage,
    pickFolder: settings.pickFolder,
    extractErrorMessage: common.extractErrorMessage,
    notify: common.notify,
  });

  return {
    driverUiPollingController,
    runtimeController,
    driverUiController,
    selectionController,
    tagUiController,
    deletionController,
    driverUiSettingsController,
  };
}