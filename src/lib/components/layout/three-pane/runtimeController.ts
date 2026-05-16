import type { RuntimeStatusDto } from '$lib/ipc';
import {
  refreshRuntimeStatus as refreshRuntimeStatusFlow,
  runRuntimeServiceAction,
} from './handlers';

export type RuntimeControllerDeps = {
  isBusy: () => boolean;
  setBusy: (busy: boolean) => void;
  setStatus: (status: RuntimeStatusDto) => void;
  setMessage: (message: string) => void;
  notify: (message: string) => void;
  extractErrorMessage: (error: unknown, fallback: string) => string;
};

export type RuntimeController = {
  refreshStatus: (getRuntimeStatus: () => Promise<RuntimeStatusDto>) => Promise<void>;
  runAction: (
    execute: () => Promise<RuntimeStatusDto>,
    successMessage: string,
    failureMessage: string,
  ) => Promise<void>;
};

export function createRuntimeController(deps: RuntimeControllerDeps): RuntimeController {
  async function refreshStatus(getRuntimeStatus: () => Promise<RuntimeStatusDto>) {
    const result = await refreshRuntimeStatusFlow(getRuntimeStatus);
    if (result.status) {
      deps.setStatus(result.status);
    }
    if (result.message) {
      deps.setMessage(result.message);
    }
  }

  async function runAction(
    execute: () => Promise<RuntimeStatusDto>,
    successMessage: string,
    failureMessage: string,
  ) {
    if (deps.isBusy()) {
      return;
    }

    deps.setBusy(true);
    const result = await runRuntimeServiceAction(
      execute,
      successMessage,
      deps.extractErrorMessage,
      failureMessage,
    );

    if (result.status) {
      deps.setStatus(result.status);
    }

    deps.setMessage(result.message);
    if (!result.status && result.message) {
      deps.notify(result.message);
    }
    deps.setBusy(false);
  }

  return {
    refreshStatus,
    runAction,
  };
}