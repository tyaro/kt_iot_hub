<script lang="ts">
  let {
    driverUiBaseDirInput,
    driverUiBaseDirSaved,
    settingsMessage,
    discoveryAvailableCount,
    discoveryInvalidCount,
    discoveryInvalidItems,
    onDriverUiBaseDirInput,
    onPickDriverUiBaseDir,
    onSaveDriverUiBaseDir,
    onClearDriverUiBaseDir,
  }: {
    driverUiBaseDirInput: string;
    driverUiBaseDirSaved: string | null;
    settingsMessage: string;
    discoveryAvailableCount: number;
    discoveryInvalidCount: number;
    discoveryInvalidItems: Array<{
      manifestPath: string;
      statusCode: string;
      statusMessage: string;
      driverTypeHint?: string | null;
    }>;
    onDriverUiBaseDirInput: (value: string) => void;
    onPickDriverUiBaseDir: () => void | Promise<void>;
    onSaveDriverUiBaseDir: () => void;
    onClearDriverUiBaseDir: () => void;
  } = $props();
</script>

<div class="content">
  <h2>設定</h2>
  <div class="settings-card">
    <h3>ドライバ実行ファイル配置</h3>
    <p class="settings-help">
      登録UI と通信ドライバを同じ場所に置く前提です。既定値は同梱インストーラの配置先で、通常は <code>&lt;app-dir&gt;\resources</code> を使います。
    </p>
    <label>
      ドライバ設置ベースパス
      <input
        value={driverUiBaseDirInput}
        oninput={(event) => onDriverUiBaseDirInput((event.currentTarget as HTMLInputElement).value)}
        placeholder="既定値は同梱インストーラの配置先（通常は &lt;app-dir&gt;\resources）"
      />
    </label>
    <p class="settings-help">
      例: 本体が <code>&lt;app-dir&gt;\kt_iot_hub.exe</code> の場合、同梱インストーラの既定値は <code>&lt;app-dir&gt;\resources</code> です。ここから <code>driver-ui\postgres\registration-ui.exe</code> と <code>driver-ui\postgres\driver-postgres.exe</code> を探索します。
    </p>
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

    <div class="discovery-summary">
      <h4>マニフェスト検出結果</h4>
      <p class="settings-help">
        利用可能: <strong>{discoveryAvailableCount}</strong> 件 / 無効: <strong>{discoveryInvalidCount}</strong> 件
      </p>

      {#if discoveryInvalidItems.length > 0}
        <ul class="invalid-list">
          {#each discoveryInvalidItems as item (`${item.manifestPath}:${item.statusCode}`)}
            <li>
              <div class="invalid-header">
                <code>{item.driverTypeHint ?? 'unknown'}</code>
                <span class="invalid-code">{item.statusCode}</span>
              </div>
              <div class="invalid-message">{item.statusMessage}</div>
              <div class="invalid-path">{item.manifestPath}</div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
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

  .discovery-summary {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid #e2e8f0;
  }

  .discovery-summary h4 {
    margin: 0 0 8px;
    font-size: 0.88rem;
    color: #1f2937;
  }

  .invalid-list {
    margin: 8px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 8px;
  }

  .invalid-header {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 4px;
  }

  .invalid-code {
    font-size: 0.74rem;
    color: #92400e;
    background: #fef3c7;
    border-radius: 999px;
    padding: 1px 8px;
  }

  .invalid-message {
    font-size: 0.8rem;
    color: #7c2d12;
  }

  .invalid-path {
    font-size: 0.76rem;
    color: #64748b;
    word-break: break-all;
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
