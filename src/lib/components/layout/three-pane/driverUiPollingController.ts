import type {
  ImportDriverUiResultResponse,
  LaunchDriverUiResponse,
} from '$lib/ipc';
import { monitorDriverUiImport } from './driverUiFlow';
import { wait } from './helpers';

export type DriverUiPollingControllerDeps = {
  setPolling: (polling: boolean) => void;
  setMessage: (message: string) => void;
  checkReady: (
    outputJsonPath: string,
    sessionId: string,
  ) => Promise<{ ready: boolean; process_active: boolean }>;
  importResult: (req: {
    session_id: string;
    driver_id?: string | null;
    output_json_path: string;
  }) => Promise<ImportDriverUiResultResponse>;
  onImported: (imported: ImportDriverUiResultResponse) => Promise<void>;
  notify: (message: string) => void;
};

export type DriverUiPollingController = {
  cancel: () => void;
  monitorAndImport: (result: LaunchDriverUiResponse) => Promise<void>;
};

export function createDriverUiPollingController(
  deps: DriverUiPollingControllerDeps,
): DriverUiPollingController {
  let pollingToken = 0;

  function cancel() {
    pollingToken += 1;
    deps.setPolling(false);
  }

  async function monitorAndImport(result: LaunchDriverUiResponse) {
    const token = pollingToken + 1;
    pollingToken = token;

    await monitorDriverUiImport({
      result,
      token,
      isTokenValid: (currentToken) => pollingToken === currentToken,
      setMessage: deps.setMessage,
      setPolling: deps.setPolling,
      checkReady: deps.checkReady,
      importResult: deps.importResult,
      onImported: deps.onImported,
      waitFn: wait,
      notify: deps.notify,
    });
  }

  return {
    cancel,
    monitorAndImport,
  };
}