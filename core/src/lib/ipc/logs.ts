import { ipcInvoke } from './_invoke';

export async function listAppLogs(limit = 500): Promise<string[]> {
  return ipcInvoke<string[]>('list_app_logs', { limit });
}

export async function clearAppLogs(): Promise<void> {
  await ipcInvoke('clear_app_logs');
}
