import type { CreateThreePaneControllersDeps } from '../controllerFactory';
import type { DriverDto, RuntimeStatusDto, ScanGroupDto, TagDto } from '$lib/ipc/index';
import type { SelectionState } from '../handlers';

type BuildControllerDepsParams = {
  notify: (message: string) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  confirmAction: (message: string) => Promise<boolean>;
  getDrivers: () => DriverDto[];
  getScanGroups: () => ScanGroupDto[];
  reloadTags: () => Promise<void>;
  reloadDrivers: () => Promise<void>;
  reloadScanGroups: () => Promise<void>;
  reloadAllRegistry: () => Promise<void>;
  reloadTagManagementData: () => Promise<void>;
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
  isRuntimeBusy: () => boolean;
  setRuntimeBusy: (busy: boolean) => void;
  setRuntimeStatus: (status: RuntimeStatusDto) => void;
  setDashboardMessage: (message: string) => void;
  ensureDriverUiNotBusy: () => boolean;
  setTagActionMessage: (message: string) => void;
  setDriverUiPolling: (polling: boolean) => void;
  getDriverUiPolling: () => boolean;
  getDriverUiBaseDirSaved: () => string | null;
  getDriverUiAvailableByType: () => Record<string, boolean>;
  checkDriverUiResultApi: CreateThreePaneControllersDeps['driverUi']['checkDriverUiResultApi'];
  importDriverUiResultApi: CreateThreePaneControllersDeps['driverUi']['importDriverUiResultApi'];
  launchDriverUiForDriverApi: CreateThreePaneControllersDeps['driverUi']['launchDriverUiForDriverApi'];
  launchDriverUiForTypeApi: CreateThreePaneControllersDeps['driverUi']['launchDriverUiForTypeApi'];
  getDriversCount: () => number;
  isDriversLoading: () => boolean;
  setDriverPickerOpen: (open: boolean) => void;
  setDriverTypePickerOpen: (open: boolean) => void;
  getDeletingTag: () => boolean;
  setDeletingTag: (value: boolean) => void;
  deleteTagApi: (tagId: string) => Promise<void>;
  deleteDriverApi: (driverId: string) => Promise<void>;
  getDriverUiBaseDirInput: () => string;
  setDriverUiBaseDirInput: (value: string) => void;
  setDriverUiBaseDirSaved: (value: string | null) => void;
  setSettingsMessage: (message: string) => void;
  normalizeDriverUiBaseDir: (value: string) => string | null;
  saveDriverUiBaseDirToStorage: (value: string | null) => void;
  pickFolder: (defaultPath: string | null) => Promise<string | null>;
};

export function buildThreePaneControllerDeps(
  params: BuildControllerDepsParams,
): CreateThreePaneControllersDeps {
  return {
    common: {
      notify: params.notify,
      extractErrorMessage: params.extractErrorMessage,
      confirmAction: params.confirmAction,
    },
    data: {
      getDrivers: params.getDrivers,
      getScanGroups: params.getScanGroups,
      reloadTags: params.reloadTags,
      reloadDrivers: params.reloadDrivers,
      reloadScanGroups: params.reloadScanGroups,
      reloadAllRegistry: params.reloadAllRegistry,
      reloadTagManagementData: params.reloadTagManagementData,
    },
    selectionState: {
      getSelectionState: params.getSelectionState,
      setSelectionState: params.setSelectionState,
      setSelectedDriver: params.setSelectedDriver,
      setSelectedTag: params.setSelectedTag,
      clearSelectedTag: params.clearSelectedTag,
      clearSelectedDriver: params.clearSelectedDriver,
      clearSelectedScanGroup: params.clearSelectedScanGroup,
      getSelectedTag: params.getSelectedTag,
      getSelectedDriver: params.getSelectedDriver,
      getSelectedScanGroup: params.getSelectedScanGroup,
    },
    runtime: {
      isRuntimeBusy: params.isRuntimeBusy,
      setRuntimeBusy: params.setRuntimeBusy,
      setRuntimeStatus: params.setRuntimeStatus,
      setDashboardMessage: params.setDashboardMessage,
    },
    driverUi: {
      ensureDriverUiNotBusy: params.ensureDriverUiNotBusy,
      setTagActionMessage: params.setTagActionMessage,
      setDriverUiPolling: params.setDriverUiPolling,
      getDriverUiPolling: params.getDriverUiPolling,
      getDriverUiBaseDirSaved: params.getDriverUiBaseDirSaved,
      getDriverUiAvailableByType: params.getDriverUiAvailableByType,
      checkDriverUiResultApi: params.checkDriverUiResultApi,
      importDriverUiResultApi: params.importDriverUiResultApi,
      launchDriverUiForDriverApi: params.launchDriverUiForDriverApi,
      launchDriverUiForTypeApi: params.launchDriverUiForTypeApi,
    },
    tagFlow: {
      getDriversCount: params.getDriversCount,
      isDriversLoading: params.isDriversLoading,
      setDriverPickerOpen: params.setDriverPickerOpen,
      setDriverTypePickerOpen: params.setDriverTypePickerOpen,
    },
    deletion: {
      getDeletingTag: params.getDeletingTag,
      setDeletingTag: params.setDeletingTag,
      deleteTagApi: params.deleteTagApi,
      deleteDriverApi: params.deleteDriverApi,
    },
    settings: {
      getDriverUiBaseDirInput: params.getDriverUiBaseDirInput,
      setDriverUiBaseDirInput: params.setDriverUiBaseDirInput,
      setDriverUiBaseDirSaved: params.setDriverUiBaseDirSaved,
      setSettingsMessage: params.setSettingsMessage,
      normalizeDriverUiBaseDir: params.normalizeDriverUiBaseDir,
      saveDriverUiBaseDirToStorage: params.saveDriverUiBaseDirToStorage,
      pickFolder: params.pickFolder,
    },
  };
}
