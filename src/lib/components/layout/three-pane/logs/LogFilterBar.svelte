<script lang="ts">
  import type { Destination, ErrorFocus, LogSource } from './logFilters';

  let {
    destination,
    lifecycleSource,
    errorFocus,
    keyword,
    loading,
    hasLines,
    onDestinationChange,
    onLifecycleSourceChange,
    onErrorFocusChange,
    onKeywordChange,
    onRefresh,
    onCopy,
    onClear,
  }: {
    destination: Destination;
    lifecycleSource: LogSource | 'all';
    errorFocus: ErrorFocus;
    keyword: string;
    loading: boolean;
    hasLines: boolean;
    onDestinationChange: (next: Destination) => void;
    onLifecycleSourceChange: (next: LogSource | 'all') => void;
    onErrorFocusChange: (next: ErrorFocus) => void;
    onKeywordChange: (next: string) => void;
    onRefresh: () => void;
    onCopy: () => void;
    onClear: () => void;
  } = $props();
</script>

<div class="header-row">
  <h2>ログ</h2>
  <div class="actions">
    <button class="btn-outline" class:active={destination === 'lifecycle'} onclick={() => onDestinationChange('lifecycle')}>
      起動/停止ログ
    </button>
    <button class="btn-outline" class:active={destination === 'other'} onclick={() => onDestinationChange('other')}>
      その他ログ
    </button>
    <button class="btn-outline" onclick={onRefresh} disabled={loading}>再読込</button>
    <button class="btn-outline" onclick={onCopy} disabled={!hasLines}>コピー</button>
    <button class="btn-outline danger" onclick={onClear} disabled={loading}>クリア</button>
  </div>
</div>

<div class="filters">
  <input
    class="filter-input"
    placeholder="キーワード検索"
    value={keyword}
    oninput={(event) => onKeywordChange((event.currentTarget as HTMLInputElement).value)}
  />
  {#if destination === 'lifecycle'}
    <div class="chips">
      <button class="chip" class:active={lifecycleSource === 'all'} onclick={() => onLifecycleSourceChange('all')}>全体</button>
      <button class="chip" class:active={lifecycleSource === 'main'} onclick={() => onLifecycleSourceChange('main')}>本体</button>
      <button class="chip" class:active={lifecycleSource === 'grpc'} onclick={() => onLifecycleSourceChange('grpc')}>gRPC</button>
      <button class="chip" class:active={lifecycleSource === 'mqtt'} onclick={() => onLifecycleSourceChange('mqtt')}>MQTT</button>
      <button class="chip" class:active={lifecycleSource === 'driver'} onclick={() => onLifecycleSourceChange('driver')}>通信ドライバ</button>
      <button class="chip" class:active={lifecycleSource === 'registration-ui'} onclick={() => onLifecycleSourceChange('registration-ui')}>登録UI</button>
    </div>
  {:else}
    <div class="chips">
      <button class="chip" class:active={errorFocus === 'all'} onclick={() => onErrorFocusChange('all')}>全ログ</button>
      <button class="chip" class:active={errorFocus === 'driver'} onclick={() => onErrorFocusChange('driver')}>ドライバエラー</button>
      <button class="chip" class:active={errorFocus === 'mqtt'} onclick={() => onErrorFocusChange('mqtt')}>MQTTエラー</button>
      <button class="chip" class:active={errorFocus === 'grpc'} onclick={() => onErrorFocusChange('grpc')}>gRPCエラー</button>
    </div>
  {/if}
</div>

<style>
  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
    color: #2c3e50;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .filters {
    display: grid;
    gap: 8px;
    margin-bottom: 10px;
  }

  .filter-input {
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 0.85rem;
  }

  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .chip {
    border: 1px solid #cbd5e1;
    background: #fff;
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
  }

  .chip.active {
    border-color: #2e86c1;
    color: #1d4ed8;
    background: #eff6ff;
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

  .btn-outline.active {
    background: #eff6ff;
    border-color: #1d4ed8;
    color: #1d4ed8;
  }

  .btn-outline.danger {
    color: #b91c1c;
    border-color: #fca5a5;
  }

  .btn-outline.danger:hover {
    background: #fef2f2;
  }

  .btn-outline:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
</style>
