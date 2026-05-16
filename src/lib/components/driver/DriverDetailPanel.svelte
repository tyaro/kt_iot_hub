<script lang="ts">
  import { saveDriver, type DriverDto, type SaveDriverRequest } from '$lib/ipc/index';
  import { reloadDrivers } from '$lib/stores/index';

  interface Props {
    driver?: DriverDto | null;
    mode?: 'detail' | 'new';
    onDone?: () => void;
    onRequestDelete?: (driverId: string) => void;
  }
  let {
    driver = null,
    mode = 'detail',
    onDone = () => {},
    onRequestDelete = () => {},
  }: Props = $props();

  let form = $state<SaveDriverRequest>({
    id: '',
    driver_type: 'postgres',
    enabled: true,
    host: '127.0.0.1',
    port: 5432,
    database: '',
    username: '',
    password: '',
  });

  let saving = $state(false);
  let message = $state('');
  let errorMsg = $state('');
  let isEditing = $state(false);

  $effect(() => {
    if (driver) {
      form = {
        id: driver.id,
        driver_type: driver.driver_type,
        enabled: driver.enabled,
        host: driver.host,
        port: driver.port,
        database: driver.database,
        username: driver.username,
        password: '',
      };
      isEditing = false;
      message = '';
      errorMsg = '';
    }
  });

  $effect(() => {
    if (mode === 'new') {
      form = {
        id: '',
        driver_type: 'postgres',
        enabled: true,
        host: '127.0.0.1',
        port: 5432,
        database: '',
        username: '',
        password: '',
      };
      isEditing = true;
      message = '';
      errorMsg = '';
    }
  });

  async function save() {
    saving = true;
    message = '';
    errorMsg = '';
    try {
      await saveDriver(form);
      message = mode === 'new' ? 'ドライバを作成しました' : 'ドライバ設定を保存しました';
      await reloadDrivers();
      if (mode === 'new') onDone();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : '保存に失敗しました';
    } finally {
      saving = false;
    }
  }

  function requestDelete() {
    if (!driver) return;
    onRequestDelete(driver.id);
  }
</script>

{#if mode === 'new' || driver}
  <div class="driver-detail">
    <div class="panel-header">
      <h3>{mode === 'new' ? '新規ドライバ' : driver?.id}</h3>
      {#if mode === 'detail' && driver}
        <div class="panel-actions">
          <button class="btn-link danger" onclick={requestDelete}>削除</button>
          <button class="btn-link" onclick={() => (isEditing = !isEditing)}>
            {isEditing ? '編集キャンセル' : '編集'}
          </button>
        </div>
      {/if}
    </div>

    {#if mode === 'detail' && !isEditing}
      <!-- 詳細表示モード -->
      <dl class="detail-list">
        <dt>ID</dt><dd class="mono">{driver?.id}</dd>
        <dt>種別</dt><dd><span class="badge">{driver?.driver_type}</span></dd>
        <dt>Host</dt><dd class="mono">{driver?.host}</dd>
        <dt>Port</dt><dd class="mono">{driver?.port}</dd>
        <dt>Database</dt><dd class="mono">{driver?.database}</dd>
        <dt>Username</dt><dd class="mono">{driver?.username}</dd>
        <dt>状態</dt>
        <dd>
          <span class="status" class:enabled={driver?.enabled}>
            {driver?.enabled ? '有効' : '無効'}
          </span>
        </dd>
      </dl>
    {:else}
      <!-- 編集/新規フォーム -->
      <div class="form">
        <label>
          ID
          <input bind:value={form.id} placeholder="postgres-main" disabled={mode === 'detail'} />
        </label>
        <label>
          種別
          <select bind:value={form.driver_type}>
            <option value="postgres">postgres</option>
          </select>
        </label>
        <label>
          Host
          <input bind:value={form.host} placeholder="127.0.0.1" />
        </label>
        <label>
          Port
          <input type="number" bind:value={form.port} min="1" max="65535" />
        </label>
        <label>
          Database
          <input bind:value={form.database} placeholder="mydb" />
        </label>
        <label>
          Username
          <input bind:value={form.username} placeholder="postgres" />
        </label>
        <label>
          Password
          <input type="password" bind:value={form.password} placeholder="（変更する場合のみ入力）" />
        </label>
        <label class="check-label">
          <input type="checkbox" bind:checked={form.enabled} />
          有効
        </label>

        <button class="btn-primary" onclick={save} disabled={saving || !form.id.trim()}>
          {saving ? '保存中...' : mode === 'new' ? '作成' : '更新'}
        </button>

        {#if message}<p class="ok">{message}</p>{/if}
        {#if errorMsg}<p class="error">{errorMsg}</p>{/if}


      </div>
    {/if}
  </div>
{:else}
  <div class="empty-state">
    <p>ドライバを選択するか<br />新規ドライバを作成してください</p>
  </div>
{/if}

<style>
  .driver-detail {
    padding: 16px;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    border-bottom: 1px solid #e3e8ef;
    padding-bottom: 10px;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #2c3e50;
    word-break: break-all;
  }

  .panel-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .detail-list {
    display: grid;
    grid-template-columns: 6em 1fr;
    gap: 6px 10px;
    margin: 0;
    font-size: 0.85rem;
  }

  dt {
    color: #7f8c8d;
    font-weight: 600;
  }

  dd {
    margin: 0;
    word-break: break-all;
  }

  .mono {
    font-family: monospace;
    font-size: 0.85rem;
  }

  .badge {
    background: #fef3c7;
    color: #92400e;
    border-radius: 3px;
    padding: 1px 6px;
    font-size: 0.8rem;
    font-family: monospace;
  }

  .status {
    font-size: 0.8rem;
    padding: 2px 6px;
    border-radius: 3px;
    background: #fee2e2;
    color: #991b1b;
  }

  .status.enabled {
    background: #dcfce7;
    color: #166534;
  }

  .form {
    display: grid;
    gap: 10px;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.85rem;
    color: #5a6776;
    font-weight: 600;
  }

  input,
  select {
    padding: 7px 8px;
    border: 1px solid #cfd8e3;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
  }

  input:disabled {
    background: #f6f8fb;
    color: #7f8c8d;
  }

  .check-label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .btn-primary {
    background: #3498db;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-link {
    background: transparent;
    border: none;
    color: #2980b9;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 4px 0;
  }

  .btn-link.danger {
    color: #b91c1c;
  }

  .ok {
    color: #2e7d32;
    font-size: 0.82rem;
    margin: 0;
  }

  .error {
    color: #c0392b;
    font-size: 0.82rem;
    margin: 0;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #95a5a6;
    font-size: 0.85rem;
    text-align: center;
    padding: 20px;
  }
</style>
