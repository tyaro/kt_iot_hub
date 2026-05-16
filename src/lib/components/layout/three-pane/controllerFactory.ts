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
  ensureDriverUiNotBusy: () => boolean;
  setTagActionMessage: (message: string) => void;
  reloadTagManagementData: () => Promise<void>;
  setSelectedDriver: (driver: DriverDto | null) => void;
  clearSelectedScanGroup: () => void;
  clearSelectedTag: () => void;
  checkDriverUiResultApi: (req: { output_json_path: string }) => Promise<{ ready: boolean }>;
  importDriverUiResultApi: (req: {
    session_id: string;
    driver_id?: string | null;
    output_json_path: string;
  }) => Promise<ImportDriverUiResultResponse>;
  notify: (message: string) => void;
  setDriverUiPolling: (polling: boolean) => void;
  isRuntimeBusy: () => boolean;
  setRuntimeBusy: (busy: boolean) => void;
  setRuntimeStatus: (status: RuntimeStatusDto) => void;
  setDashboardMessage: (message: string) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  getDriverUiPolling: () => boolean;
  getDriverUiBaseDirSaved: () => string | null;
  launchDriverUiForDriverApi: (req: {
    driver_id: string;
    driver_ui_base_dir?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  launchDriverUiForTypeApi: (req: {
    driver_type: string;
    driver_ui_base_dir?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  getDrivers: () => DriverDto[];
  getDriverUiAvailableByType: () => Record<string, boolean>;
  getSelectionState: () => SelectionState;
  setSelectionState: (state: SelectionState) => void;
  getScanGroups: () => ScanGroupDto[];
  reloadTags: () => Promise<void>;
  getDriversCount: () => number;
  isDriversLoading: () => boolean;
  reloadDrivers: () => Promise<void>;
  setDriverPickerOpen: (open: boolean) => void;
  setDriverTypePickerOpen: (open: boolean) => void;
  setSelectedTag: (tag: TagDto) => void;
  getDeletingTag: () => boolean;
  setDeletingTag: (value: boolean) => void;
  confirmAction: (message: string) => boolean;
  deleteTagApi: (tagId: string) => Promise<void>;
  deleteDriverApi: (driverId: string) => Promise<void>;
  reloadScanGroups: () => Promise<void>;
  getSelectedTag: () => TagDto | null;
  getSelectedDriver: () => DriverDto | null;
  getSelectedScanGroup: () => ScanGroupDto | null;
  clearSelectedDriver: () => void;
  getDriverUiBaseDirInput: () => string;
  setDriverUiBaseDirInput: (value: string) => void;
  setDriverUiBaseDirSaved: (value: string | null) => void;
  setSettingsMessage: (message: string) => void;
  normalizeDriverUiBaseDir: (value: string) => string | null;
  saveDriverUiBaseDirToStorage: (value: string | null) => void;
  pickFolder: (defaultPath: string | null) => Promise<string | null>;
};

export function createThreePaneControllers(deps: CreateThreePaneControllersDeps) {
  const driverUiPollingController = createDriverUiPollingController({
    setPolling: deps.setDriverUiPolling,
    setMessage: deps.setTagActionMessage,
    checkReady: async (outputJsonPath) => {
      const check = await deps.checkDriverUiResultApi({ output_json_path: outputJsonPath });
      return check.ready;
    },
    importResult: deps.importDriverUiResultApi,
    onImported: async (imported) => {
      await deps.reloadTagManagementData();
      deps.setSelectedDriver(
        deps.getDrivers().find((item) => item.id === imported.driver_id) ?? null,
      );
      deps.clearSelectedScanGroup();
      deps.clearSelectedTag();
      deps.setTagActionMessage(
        `取込完了: ${imported.imported_tag_count}件のタグ / ${imported.imported_scan_group_count}件のスキャングループを反映しました`,
      );
    },
    notify: deps.notify,
  });

  const runtimeController = createRuntimeController({
    isBusy: deps.isRuntimeBusy,
    setBusy: deps.setRuntimeBusy,
    setStatus: deps.setRuntimeStatus,
    setMessage: deps.setDashboardMessage,
    notify: deps.notify,
    extractErrorMessage: deps.extractErrorMessage,
  });

  const driverUiController = createDriverUiController({
    getDriverUiPolling: deps.getDriverUiPolling,
    getDriverUiBaseDirSaved: deps.getDriverUiBaseDirSaved,
    launchDriverUiApi: deps.launchDriverUiForDriverApi,
    setMessage: deps.setTagActionMessage,
    monitorAndImport: (result) => {
      void driverUiPollingController.monitorAndImport(result);
    },
    notify: deps.notify,
    getDrivers: deps.getDrivers,
    getDriverUiAvailableByType: deps.getDriverUiAvailableByType,
  });

  const selectionController = createSelectionController({
    getState: deps.getSelectionState,
    setState: deps.setSelectionState,
    getDrivers: deps.getDrivers,
    getScanGroups: deps.getScanGroups,
    reloadTags: deps.reloadTags,
  });

  const tagUiController = createTagUiController({
    ensureDriverUiNotBusy: deps.ensureDriverUiNotBusy,
    getDriversCount: deps.getDriversCount,
    isDriversLoading: deps.isDriversLoading,
    reloadDrivers: deps.reloadDrivers,
    setDriverPickerOpen: deps.setDriverPickerOpen,
    setDriverTypePickerOpen: deps.setDriverTypePickerOpen,
    canUseDriverUi: driverUiController.canUseDriverUi,
    setSelectedTag: deps.setSelectedTag,
    openDriverUiForDriver: (driverId, actionLabel) => {
      void driverUiController.openForDriver(driverId, actionLabel);
    },
    openManualTagEditor: selectionController.openManualTagEditor,
    getDriverUiBaseDirSaved: deps.getDriverUiBaseDirSaved,
    launchDriverUiApi: deps.launchDriverUiForTypeApi,
    setMessage: deps.setTagActionMessage,
    monitorAndImport: (result) => {
      void driverUiPollingController.monitorAndImport(result);
    },
    extractErrorMessage: deps.extractErrorMessage,
    notify: deps.notify,
  });

  const deletionController = createDeletionController({
    getDeletingTag: deps.getDeletingTag,
    setDeletingTag: deps.setDeletingTag,
    getDriverUiPolling: deps.getDriverUiPolling,
    setMessage: deps.setTagActionMessage,
    confirmAction: deps.confirmAction,
    deleteTagApi: deps.deleteTagApi,
    deleteDriverApi: deps.deleteDriverApi,
    reloadTags: deps.reloadTags,
    reloadDrivers: deps.reloadDrivers,
    reloadScanGroups: deps.reloadScanGroups,
    getSelectedTag: deps.getSelectedTag,
    getSelectedDriver: deps.getSelectedDriver,
    getSelectedScanGroup: deps.getSelectedScanGroup,
    clearSelectedTag: deps.clearSelectedTag,
    clearSelectedDriver: deps.clearSelectedDriver,
    clearSelectedScanGroup: deps.clearSelectedScanGroup,
    extractErrorMessage: deps.extractErrorMessage,
    notify: deps.notify,
  });

  const driverUiSettingsController = createDriverUiSettingsController({
    getInput: deps.getDriverUiBaseDirInput,
    setInput: deps.setDriverUiBaseDirInput,
    getSaved: deps.getDriverUiBaseDirSaved,
    setSaved: deps.setDriverUiBaseDirSaved,
    setMessage: deps.setSettingsMessage,
    normalize: deps.normalizeDriverUiBaseDir,
    saveToStorage: deps.saveDriverUiBaseDirToStorage,
    pickFolder: deps.pickFolder,
    extractErrorMessage: deps.extractErrorMessage,
    notify: deps.notify,
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