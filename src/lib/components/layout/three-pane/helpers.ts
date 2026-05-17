const DRIVER_UI_BASE_DIR_KEY = 'kt_iot_hub.driverUiBaseDir';
export const DRIVER_UI_IMPORT_BUSY_MESSAGE = '前回のドライバUI結果を取込中です。完了までお待ちください。';

export function normalizeDriverUiBaseDir(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function loadDriverUiBaseDirFromStorage(): string | null {
  if (typeof globalThis.localStorage === 'undefined') {
    return null;
  }
  const raw = globalThis.localStorage.getItem(DRIVER_UI_BASE_DIR_KEY);
  return raw && raw.trim().length > 0 ? raw.trim() : null;
}

export function saveDriverUiBaseDirToStorage(value: string | null): void {
  if (typeof globalThis.localStorage === 'undefined') {
    return;
  }

  if (value) {
    globalThis.localStorage.setItem(DRIVER_UI_BASE_DIR_KEY, value);
  } else {
    globalThis.localStorage.removeItem(DRIVER_UI_BASE_DIR_KEY);
  }
}

export function notify(message: string): void {
  if (typeof globalThis.alert === 'function') {
    globalThis.alert(message);
  }
}

export function extractErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  if (typeof error === 'string' && error.length > 0) {
    return error;
  }
  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>;
    const nested = record.error;
    if (typeof nested === 'string' && nested.length > 0) {
      return nested;
    }
    const message = record.message;
    if (typeof message === 'string' && message.length > 0) {
      return message;
    }
  }
  return fallback;
}

export async function wait(ms: number): Promise<void> {
  await new Promise((resolve) => {
    if (typeof globalThis.setTimeout === 'function') {
      globalThis.setTimeout(resolve, ms);
      return;
    }
    resolve(undefined);
  });
}
