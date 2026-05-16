import type { LaunchDriverUiResponse, TagDto } from '$lib/ipc';
import {
  ensureDriversLoaded,
  runOpenDriverUiForTypeFlow,
  runRequestEditTagFlow,
  runRequestNewTagForDriverFlow,
} from './driverUiActions';

export type CreateTagUiControllerDeps = {
  ensureDriverUiNotBusy: () => boolean;
  getDriversCount: () => number;
  isDriversLoading: () => boolean;
  reloadDrivers: () => Promise<void>;
  setDriverPickerOpen: (open: boolean) => void;
  setDriverTypePickerOpen: (open: boolean) => void;
  canUseDriverUi: (driverId: string) => boolean;
  setSelectedTag: (tag: TagDto) => void;
  openDriverUiForDriver: (driverId: string, actionLabel: '新規' | '編集') => void;
  openManualTagEditor: (driverId: string, mode: 'new' | 'edit', tag?: TagDto | null) => void;
  getDriverUiBaseDirSaved: () => string | null;
  launchDriverUiApi: (req: {
    driver_type: string;
    driver_ui_base_dir?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  setMessage: (message: string) => void;
  monitorAndImport: (result: LaunchDriverUiResponse) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export type TagUiController = {
  requestEditTag: (tag: TagDto) => void;
  requestNewTagForDriver: (driverId: string) => void;
  newTag: () => Promise<void>;
  newDriver: () => Promise<void>;
  closeDriverPicker: () => void;
  closeDriverTypePicker: () => void;
  onDriverPicked: (driverId: string) => void;
  onDriverTypePicked: (driverType: string) => Promise<void>;
};

export function createTagUiController(deps: CreateTagUiControllerDeps): TagUiController {
  function requestEditTag(tag: TagDto) {
    runRequestEditTagFlow(
      tag,
      deps.canUseDriverUi,
      deps.setSelectedTag,
      deps.openDriverUiForDriver,
      deps.openManualTagEditor,
    );
  }

  function requestNewTagForDriver(driverId: string) {
    runRequestNewTagForDriverFlow(
      driverId,
      deps.canUseDriverUi,
      (nextDriverId, actionLabel) => {
        deps.openDriverUiForDriver(nextDriverId, actionLabel);
      },
      (nextDriverId, mode) => {
        deps.openManualTagEditor(nextDriverId, mode);
      },
    );
  }

  async function newTag() {
    if (!deps.ensureDriverUiNotBusy()) {
      return;
    }

    deps.setDriverPickerOpen(true);
    await ensureDriversLoaded(deps.getDriversCount(), deps.isDriversLoading(), deps.reloadDrivers);
  }

  async function newDriver() {
    if (!deps.ensureDriverUiNotBusy()) {
      return;
    }

    await ensureDriversLoaded(deps.getDriversCount(), deps.isDriversLoading(), deps.reloadDrivers);
    deps.setDriverTypePickerOpen(true);
  }

  function closeDriverPicker() {
    deps.setDriverPickerOpen(false);
  }

  function closeDriverTypePicker() {
    deps.setDriverTypePickerOpen(false);
  }

  function onDriverPicked(driverId: string) {
    deps.setDriverPickerOpen(false);
    requestNewTagForDriver(driverId);
  }

  async function onDriverTypePicked(driverType: string) {
    deps.setDriverTypePickerOpen(false);
    await runOpenDriverUiForTypeFlow({
      driverType,
      driverUiBaseDirSaved: deps.getDriverUiBaseDirSaved(),
      launchDriverUiApi: deps.launchDriverUiApi,
      setMessage: deps.setMessage,
      monitorAndImport: deps.monitorAndImport,
      extractErrorMessage: deps.extractErrorMessage,
      notify: deps.notify,
    });
  }

  return {
    requestEditTag,
    requestNewTagForDriver,
    newTag,
    newDriver,
    closeDriverPicker,
    closeDriverTypePicker,
    onDriverPicked,
    onDriverTypePicked,
  };
}