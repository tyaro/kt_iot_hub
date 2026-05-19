import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc/index';

export type DriverSelectionSnapshot = {
  selectedDriver: DriverDto | null;
  selectedTag: TagDto | null;
  selectedScanGroup: ScanGroupDto | null;
};

export function ensureDriverUiNotBusy(
  driverUiPolling: boolean,
  busyMessage: string,
  setTagActionMessage: (message: string) => void,
): boolean {
  if (!driverUiPolling) {
    return true;
  }
  setTagActionMessage(busyMessage);
  return false;
}

export function syncSelectionAfterDriverSaved(
  drivers: DriverDto[],
  driverId: string | undefined,
  selectedTag: TagDto | null,
  selectedScanGroup: ScanGroupDto | null,
): DriverSelectionSnapshot {
  if (!driverId) {
    return {
      selectedDriver: null,
      selectedTag,
      selectedScanGroup,
    };
  }

  const selectedDriver = drivers.find((item) => item.id === driverId) ?? null;
  const nextSelectedTag = selectedTag?.driver_id === driverId ? selectedTag : null;
  const nextSelectedScanGroup = selectedScanGroup?.driver_id === driverId ? selectedScanGroup : null;

  return {
    selectedDriver,
    selectedTag: nextSelectedTag,
    selectedScanGroup: nextSelectedScanGroup,
  };
}
