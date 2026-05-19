import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';
import { runDeleteDriverFlow, runDeleteTagFlow } from './driverUiActions';

export type CreateDeletionControllerDeps = {
  getDeletingTag: () => boolean;
  setDeletingTag: (value: boolean) => void;
  getDriverUiPolling: () => boolean;
  setMessage: (message: string) => void;
  confirmAction: (message: string) => Promise<boolean>;
  deleteTagApi: (tagId: string) => Promise<void>;
  deleteDriverApi: (driverId: string) => Promise<void>;
  reloadTags: () => Promise<void>;
  reloadDrivers: () => Promise<void>;
  reloadScanGroups: () => Promise<void>;
  reloadAllRegistry: () => Promise<void>;
  getSelectedTag: () => TagDto | null;
  getSelectedDriver: () => DriverDto | null;
  getSelectedScanGroup: () => ScanGroupDto | null;
  clearSelectedTag: () => void;
  clearSelectedDriver: () => void;
  clearSelectedScanGroup: () => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export type DeletionController = {
  requestDeleteTag: (tag: TagDto) => Promise<void>;
  requestDeleteDriver: (driverId: string) => Promise<void>;
  onDriverDetailDone: () => Promise<void>;
};

export function createDeletionController(
  deps: CreateDeletionControllerDeps,
): DeletionController {
  async function requestDeleteTag(tag: TagDto) {
    await runDeleteTagFlow({
      tag,
      deletingTag: deps.getDeletingTag(),
      confirmAction: deps.confirmAction,
      setDeletingTag: deps.setDeletingTag,
      deleteTagApi: deps.deleteTagApi,
      reloadTags: deps.reloadTags,
      isSelectedTag: (tagId) => deps.getSelectedTag()?.id === tagId,
      clearSelectedTag: deps.clearSelectedTag,
      setMessage: deps.setMessage,
      extractErrorMessage: deps.extractErrorMessage,
      notify: deps.notify,
    });
  }

  async function requestDeleteDriver(driverId: string) {
    await runDeleteDriverFlow({
      driverId,
      driverUiPolling: deps.getDriverUiPolling(),
      setMessage: deps.setMessage,
      confirmAction: deps.confirmAction,
      deleteDriverApi: deps.deleteDriverApi,
      reloadAllRegistry: deps.reloadAllRegistry,
      selectedDriver: deps.getSelectedDriver(),
      selectedTag: deps.getSelectedTag(),
      selectedScanGroup: deps.getSelectedScanGroup(),
      clearSelectedDriver: deps.clearSelectedDriver,
      clearSelectedTag: deps.clearSelectedTag,
      clearSelectedScanGroup: deps.clearSelectedScanGroup,
      extractErrorMessage: deps.extractErrorMessage,
      notify: deps.notify,
    });
  }

  async function onDriverDetailDone() {
    await deps.reloadDrivers();
    await deps.reloadScanGroups();
    deps.clearSelectedDriver();
  }

  return {
    requestDeleteTag,
    requestDeleteDriver,
    onDriverDetailDone,
  };
}