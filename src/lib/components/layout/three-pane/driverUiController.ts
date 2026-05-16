import type { DriverDto, LaunchDriverUiResponse } from '$lib/ipc';
import { canUseDriverUiForDriver } from './driverUiFlow';
import { runOpenDriverUiForDriverFlow } from './driverUiActions';

export type CreateDriverUiControllerDeps = {
  getDriverUiPolling: () => boolean;
  getDriverUiBaseDirSaved: () => string | null;
  launchDriverUiApi: (req: {
    driver_id: string;
    driver_ui_base_dir?: string | null;
  }) => Promise<LaunchDriverUiResponse>;
  setMessage: (message: string) => void;
  monitorAndImport: (result: LaunchDriverUiResponse) => void;
  notify: (message: string) => void;
  getDrivers: () => DriverDto[];
  getDriverUiAvailableByType: () => Record<string, boolean>;
};

export type DriverUiController = {
  openForDriver: (driverId: string, actionLabel: '新規' | '編集') => Promise<void>;
  canUseDriverUi: (driverId: string) => boolean;
};

export function createDriverUiController(
  deps: CreateDriverUiControllerDeps,
): DriverUiController {
  async function openForDriver(driverId: string, actionLabel: '新規' | '編集') {
    await runOpenDriverUiForDriverFlow({
      driverId,
      actionLabel,
      driverUiPolling: deps.getDriverUiPolling(),
      driverUiBaseDirSaved: deps.getDriverUiBaseDirSaved(),
      launchDriverUiApi: deps.launchDriverUiApi,
      setMessage: deps.setMessage,
      monitorAndImport: deps.monitorAndImport,
      notify: deps.notify,
    });
  }

  function canUseDriverUi(driverId: string): boolean {
    return canUseDriverUiForDriver(
      driverId,
      deps.getDrivers(),
      deps.getDriverUiAvailableByType(),
    );
  }

  return {
    openForDriver,
    canUseDriverUi,
  };
}