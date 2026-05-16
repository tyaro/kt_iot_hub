<script lang="ts">
  let {
    driverUiBaseDirInput,
    driverUiBaseDirSaved,
    settingsMessage,
    onDriverUiBaseDirInput,
    onPickDriverUiBaseDir,
    onSaveDriverUiBaseDir,
    onClearDriverUiBaseDir,
  }: {
    driverUiBaseDirInput: string;
    driverUiBaseDirSaved: string | null;
    settingsMessage: string;
    onDriverUiBaseDirInput: (value: string) => void;
    onPickDriverUiBaseDir: () => void | Promise<void>;
    onSaveDriverUiBaseDir: () => void;
    onClearDriverUiBaseDir: () => void;
  } = $props();
</script>

<div class="content">
  <h2>設定</h2>
  <div class="settings-card">
    <h3>ドライバUI実行ファイル配置</h3>
    <p class="settings-help">
      例: <code>D:\develop\kt_iot_hub</code> または <code>D:\develop\kt_iot_hub\driver-ui</code>
    </p>
    <label>
      ドライバUI設置ベースパス
      <input
        value={driverUiBaseDirInput}
        oninput={(event) => onDriverUiBaseDirInput((event.currentTarget as HTMLInputElement).value)}
        placeholder="未指定時は自動探索（driver-ui/&lt;type&gt;/registration-ui.exe）"
      />
    </label>
    <div class="settings-actions">
      <button class="btn-outline" onclick={onPickDriverUiBaseDir}>フォルダ選択...</button>
      <button class="btn-primary" onclick={onSaveDriverUiBaseDir}>保存</button>
      <button class="btn-outline" onclick={onClearDriverUiBaseDir}>クリア</button>
    </div>
    {#if driverUiBaseDirSaved}
      <p class="settings-current">現在値: <code>{driverUiBaseDirSaved}</code></p>
    {/if}
    {#if settingsMessage}
      <p class="action-message">{settingsMessage}</p>
    {/if}
  </div>
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .content h2 {
    margin: 0 0 20px;
    font-size: 1.1rem;
    color: #2c3e50;
    border-bottom: 2px solid #2e86c1;
    padding-bottom: 8px;
  }

  .settings-card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 16px;
    max-width: 760px;
  }

  .settings-card h3 {
    margin: 0 0 10px;
    font-size: 0.95rem;
    color: #1f2937;
  }

  .settings-help {
    margin: 0 0 10px;
    font-size: 0.82rem;
    color: #64748b;
  }

  .settings-card label {
    display: grid;
    gap: 6px;
    font-size: 0.82rem;
    color: #334155;
    margin-bottom: 10px;
  }

  .settings-card input {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font-size: 0.84rem;
    box-sizing: border-box;
  }

  .settings-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }

  .settings-current {
    margin: 0 0 8px;
    font-size: 0.8rem;
    color: #334155;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .action-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #2563eb;
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .btn-primary {
    background-color: #2e86c1;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .btn-primary:hover {
    background-color: #2471a3;
  }

  .btn-outline {
    background: #fff;
    color: #2e86c1;
    border: 1px solid #2e86c1;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-outline:hover {
    background: #ebf5fb;
  }
</style>
