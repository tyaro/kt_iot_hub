export type CreateDriverUiSettingsControllerDeps = {
  getInput: () => string;
  setInput: (value: string) => void;
  getSaved: () => string | null;
  setSaved: (value: string | null) => void;
  setMessage: (message: string) => void;
  normalize: (value: string) => string | null;
  saveToStorage: (value: string | null) => void;
  pickFolder: (defaultPath: string | null) => Promise<string | null>;
  extractErrorMessage: (error: unknown, fallback: string) => string;
  notify: (message: string) => void;
};

export type DriverUiSettingsController = {
  save: () => void;
  clear: () => void;
  pick: () => Promise<void>;
  setInputValue: (value: string) => void;
};

export function createDriverUiSettingsController(
  deps: CreateDriverUiSettingsControllerDeps,
): DriverUiSettingsController {
  function save() {
    try {
      const normalized = deps.normalize(deps.getInput());
      deps.saveToStorage(normalized);
      deps.setSaved(normalized);
      deps.setMessage(
        normalized
          ? `ドライバUI設置ベースパスを保存しました: ${normalized}`
          : 'ドライバUI設置ベースパス設定をクリアしました。',
      );
    } catch (error) {
      const message = deps.extractErrorMessage(error, '設定保存に失敗しました');
      deps.setMessage(message);
      deps.notify(message);
    }
  }

  function clear() {
    deps.setInput('');
    save();
  }

  async function pick() {
    try {
      const selected = await deps.pickFolder(deps.getSaved());
      if (!selected) {
        return;
      }

      deps.setInput(selected);
      deps.setMessage(`フォルダを選択しました: ${selected}`);
    } catch (error) {
      const message = deps.extractErrorMessage(error, 'フォルダ選択に失敗しました');
      deps.setMessage(message);
      deps.notify(message);
    }
  }

  function setInputValue(value: string) {
    deps.setInput(value);
  }

  return {
    save,
    clear,
    pick,
    setInputValue,
  };
}