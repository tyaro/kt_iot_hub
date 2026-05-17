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
  import MqttTopicTreeNode from './MqttTopicTreeNode.svelte';
  import { buildTopicTree, findNodeByPath, type TopicTreeNode } from './tree';

  const defaultStatus: MqttMonitorStatusDto = {
    connected: false,
    subscribing: false,
    include_sys: true,
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

  let loading = $state(true);
  let actionBusy = $state(false);
  let publishers = $state<MqttMonitorPublisherDto[]>([]);
  let status = $state<MqttMonitorStatusDto>({ ...defaultStatus });
  let messages = $state<MqttMonitorMessageDto[]>([]);
  let selectedPublisherId = $state('');
  let topicFilter = $state('#');
  let includeSys = $state(true);
  let localError = $state('');
  let initialized = $state(false);
  let selectedPath = $state<string | null>(null);

  const treeNodes = $derived.by<TopicTreeNode[]>(() => buildTopicTree(messages));
  const selectedNode = $derived(findNodeByPath(treeNodes, selectedPath));

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

    const selectedPublisher = publisherItems.find((item) => item.id === preferredId) ?? publisherItems[0] ?? null;
    if (!currentStatus.connected && !topicFilter.trim()) {
      topicFilter = defaultTopicFilter(selectedPublisher);
    } else if (currentStatus.topic_filter) {
      topicFilter = currentStatus.topic_filter;
    }
    includeSys = currentStatus.connected ? currentStatus.include_sys : includeSys;

    if (!selectedPath && currentMessages[0]) {
      selectedPath = currentMessages[0].topic;
    }
  }

  async function startMonitor(auto = false) {
    if (!selectedPublisherId || !topicFilter.trim()) {
      return;
    }

    actionBusy = !auto;
    localError = '';
    try {
      status = await startMqttMonitor({
        publisher_id: selectedPublisherId,
        topic_filter: topicFilter.trim(),
        include_sys: includeSys,
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

  function handlePublisherChange(event: Event) {
    const nextId = (event.currentTarget as HTMLSelectElement).value;
    selectedPublisherId = nextId;
    const publisher = publishers.find((item) => item.id === nextId) ?? null;
    topicFilter = defaultTopicFilter(publisher);
  }

  $effect(() => {
    if (initialized) {
      return;
    }

    initialized = true;
    void (async () => {
      loading = true;
      try {
        await refresh();
        if (!status.connected && !status.publisher_id && (selectedPublisherId || publishers[0]?.id)) {
          if (!selectedPublisherId && publishers[0]) {
            selectedPublisherId = publishers[0].id;
            topicFilter = defaultTopicFilter(publishers[0]);
          }
          await startMonitor(true);
          await refresh();
        }
      } catch (e) {
        localError = e instanceof Error ? e.message : 'MQTT モニタの読込に失敗しました';
      } finally {
        loading = false;
      }
    })();
  });

  $effect(() => {
    let disposed = false;
    const timerId = window.setInterval(() => {
      void (async () => {
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
      })();
    }, 2000);

    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  });
</script>

<div class="window-shell">
  <header class="window-header">
    <div>
      <h1>MQTT モニタ</h1>
      <p>本体が publish している topic と broker の <code>$SYS</code> を確認します。</p>
    </div>
  </header>

  <section class="toolbar">
    <label>
      Publisher
      <select bind:value={selectedPublisherId} onchange={handlePublisherChange} disabled={loading || actionBusy}>
        {#each publishers as publisher}
          <option value={publisher.id}>{publisher.id} ({publisher.broker}:{publisher.port})</option>
        {/each}
      </select>
    </label>

    <label class="wide">
      Topic Filter
      <input bind:value={topicFilter} placeholder="plant/#" disabled={loading || actionBusy} />
    </label>

    <label class="check-label">
      <input type="checkbox" bind:checked={includeSys} disabled={loading || actionBusy} />
      $SYS を表示
    </label>

    <div class="actions">
      <button class="btn-primary" onclick={() => startMonitor(false)} disabled={loading || actionBusy || !selectedPublisherId || !topicFilter.trim()}>
        開始
      </button>
      <button class="btn-outline danger" onclick={stopMonitorAction} disabled={loading || actionBusy || !status.connected}>
        停止
      </button>
      <button class="btn-outline" onclick={clearMessagesAction} disabled={loading || actionBusy}>
        クリア
      </button>
    </div>
  </section>

  <section class="status-row">
    <span class="badge" class:connected={status.connected}>{status.connected ? '購読中' : '未接続'}</span>
    <span>Broker: {status.broker || '-' }:{status.port || 0}</span>
    <span>Filter: {status.topic_filter || topicFilter}</span>
    <span>Messages: {status.message_count}</span>
    {#if status.last_message_at}<span>Last: {status.last_message_at}</span>{/if}
  </section>

  {#if localError}
    <p class="error-message">{localError}</p>
  {/if}
  {#if status.last_error}
    <p class="error-message">{status.last_error}</p>
  {/if}

  <section class="content-grid">
    <aside class="tree-panel">
      <div class="panel-title">{status.broker || 'broker'}</div>
      {#if loading}
        <p class="muted">読込中...</p>
      {:else if treeNodes.length === 0}
        <p class="muted">まだ受信メッセージはありません</p>
      {:else}
        <div class="tree-root">
          {#each treeNodes as node (node.id)}
            <MqttTopicTreeNode node={node} {selectedPath} onSelect={(path) => (selectedPath = path)} />
          {/each}
        </div>
      {/if}
    </aside>

    <section class="detail-panel">
      <div class="panel-title">詳細</div>
      {#if selectedNode?.latestMessage}
        <div class="detail-grid">
          <div>
            <span class="detail-label">Topic</span>
            <div class="detail-value code">{selectedNode.fullPath}</div>
          </div>
          <div>
            <span class="detail-label">Timestamp</span>
            <div class="detail-value">{selectedNode.latestMessage.timestamp}</div>
          </div>
          <div>
            <span class="detail-label">QoS</span>
            <div class="detail-value">{selectedNode.latestMessage.qos}</div>
          </div>
          <div>
            <span class="detail-label">Retain</span>
            <div class="detail-value">{selectedNode.latestMessage.retain ? 'true' : 'false'}</div>
          </div>
          <div class="full-width">
            <span class="detail-label">Payload</span>
            <pre class="payload-box">{selectedNode.latestMessage.payload}</pre>
          </div>
        </div>
      {:else}
        <p class="muted">左の topic ツリーから項目を選択してください。</p>
      {/if}
    </section>
  </section>
</div>

<style>
  :global(html, body, #app) {
    overflow: hidden;
  }

  .window-shell {
    height: 100%;
    display: grid;
    grid-template-rows: auto auto auto 1fr;
    gap: 12px;
    padding: 18px 20px;
    box-sizing: border-box;
    background: #f4f6f9;
  }

  .window-header h1 {
    margin: 0 0 4px;
    font-size: 1.2rem;
    color: #1f2937;
  }

  .window-header p {
    margin: 0;
    font-size: 0.84rem;
    color: #64748b;
  }

  .toolbar {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr) auto auto;
    gap: 12px;
    align-items: end;
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 10px;
    padding: 14px;
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

  .status-row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    font-size: 0.82rem;
    color: #475569;
  }

  .badge {
    display: inline-block;
    font-size: 0.78rem;
    font-weight: 700;
    color: #92400e;
    background: #fef3c7;
    border-radius: 999px;
    padding: 4px 10px;
  }

  .badge.connected {
    color: #166534;
    background: #dcfce7;
  }

  .content-grid {
    min-height: 0;
    display: grid;
    grid-template-columns: 420px minmax(0, 1fr);
    gap: 12px;
  }

  .tree-panel,
  .detail-panel {
    min-height: 0;
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
  }

  .panel-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: #475569;
    margin-bottom: 10px;
  }

  .tree-root {
    min-height: 0;
    overflow: auto;
    font-size: 0.82rem;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .detail-label {
    display: block;
    margin-bottom: 4px;
    font-size: 0.76rem;
    color: #64748b;
  }

  .detail-value {
    font-size: 0.84rem;
    color: #111827;
    word-break: break-all;
  }

  .code,
  .payload-box {
    font-family: Consolas, monospace;
  }

  .payload-box {
    margin: 0;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 10px;
    white-space: pre-wrap;
    word-break: break-all;
    overflow: auto;
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

  .error-message {
    margin: 0;
    font-size: 0.82rem;
    color: #b91c1c;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .muted {
    margin: 0;
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
