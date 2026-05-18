/* eslint-env browser */

export function bindTauriInvoke(win = globalThis) {
  return win.__TAURI_INTERNALS__?.invoke?.bind(win.__TAURI_INTERNALS__);
}

export function invokeWithGuard(tauriInvoke, cmd, args = {}) {
  if (!tauriInvoke) {
    return Promise.reject(new Error('Tauri runtime is not available in static preview'));
  }
  return tauriInvoke(cmd, args);
}

export function invokeDirect(cmd, args = {}, win = globalThis) {
  return win.__TAURI_INTERNALS__.invoke(cmd, args);
}

export function formatError(error) {
  if (!error) return '不明なエラーが発生しました';
  if (typeof error === 'string') return error;
  if (typeof error === 'object') {
    if (typeof error.error === 'string' && error.error) return error.error;
    if (typeof error.message === 'string' && error.message) return error.message;
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }
  return String(error);
}

export function normalizeId(raw) {
  return String(raw || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

export function clearMessageElements(elements) {
  elements.forEach((element) => {
    if (element) {
      element.textContent = '';
    }
  });
}
