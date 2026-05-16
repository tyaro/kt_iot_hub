<script lang="ts">
  import { driversStore, reloadDrivers } from '$lib/stores/index';
  import type { DriverDto } from '$lib/ipc/index';

  interface Props {
    onSelect?: (driver: DriverDto | null) => void;
    selectedId?: string | null;
  }
  let { onSelect = () => {}, selectedId = null }: Props = $props();

  $effect(() => {
    reloadDrivers();
  });
</script>

<div class="driver-list-wrap">
  <div class="toolbar">
    <button
      class="btn-primary"
      onclick={() => reloadDrivers()}
      disabled={$driversStore.loading}
    >
      {$driversStore.loading ? '読み込み中...' : '再読み込み'}
    </button>
  </div>

  {#if $driversStore.error}
    <p class="error">{$driversStore.error}</p>
  {/if}

  <table class="driver-table">
    <thead>
      <tr>
        <th>ID</th>
        <th>種別</th>
        <th>Host</th>
        <th>Port</th>
        <th>状態</th>
      </tr>
    </thead>
    <tbody>
      {#if $driversStore.items.length === 0 && !$driversStore.loading}
        <tr><td colspan="5" class="empty">ドライバがありません</td></tr>
      {:else}
        {#each $driversStore.items as driver (driver.id)}
          <tr
            class:selected={driver.id === selectedId}
            onclick={() => onSelect(driver)}
          >
            <td class="mono">{driver.id}</td>
            <td><span class="badge">{driver.driver_type}</span></td>
            <td>{driver.host}</td>
            <td>{driver.port}</td>
            <td>
              <span class="status" class:enabled={driver.enabled}>
                {driver.enabled ? '有効' : '無効'}
              </span>
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>

<style>
  .toolbar {
    margin-bottom: 10px;
  }

  .driver-table {
    width: 100%;
    border-collapse: collapse;
    background: #fff;
    border: 1px solid #dbe2ea;
    font-size: 0.85rem;
  }

  .driver-table th,
  .driver-table td {
    padding: 8px 10px;
    border-bottom: 1px solid #ecf0f1;
    text-align: left;
  }

  .driver-table th {
    background-color: #f6f8fb;
    font-weight: 600;
    color: #5a6776;
  }

  .driver-table tr:hover {
    background-color: #f0f7ff;
    cursor: pointer;
  }

  .driver-table tr.selected {
    background-color: #dbeafe;
  }

  .mono {
    font-family: monospace;
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

  .empty {
    text-align: center;
    color: #95a5a6;
    padding: 20px;
  }

  .error {
    color: #c0392b;
    margin: 8px 0;
    font-size: 0.85rem;
  }

  .btn-primary {
    background-color: #3498db;
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
</style>
