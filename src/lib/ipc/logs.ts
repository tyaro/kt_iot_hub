import { invoke } from '@tauri-apps/api/core';

export async function listAppLogs(limit = 500): Promise<string[]> {
  return invoke<string[]>('list_app_logs', { limit });
}

export async function clearAppLogs(): Promise<void> {
  await invoke('clear_app_logs');
}
