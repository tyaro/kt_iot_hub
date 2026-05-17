<script lang="ts">
  import {
    clearMqttMonitorMessages,
    getMqttMonitorStatus,
    listMqttMonitorMessages,
    listMqttMonitorPublishers,
    startMqttMonitor,
    stopMqttMonitor,
    type MqttMonitorMessageDto,
    type MqttMonitorPublisherDto,
    type MqttMonitorStatusDto,
  } from '$lib/ipc';

  const defaultStatus: MqttMonitorStatusDto = {
    connected: false,
    subscribing: false,
    publisher_id: null,
    broker: '',
    port: 0,
    topic_filter: '',
    message_count: 0,
    last_message_at: null,
    last_error: null,
  };

  function defaultTopicFilter(publisher: MqttMonitorPublisherDto | null): string {
    if (!publisher) {
      return '#';
    }
    return publisher.topic.trim() ? `${publisher.topic.trim()}/#` : '#';
  }

  let expanded = $state(false);
  let loading = $state(false);
  let actionBusy = $state(false);
  let publishers = $state<MqttMonitorPublisherDto[]>([]);
  let status = $state<MqttMonitorStatusDto>({ ...defaultStatus });
  let messages = $state<MqttMonitorMessageDto[]>([]);
  let selectedPublisherId = $state('');
  let topicFilter = $state('#');
  let localError = $state('');
  let initialized = $state(false);
  let autoScroll = $state(true);
  let listContainer = $state<HTMLDivElement | null>(null);

  async function refresh() {
    const [publisherItems, currentStatus, currentMessages] = await Promise.all([
      listMqttMonitorPublishers(),
      getMqttMonitorStatus(),
      listMqttMonitorMessages(),
    ]);

    publishers = publisherItems;
    status = currentStatus;
    messages = currentMessages;

    const preferredId = currentStatus.publisher_id ?? selectedPublisherId ?? publisherItems[0]?.id ?? '';
    selectedPublisherId = preferredId;

    if (!topicFilter.trim()) {
      const selectedPublisher =
        publisherItems.find((item) => item.id === preferredId) ?? publisherItems[0] ?? null;
      topicFilter = currentStatus.topic_filter || defaultTopicFilter(selectedPublisher);
    }
  }

  async function ensureLoaded() {
    loading = true;
    localError = '';
    try {
      await refresh();
    } catch (e) {
      localError = e instanceof Error ? e.message : 'MQTT モニタの読込に失敗しました';
    } finally {
      loading = false;
    }
  }

  function toggleExpanded() {
    expanded = !expanded;
    if (expanded && !initialized) {
      initialized = true;
      void ensureLoaded();
    }
  }

  function handlePublisherChange(event: Event) {
    const nextId = (event.currentTarget as HTMLSelectElement).value;
    selectedPublisherId = nextId;
    const publisher = publishers.find((item) => item.id === nextId) ?? null;
    topicFilter = defaultTopicFilter(publisher);
  }

  async function startMonitor() {
    actionBusy = true;
    localError = '';
    try {
      status = await startMqttMonitor({
        publisher_id: selectedPublisherId,
        topic_filter: topicFilter.trim(),
      });
      messages = await listMqttMonitorMessages();
    } catch (e) {
      localError = e instanceof Error ? e.message : 'MQTT モニタ開始に失敗しました';
    } finally {
      actionBusy = false;
    }
  }

  async function stopMonitorAction() {
    actionBusy = true;
    localError = '';
    try {
      status = await stopMqttMonitor();
    } catch (e) {
      localError = e instanceof Error ? e.message : 'MQTT モニタ停止に失敗しました';
    } finally {
      actionBusy = false;
    }
  }

  async function clearMessagesAction() {
    actionBusy = true;
    localError = '';
    try {
      await clearMqttMonitorMessages();
      messages = [];
      status = await getMqttMonitorStatus();
    } catch (e) {
      localError = e instanceof Error ? e.message : 'MQTT モニタのクリアに失敗しました';
    } finally {
      actionBusy = false;
    }
  }

  $effect(() => {
    if (!expanded) {
      return;
    }

    let disposed = false;
    const tick = async () => {
      if (disposed) {
        return;
      }
      try {
        await refresh();
      } catch (e) {
        if (!disposed) {
          localError = e instanceof Error ? e.message : 'MQTT モニタの更新に失敗しました';
        }
      }
    };

    const timerId = window.setInterval(() => {
      void tick();
    }, 2000);

    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  });

  $effect(() => {
    if (!autoScroll || !listContainer) {
      return;
    }
    listContainer.scrollTop = listContainer.scrollHeight;
  });
</script>

