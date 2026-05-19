import type { RuntimeStatusDto } from '$lib/ipc';

export type PageId = 'dashboard' | 'tags' | 'publishers' | 'logs' | 'settings';

export const pages: Array<{ id: PageId; label: string; icon: string }> = [
  { id: 'dashboard', label: 'ダッシュボード', icon: '📊' },
  { id: 'tags', label: 'タグ管理', icon: '🏷️' },
  { id: 'publishers', label: 'パブリッシャ', icon: '📤' },
  { id: 'logs', label: 'ログ', icon: '📋' },
  { id: 'settings', label: '設定', icon: '🔧' },
];

export const defaultRuntimeStatus: RuntimeStatusDto = {
  drivers_running: false,
  publishers_running: false,
  grpc_running: false,
  last_error: null,
};

export function isPageId(value: string): value is PageId {
  return pages.some((page) => page.id === value);
}