<script lang="ts">
  let {
    runtimeBusy,
    driversRunning,
    publishersRunning,
    onNavigateTags,
    onOpenMqttMonitor,
    onStartServers,
    onStopServers,
  }: {
    runtimeBusy: boolean;
    driversRunning: boolean;
    publishersRunning: boolean;
    onNavigateTags: () => void;
    onOpenMqttMonitor: () => void;
    onStartServers: () => void;
    onStopServers: () => void;
  } = $props();
</script>

<div class="quicklinks">
  <button class="btn-outline" onclick={onNavigateTags}>タグを管理</button>
  <button class="btn-outline" onclick={onOpenMqttMonitor}>MQTT モニタを開く</button>
  <button
    class="btn-primary"
    onclick={onStartServers}
    disabled={runtimeBusy || driversRunning || publishersRunning}
  >
    {runtimeBusy ? '実行中...' : 'ドライバ / MQTT 開始'}
  </button>
  <button
    class="btn-outline danger"
    onclick={onStopServers}
    disabled={runtimeBusy || (!driversRunning && !publishersRunning)}
  >
    ドライバ / MQTT 停止
  </button>
</div>

<style>
  .quicklinks {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
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

  .btn-outline.danger {
    color: #b91c1c;
    border-color: #fca5a5;
  }

  .btn-outline.danger:hover {
    background: #fef2f2;
  }
</style>
