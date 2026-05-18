<script lang="ts">
  import type { DriverDto } from '$lib/ipc';

  let {
    driver,
  }: {
    driver: DriverDto;
  } = $props();
</script>

<dl class="detail-list">
  <dt>ID</dt><dd class="mono">{driver.id}</dd>
  <dt>種別</dt><dd><span class="badge">{driver.driver_type}</span></dd>
  <dt>Host</dt><dd class="mono">{driver.host}</dd>
  <dt>Port</dt><dd class="mono">{driver.port}</dd>
  <dt>Database</dt><dd class="mono">{driver.database}</dd>
  <dt>Username</dt><dd class="mono">{driver.username}</dd>
  <dt>状態</dt>
  <dd>
    <span class="status" class:enabled={driver.enabled}>
      {driver.enabled ? '有効' : '無効'}
    </span>
  </dd>
</dl>

<style>
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
</style>
