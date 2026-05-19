<script lang="ts">
  import type { DriverDto } from '$lib/ipc/index';

  interface Props {
    open?: boolean;
    drivers?: DriverDto[];
    loading?: boolean;
    error?: string;
    onSelect?: (driverId: string) => void;
    onClose?: () => void;
    onReload?: () => void;
  }

  let {
    open = false,
    drivers = [],
    loading = false,
    error = '',
    onSelect = () => {},
    onClose = () => {},
    onReload = () => {},
  }: Props = $props();

  const enabledDrivers = $derived.by(() => drivers.filter((driver) => driver.enabled));
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onClose}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="ドライバ選択"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          onClose();
        }
      }}
    >
      <div class="header">
        <h3>接続先ドライバを選択</h3>
        <button type="button" class="btn-close" onclick={onClose} aria-label="閉じる">✕</button>
      </div>

      <p class="description">新規タグ登録を行う接続先を選択してください。</p>

      <div class="toolbar">
        <button type="button" class="btn-reload" onclick={onReload} disabled={loading}>
          {loading ? '読み込み中...' : '再読み込み'}
        </button>
      </div>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      {#if enabledDrivers.length === 0 && !loading}
        <p class="empty">利用可能なドライバがありません。ドライバ設定を確認してください。</p>
      {:else}
        <div class="list">
          {#each enabledDrivers as driver (driver.id)}
            <button type="button" class="driver-item" onclick={() => onSelect(driver.id)}>
              <span class="driver-main">
                <strong>{driver.id}</strong>
                <span class="type">{driver.driver_type}</span>
              </span>
              <span class="driver-sub">{driver.host}:{driver.port} / {driver.database}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="actions">
        <button type="button" class="btn-cancel" onclick={onClose}>キャンセル</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1400;
    padding: 16px;
  }

  .dialog {
    width: min(640px, 100%);
    max-height: 80vh;
    overflow: auto;
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.22);
    padding: 16px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .header h3 {
    margin: 0;
    font-size: 1rem;
    color: #1e293b;
  }

  .btn-close {
    border: none;
    background: transparent;
    font-size: 1rem;
    color: #64748b;
    cursor: pointer;
  }

  .description {
    margin: 0 0 12px;
    font-size: 0.85rem;
    color: #475569;
  }

  .toolbar {
    margin-bottom: 10px;
  }

  .btn-reload,
  .btn-cancel {
    border: none;
    border-radius: 6px;
    padding: 7px 12px;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .btn-reload {
    background: #2563eb;
    color: #fff;
  }

  .btn-reload:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .list {
    display: grid;
    gap: 8px;
  }

  .driver-item {
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    background: #fff;
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
    display: grid;
    gap: 4px;
  }

  .driver-item:hover {
    background: #eff6ff;
    border-color: #93c5fd;
  }

  .driver-main {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .type {
    font-family: monospace;
    font-size: 0.75rem;
    background: #fef3c7;
    color: #92400e;
    border-radius: 999px;
    padding: 1px 8px;
  }

  .driver-sub {
    color: #64748b;
    font-size: 0.78rem;
  }

  .error {
    color: #b91c1c;
    font-size: 0.82rem;
    margin: 0 0 10px;
  }

  .empty {
    color: #64748b;
    font-size: 0.82rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 10px;
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 12px;
  }

  .btn-cancel {
    background: #e2e8f0;
    color: #334155;
  }
</style>
