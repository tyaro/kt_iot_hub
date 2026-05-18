import { invoke } from '@tauri-apps/api/core';

export const ipcInvoke = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  invoke<T>(cmd, args);
