<script lang="ts">
  import { listDrivers, saveDriver, type DriverDto, type SaveDriverRequest } from '../../ipc/index';

  let drivers = $state<DriverDto[]>([]);
  let loading = $state(false);
  let saving = $state(false);
  let message = $state('');
  let errorMessage = $state('');

  let form = $state<SaveDriverRequest>({
    id: '',
    driver_type: 'postgres',
    enabled: true,
    host: '127.0.0.1',
    port: 5432,
    database: '',
    username: '',
    password: '',
    password_key: null,
    tls_enabled: false,
    tls_ca_path: null,
    tls_client_cert_path: null,
    tls_client_key_path: null,
    connect_timeout_ms: null,
    statement_timeout_ms: null,
    auto_restart: true,
    max_restart_per_minute: null,
  });

  async function reload() {
    loading = true;
    errorMessage = '';
    try {
      drivers = await listDrivers();
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : 'ドライバ一覧の取得に失敗しました';
    } finally {
      loading = false;
    }
  }

  async function submit() {
    saving = true;
    errorMessage = '';
    message = '';
    try {
      await saveDriver(form);
      message = 'ドライバ設定を保存しました';
      await reload();
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : 'ドライバ保存に失敗しました';
    } finally {
      saving = false;
    }
  }

  function editDriver(driver: DriverDto) {
    form = {
      ...form,
      id: driver.id,
      driver_type: driver.driver_type,
      enabled: driver.enabled,
      host: driver.host,
      port: driver.port,
      database: driver.database,
      username: driver.username,
      password: '',
      password_key: driver.password_key ?? null,
      tls_enabled: driver.tls_enabled ?? false,
      tls_ca_path: driver.tls_ca_path ?? null,
      tls_client_cert_path: driver.tls_client_cert_path ?? null,
      tls_client_key_path: driver.tls_client_key_path ?? null,
      connect_timeout_ms: driver.connect_timeout_ms ?? null,
      statement_timeout_ms: driver.statement_timeout_ms ?? null,
      auto_restart: driver.auto_restart ?? true,
      max_restart_per_minute: driver.max_restart_per_minute ?? null,
    };
  }

  $effect(() => {
    reload();
  });
</script>

<div class="driver-wrap">
  <div class="toolbar">
    <button class="btn-primary" onclick={reload} disabled={loading}>
      {loading ? '読み込み中...' : '一覧を更新'}
    </button>
  </div>

  <div class="grid">
    <div>
      <h3>登録済みドライバ</h3>
      <table class="driver-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>種別</th>
            <th>Host</th>
            <th>Port</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#if drivers.length === 0}
            <tr><td colspan="5" class="empty">ドライバがありません</td></tr>
          {:else}
            {#each drivers as driver (driver.id)}
              <tr>
                <td>{driver.id}</td>
                <td>{driver.driver_type}</td>
                <td>{driver.host}</td>
                <td>{driver.port}</td>
                <td><button class="btn-link" onclick={() => editDriver(driver)}>編集</button></td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>

    <div>
      <h3>ドライバ設定</h3>
      <div class="form">
        <label>ID<input bind:value={form.id} /></label>
        <label>種別
          <select bind:value={form.driver_type}>
            <option value="postgres">postgres</option>
          </select>
        </label>
        <label>Host<input bind:value={form.host} /></label>
        <label>Port<input type="number" bind:value={form.port} /></label>
        <label>Database<input bind:value={form.database} /></label>
        <label>Username<input bind:value={form.username} /></label>
        <label>Password<input type="password" bind:value={form.password} /></label>
        <label class="check"><input type="checkbox" bind:checked={form.enabled} />有効</label>
        <button class="btn-primary" onclick={submit} disabled={saving || !form.id.trim()}>
          {saving ? '保存中...' : '保存'}
        </button>
      </div>
      {#if message}<p class="ok">{message}</p>{/if}
      {#if errorMessage}<p class="error">{errorMessage}</p>{/if}
    </div>
  </div>
</div>

<style>
  .toolbar { margin-bottom: 12px; }
  .grid { display: grid; grid-template-columns: 1.1fr 1fr; gap: 18px; }
  .driver-table { width: 100%; border-collapse: collapse; border: 1px solid #dbe2ea; }
  .driver-table th, .driver-table td { padding: 8px 10px; border-bottom: 1px solid #ecf0f1; }
  .driver-table th { background: #f6f8fb; text-align: left; }
  .empty { text-align: center; color: #95a5a6; }
  .form { display: grid; gap: 8px; }
  label { display: grid; gap: 4px; font-size: 0.85rem; }
  input, select { padding: 7px 8px; border: 1px solid #cfd8e3; border-radius: 4px; }
  .check { display: flex; align-items: center; gap: 8px; }
  .btn-primary { background: #3498db; color: #fff; border: none; padding: 8px 14px; border-radius: 4px; cursor: pointer; }
  .btn-link { background: transparent; border: none; color: #2980b9; cursor: pointer; }
  .ok { color: #2e7d32; margin-top: 8px; }
  .error { color: #c0392b; margin-top: 8px; }
</style>
