<script lang="ts">
  import type { RuntimeStatusDto } from '$lib/ipc';

  let {
    tagCount,
    driverCount,
    enabledDriverCount,
    runtimeStatus,
    runtimeBusy,
    dashboardMessage,
    onNavigateTags,
    onStartServers,
    onStopServers,
  }: {
    tagCount: number;
    driverCount: number;
    enabledDriverCount: number;
    runtimeStatus: RuntimeStatusDto;
    runtimeBusy: boolean;
    dashboardMessage: string;
    onNavigateTags: () => void;
    onStartServers: () => void;
    onStopServers: () => void;
  } = $props();
</script>

<div class="content">
  <h2>ダッシュボード</h2>
  <div class="dashboard-grid">
    <div class="card">
      <span class="card-icon">🏷️</span>
      <h3>タグ</h3>
      <p class="value">{tagCount}</p>
      <p class="sub">登録済み</p>
    </div>
    <div class="card">
      <span class="card-icon">⚙️</span>
      <h3>ドライバ</h3>
      <p class="value">{driverCount}</p>
      <p class="sub">登録済み</p>
    </div>
    <div class="card">
      <span class="card-icon">⚡</span>
      <h3>有効ドライバ</h3>
      <p class="value">{enabledDriverCount}</p>
      <p class="sub">稼働中</p>
    </div>
    <div class="card runtime-card">
      <span class="card-icon">🧩</span>
      <h3>サービス状態</h3>
      <p class="runtime-badge" class:running={runtimeStatus.drivers_running || runtimeStatus.publishers_running}>
        {runtimeStatus.drivers_running || runtimeStatus.publishers_running ? '起動中' : '停止中'}
      </p>
      <ul class="runtime-list">
        <li>Drivers: {runtimeStatus.drivers_running ? 'ON' : 'OFF'}</li>
        <li>Publishers: {runtimeStatus.publishers_running ? 'ON' : 'OFF'}</li>
        <li>gRPC (IPC): {runtimeStatus.grpc_running ? 'ON' : 'OFF'}</li>
      </ul>
    </div>
  </div>
  {#if dashboardMessage}
    <p class="action-message">{dashboardMessage}</p>
  {/if}
  {#if runtimeStatus.last_error}
    <p class="error-message">{runtimeStatus.last_error}</p>
  {/if}
  <div class="quicklinks">
    <button class="btn-outline" onclick={onNavigateTags}>タグを管理</button>
    <button
      class="btn-primary"
      onclick={onStartServers}
      disabled={runtimeBusy || (runtimeStatus.drivers_running || runtimeStatus.publishers_running)}
    >
      {runtimeBusy ? '実行中...' : 'サーバ起動'}
    </button>
    <button
      class="btn-outline danger"
      onclick={onStopServers}
      disabled={runtimeBusy || (!runtimeStatus.drivers_running && !runtimeStatus.publishers_running)}
    >
      サーバ停止
    </button>
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

  .dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }

  .card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 20px 16px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  .card-icon {
    font-size: 1.8rem;
  }

  .card h3 {
    margin: 8px 0 4px;
    font-size: 0.8rem;
    color: #7f8c8d;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .value {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    color: #2e86c1;
  }

  .sub {
    margin: 2px 0 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }

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

  .action-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #2563eb;
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .error-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #b91c1c;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .runtime-card {
    text-align: left;
  }

  .runtime-badge {
    display: inline-block;
    margin: 0 0 8px;
    font-size: 0.8rem;
    font-weight: 700;
    color: #92400e;
    background: #fef3c7;
    border-radius: 999px;
    padding: 3px 10px;
  }

  .runtime-badge.running {
    color: #166534;
    background: #dcfce7;
  }

  .runtime-list {
    margin: 0;
    padding-left: 18px;
    color: #475569;
    font-size: 0.8rem;
  }
</style>
