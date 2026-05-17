/* eslint-env browser */

const browserWindow = globalThis;
const invoke = (cmd, args = {}) => browserWindow.__TAURI_INTERNALS__.invoke(cmd, args);

let launchContext = null;
let scanGroups = [];
let tags = [];
let isEditMode = false;

const el = (id) => browserWindow.document.getElementById(id);

function formatError(error) {
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

function setStep(step) {
  browserWindow.document.querySelectorAll('[data-step-panel]').forEach((panel) => {
    panel.classList.toggle('hidden', Number(panel.dataset.stepPanel) !== step);
  });
}

function collectDriverSettings() {
  return {
    // TODO: ドライバ固有の設定項目を返す
  };
}

function buildPayload() {
  const driverId = el('driverId').value.trim();
  const driverType = launchContext?.driver?.driverType || 'your-driver-type';

  return {
    schemaVersion: 1,
    requestId: launchContext?.requestId || browserWindow.crypto.randomUUID(),
    driver: {
      id: driverId,
      driverType,
      enabled: true,
      settings: collectDriverSettings()
    },
    scanGroups,
    tags
  };
}

async function loadLaunchContext() {
  launchContext = await invoke('get_driver_ui_launch_context');
  isEditMode = Boolean(launchContext?.driver?.driverId);

  if (launchContext?.driver?.driverId) {
    el('driverId').value = launchContext.driver.driverId;
  }

  scanGroups = Array.isArray(launchContext?.context?.scanGroups) ? [...launchContext.context.scanGroups] : [];
  tags = Array.isArray(launchContext?.context?.tags) ? [...launchContext.context.tags] : [];
}

async function saveOutput() {
  try {
    const payload = buildPayload();
    await invoke('save_driver_ui_output', { req: { payload } });
    el('msgOk').textContent = isEditMode ? '既存設定を保存しました。' : '新規設定を保存しました。';
    el('msgErr').textContent = '';
  } catch (error) {
    el('msgOk').textContent = '';
    el('msgErr').textContent = formatError(error);
  }
}

async function closeWindow() {
  await invoke('close_driver_ui_window');
}

browserWindow.addEventListener('DOMContentLoaded', async () => {
  try {
    await loadLaunchContext();
    setStep(1);
    el('saveButton')?.addEventListener('click', saveOutput);
    el('cancelButton')?.addEventListener('click', closeWindow);
  } catch (error) {
    el('msgErr').textContent = formatError(error);
  }
});