<div class="monitor-shell">
  <div class="monitor-header">
    <div>
      <h3>MQTT モニタ</h3>
      <p>broker に実際に流れた topic / payload を確認します。</p>
    </div>
    <button class="btn-outline" onclick={toggleExpanded}>
      {expanded ? 'モニタを閉じる' : 'MQTT モニタを開く'}
    </button>
  </div>

  {#if expanded}
    <div class="monitor-panel">
      <div class="toolbar">
        <label>
          Publisher
          <select bind:value={selectedPublisherId} onchange={handlePublisherChange} disabled={actionBusy || loading}>
            {#each publishers as publisher}
              <option value={publisher.id}>{publisher.id} ({publisher.broker}:{publisher.port})</option>
            {/each}
          </select>
        </label>

        <label class="filter-field">
          Topic Filter
          <input bind:value={topicFilter} placeholder="plant/#" disabled={actionBusy || loading} />
        </label>

        <div class="actions">
          <button class="btn-primary" onclick={startMonitor} disabled={actionBusy || loading || !selectedPublisherId || !topicFilter.trim()}>
            開始
          </button>
          <button class="btn-outline danger" onclick={stopMonitorAction} disabled={actionBusy || loading || !status.connected}>
            停止
          </button>
          <button class="btn-outline" onclick={clearMessagesAction} disabled={actionBusy || loading}>
            クリア
          </button>
        </div>
      </div>

      <div class="status-grid">
        <div class="status-card">
          <span class="status-label">状態</span>
          <strong class:ok={status.connected}>{status.connected ? '購読中' : '未接続'}</strong>
        </div>
        <div class="status-card">
          <span class="status-label">Broker</span>
          <strong>{status.broker || '-' }:{status.port || 0}</strong>
        </div>
        <div class="status-card">
          <span class="status-label">Topic Filter</span>
          <strong>{status.topic_filter || topicFilter}</strong>
        </div>
        <div class="status-card">
          <span class="status-label">受信件数</span>
          <strong>{status.message_count}</strong>
        </div>
      </div>

      <div class="sub-toolbar">
        <label class="check-label">
          <input type="checkbox" bind:checked={autoScroll} />
          自動スクロール
        </label>
        {#if status.last_message_at}
          <span class="muted">最終受信: {status.last_message_at}</span>
        {/if}
      </div>

      {#if localError}
        <p class="error-message">{localError}</p>
      {/if}
      {#if status.last_error}
        <p class="error-message">{status.last_error}</p>
      {/if}

      <div class="message-list" bind:this={listContainer}>
        {#if loading}
          <p class="muted">読込中...</p>
        {:else if messages.length === 0}
          <p class="muted">まだ受信メッセージはありません</p>
        {:else}
          {#each messages as message}
            <div class="message-item">
              <div class="message-meta">
                <span>{message.timestamp}</span>
                <span>QoS {message.qos}</span>
                <span>{message.retain ? 'retain' : 'live'}</span>
              </div>
              <div class="message-topic">{message.topic}</div>
              <div class="message-payload">{message.payload}</div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .monitor-shell {
    margin-top: 16px;
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 10px;
    padding: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  .monitor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .monitor-header h3 {
    margin: 0 0 4px;
    font-size: 1rem;
    color: #1f2937;
  }

  .monitor-header p {
    margin: 0;
    color: #64748b;
    font-size: 0.82rem;
  }

  .monitor-panel {
    margin-top: 16px;
    display: grid;
    gap: 14px;
  }

  .toolbar {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: end;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.82rem;
    font-weight: 600;
    color: #475569;
  }

  select,
  input {
    padding: 7px 8px;
    border: 1px solid #cfd8e3;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
  }

  .filter-field {
    min-width: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  .status-card {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 10px 12px;
    display: grid;
    gap: 4px;
  }

  .status-label {
    font-size: 0.75rem;
    color: #64748b;
  }

  strong.ok {
    color: #166534;
  }

  .sub-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .check-label {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .message-list {
    max-height: 320px;
    overflow: auto;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    background: #f8fafc;
    padding: 10px;
    display: grid;
    gap: 10px;
  }

  .message-item {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 10px 12px;
    display: grid;
    gap: 6px;
  }

  .message-meta {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 0.75rem;
    color: #64748b;
  }

  .message-topic {
    font-family: Consolas, monospace;
    font-size: 0.82rem;
    color: #1d4ed8;
    word-break: break-all;
  }

  .message-payload {
    font-family: Consolas, monospace;
    font-size: 0.82rem;
    color: #111827;
    word-break: break-all;
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

  .muted {
    margin: 0;
    color: #94a3b8;
    font-size: 0.82rem;
  }

  .error-message {
    margin: 0;
    font-size: 0.82rem;
    color: #b91c1c;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 10px;
  }
</style>
