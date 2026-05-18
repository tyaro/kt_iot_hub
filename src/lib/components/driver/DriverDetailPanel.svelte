<script lang="ts">
  import { listDrivers, saveDriver, type DriverDto, type SaveDriverRequest } from '$lib/ipc/index';
  import { reloadAllRegistry } from '$lib/stores/index';

  interface Props {
    driver?: DriverDto | null;
    mode?: 'detail' | 'new';
    onDone?: (driverId?: string) => void | Promise<DriverDto | null>;
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
    original_id: null,
    driver_type: 'postgres',
    enabled: true,
    host: '127.0.0.1',
    port: 5432,
    database: '',
    username: '',
    password: '',
  });
  let currentDriver = $state<DriverDto | null>(null);
  let editing = $state(false);

  let saving = $state(false);
  let message = $state('');
  let errorMsg = $state('');
  $effect(() => {
    if (driver) {
      currentDriver = driver;
      form = {
        id: driver.id,
        original_id: driver.id,
        driver_type: driver.driver_type,
        enabled: driver.enabled,
        host: driver.host,
        port: driver.port,
        database: driver.database,
        username: driver.username,
        password: '',
      };
      editing = false;
      message = '';
      errorMsg = '';
    }
  });

  $effect(() => {
    if (mode === 'new') {
      currentDriver = null;
      form = {
        id: '',
        original_id: null,
        driver_type: 'postgres',
        enabled: true,
        host: '127.0.0.1',
        port: 5432,
        database: '',
        username: '',
        password: '',
      };
      editing = true;
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
      await reloadAllRegistry();

      const driverId = form.id.trim();
      const refreshedDrivers = await listDrivers();
      const refreshedDriver = refreshedDrivers.find((item) => item.id === driverId) ?? null;

      currentDriver = refreshedDriver;
      form = {
        id: refreshedDriver?.id ?? driverId,
        original_id: refreshedDriver?.id ?? driverId,
        driver_type: refreshedDriver?.driver_type ?? form.driver_type,
        enabled: refreshedDriver?.enabled ?? form.enabled,
        host: refreshedDriver?.host ?? form.host,
        port: refreshedDriver?.port ?? form.port,
        database: refreshedDriver?.database ?? form.database,
        username: refreshedDriver?.username ?? form.username,
        password: '',
      };
      editing = false;
      message = mode === 'new' ? 'ドライバを作成しました' : 'ドライバ設定を保存しました';
      await onDone(form.id.trim());
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : '保存に失敗しました';
    } finally {
      saving = false;
    }
  }

  function requestDelete() {
    const target = currentDriver ?? driver;
    if (!target) return;
    onRequestDelete(target.id);
  }

  function requestEdit() {
    const target = currentDriver ?? driver;
    if (!target) return;
    form = {
      id: target.id,
      original_id: target.id,
      driver_type: target.driver_type,
      enabled: target.enabled,
      host: target.host,
      port: target.port,
      database: target.database,
      username: target.username,
      password: '',
    };
    message = '';
    errorMsg = '';
    editing = true;
  }

  function cancelEdit() {
    if (mode === 'new') {
      return;
    }

    const target = currentDriver ?? driver;
    if (!target) {
      editing = false;
      return;
    }

    form = {
      id: target.id,
      original_id: target.id,
      driver_type: target.driver_type,
      enabled: target.enabled,
      host: target.host,
      port: target.port,
      database: target.database,
      username: target.username,
      password: '',
    };
    message = '';
    errorMsg = '';
    editing = false;
  }
</script>

{#if mode === 'new' || currentDriver || driver}
  <div class="driver-detail">
    <div class="panel-header">
      <h3>{mode === 'new' ? '新規ドライバ' : (currentDriver?.id ?? driver?.id)}</h3>
      {#if mode === 'detail' && !editing && (currentDriver || driver)}
        <div class="panel-actions">
          <button class="btn-link" onclick={requestEdit}>編集</button>
          <span class="action-separator" aria-hidden="true"></span>
          <button class="btn-link danger" onclick={requestDelete}>削除</button>
        </div>
      {/if}
    </div>

    {#if mode === 'detail' && !editing}
      <!-- 詳細表示モード -->
      <dl class="detail-list">
        <dt>ID</dt><dd class="mono">{currentDriver?.id ?? driver?.id}</dd>
        <dt>種別</dt><dd><span class="badge">{currentDriver?.driver_type ?? driver?.driver_type}</span></dd>
        <dt>Host</dt><dd class="mono">{currentDriver?.host ?? driver?.host}</dd>
        <dt>Port</dt><dd class="mono">{currentDriver?.port ?? driver?.port}</dd>
        <dt>Database</dt><dd class="mono">{currentDriver?.database ?? driver?.database}</dd>
        <dt>Username</dt><dd class="mono">{currentDriver?.username ?? driver?.username}</dd>
        <dt>状態</dt>
        <dd>
          <span class="status" class:enabled={currentDriver?.enabled ?? driver?.enabled}>
            {(currentDriver?.enabled ?? driver?.enabled) ? '有効' : '無効'}
          </span>
        </dd>
      </dl>
    {:else}
      <!-- 編集/新規フォーム -->
      <div class="form">
        <label>
          ID
          <input bind:value={form.id} placeholder="postgres-main" />
        </label>
        {#if mode !== 'new'}
          <p class="hint">接続先IDを変更すると、配下の Scan グループとタグ参照もまとめて更新します。</p>
        {/if}
        <label>
          種別
          <select bind:value={form.driver_type} disabled={mode !== 'new'}>
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

        <div class="form-actions">
          <button class="btn-primary" onclick={save} disabled={saving || !form.id.trim()}>
            {saving ? '保存中...' : mode === 'new' ? '作成' : '更新'}
          </button>
          {#if mode !== 'new'}
            <button class="btn-secondary" onclick={cancelEdit} disabled={saving}>キャンセル</button>
          {/if}
        </div>

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
    gap: 12px;
  }

  .action-separator {
    width: 1px;
    height: 18px;
    background: #d7dee8;
    flex: 0 0 auto;
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

  .hint {
    margin: -2px 0 0;
    font-size: 0.78rem;
    color: #64748b;
    line-height: 1.45;
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

  .form-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
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

  .btn-secondary {
    background: #fff;
    color: #475569;
    border: 1px solid #cbd5e1;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary:disabled {
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
