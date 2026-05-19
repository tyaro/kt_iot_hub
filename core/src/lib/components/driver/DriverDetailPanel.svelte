<script lang="ts">
  import type { DriverDto, SaveDriverRequest } from '$lib/ipc/index';
  import { listDrivers, saveDriver } from '$lib/ipc/index';
  import { reloadAllRegistry } from '$lib/stores/index';
  import ConnectionForm from './driver-detail/ConnectionForm.svelte';
  import DriverMetricsSection from './driver-detail/DriverMetricsSection.svelte';

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

  function defaultForm(): SaveDriverRequest {
    return {
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
  }

  function toForm(target: DriverDto): SaveDriverRequest {
    return {
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
  }

  let form = $state<SaveDriverRequest>(defaultForm());
  let currentDriver = $state<DriverDto | null>(null);
  let editing = $state(false);
  let saving = $state(false);
  let message = $state('');
  let errorMsg = $state('');

  $effect(() => {
    if (driver) {
      currentDriver = driver;
      form = toForm(driver);
      editing = false;
      message = '';
      errorMsg = '';
    }
  });

  $effect(() => {
    if (mode === 'new') {
      currentDriver = null;
      form = defaultForm();
      editing = true;
      message = '';
      errorMsg = '';
    }
  });

  function setFormField(key: keyof SaveDriverRequest, value: string | number | boolean | null) {
    form = {
      ...form,
      [key]: value,
    } as SaveDriverRequest;
  }

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
    if (!target) {
      return;
    }
    onRequestDelete(target.id);
  }

  function requestEdit() {
    const target = currentDriver ?? driver;
    if (!target) {
      return;
    }
    form = toForm(target);
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

    form = toForm(target);
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
      <DriverMetricsSection driver={currentDriver ?? driver!} />
    {:else}
      <ConnectionForm
        {mode}
        {form}
        {saving}
        {message}
        {errorMsg}
        onFieldChange={setFormField}
        onSave={save}
        onCancel={cancelEdit}
      />
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
