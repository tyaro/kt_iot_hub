<script lang="ts">
  import type { MqttMonitorPublisherDto } from '$lib/ipc';

  let {
    publishers,
    selectedPublisherId,
    topicFilter,
    includeSys,
    loading,
    actionBusy,
    statusConnected,
    onPublisherChange,
    onTopicFilterChange,
    onIncludeSysChange,
    onStart,
    onStop,
    onClear,
  }: {
    publishers: MqttMonitorPublisherDto[];
    selectedPublisherId: string;
    topicFilter: string;
    includeSys: boolean;
    loading: boolean;
    actionBusy: boolean;
    statusConnected: boolean;
    onPublisherChange: (event: Event) => void;
    onTopicFilterChange: (value: string) => void;
    onIncludeSysChange: (value: boolean) => void;
    onStart: () => void;
    onStop: () => void;
    onClear: () => void;
  } = $props();
</script>

<section class="toolbar">
  <label>
    Publisher
    <select value={selectedPublisherId} onchange={onPublisherChange} disabled={loading || actionBusy}>
      {#each publishers as publisher}
        <option value={publisher.id}>{publisher.id} ({publisher.broker}:{publisher.port})</option>
      {/each}
    </select>
  </label>

  <label class="wide">
    Topic Filter
    <input
      value={topicFilter}
      placeholder="plant/#"
      disabled={loading || actionBusy}
      oninput={(event) => onTopicFilterChange((event.currentTarget as HTMLInputElement).value)}
    />
  </label>

  <label class="check-label">
    <input
      type="checkbox"
      checked={includeSys}
      disabled={loading || actionBusy}
      onchange={(event) => onIncludeSysChange((event.currentTarget as HTMLInputElement).checked)}
    />
    $SYS を表示
  </label>

  <div class="actions">
    <button class="btn-primary" onclick={onStart} disabled={loading || actionBusy || !selectedPublisherId || !topicFilter.trim()}>
      開始
    </button>
    <button class="btn-outline danger" onclick={onStop} disabled={loading || actionBusy || !statusConnected}>
      停止
    </button>
    <button class="btn-outline" onclick={onClear} disabled={loading || actionBusy}>
      クリア
    </button>
  </div>
</section>

<style>
  .toolbar {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr) auto auto;
    gap: 10px;
    align-items: end;
    background: linear-gradient(180deg, #fefefe 0%, #f8fafc 100%);
    border: 1px solid #cfd8e3;
    border-radius: 8px;
    padding: 12px;
    box-shadow: 0 1px 2px rgb(15 23 42 / 0.06);
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.82rem;
    font-weight: 600;
    color: #475569;
  }

  .wide {
    min-width: 0;
  }

  input,
  select {
    padding: 7px 8px;
    border: 1px solid #cfd8e3;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
    min-height: 34px;
  }

  .check-label {
    display: flex;
    align-items: center;
    gap: 8px;
    align-self: center;
  }

  .actions {
    display: flex;
    gap: 8px;
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

  .btn-outline.danger {
    color: #b91c1c;
    border-color: #fca5a5;
  }
</style>
