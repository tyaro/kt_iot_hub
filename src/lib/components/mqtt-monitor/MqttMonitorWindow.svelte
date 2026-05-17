<script lang="ts">
  import {
    clearMqttMonitorMessages,
    getMqttMonitorTopicDetail,
    getMqttMonitorTree,
    getMqttMonitorStatus,
    listMqttMonitorPublishers,
    startMqttMonitor,
    stopMqttMonitor,
    type MqttMonitorTopicDetailDto,
    type MqttMonitorPublisherDto,
    type MqttMonitorStatusDto,
  } from '$lib/ipc';
  import MqttTopicTreeNode from './MqttTopicTreeNode.svelte';
  import { mergeTreeValues, type TopicTreeNode } from './tree';

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
  let treeNodes = $state<TopicTreeNode[]>([]);
  let selectedPublisherId = $state('');
  let topicFilter = $state('#');
  let includeSys = $state(true);
  let localError = $state('');
  let initialized = $state(false);
  let selectedPath = $state<string | null>(null);
  let expandedPaths = $state<string[]>([]);
  let highlightedPaths = $state<string[]>([]);
  let selectedDetail = $state<MqttMonitorTopicDetailDto | null>(null);
  let statusRefreshBusy = false;
  let treeRefreshBusy = false;
  let detailRefreshBusy = false;
  let highlightTimerId: number | null = null;

  const displayedDetail = $derived(selectedDetail?.fullPath === selectedPath ? selectedDetail : null);

  function flattenLatestMessages(nodes: TopicTreeNode[], target = new Map<string, string>()): Map<string, string> {
    for (const node of nodes) {
      if (node.latestMessage?.timestamp) {
        target.set(node.fullPath, node.latestMessage.timestamp);
      }
      flattenLatestMessages(node.children, target);
    }
    return target;
  }

  function scheduleHighlightClear() {
    if (highlightTimerId !== null) {
      window.clearTimeout(highlightTimerId);
    }
    highlightTimerId = window.setTimeout(() => {
      highlightedPaths = [];
      highlightTimerId = null;
    }, 1800);
  }

  function applyTreeUpdate(nextTree: TopicTreeNode[], options?: { markHighlights?: boolean; valuesOnly?: boolean }) {
    const { markHighlights = false, valuesOnly = false } = options ?? {};
    const appliedTree = valuesOnly ? mergeTreeValues(treeNodes, nextTree) : nextTree;

    if (markHighlights) {
      const previous = flattenLatestMessages(treeNodes);
      const current = flattenLatestMessages(appliedTree);
      const changed = Array.from(current.entries())
        .filter(([path, timestamp]) => !previous.has(path) || previous.get(path) !== timestamp)
        .map(([path]) => path);

      if (changed.length > 0) {
        highlightedPaths = changed;
        scheduleHighlightClear();
      }
    }

    treeNodes = appliedTree;
  }

  function handleSelectPath(path: string) {
    selectedPath = path;
    void refreshSelectedDetail();
  }

  function handleTogglePath(path: string) {
    const isExpanding = !expandedPaths.includes(path);
    expandedPaths = isExpanding
      ? [...expandedPaths, path]
      : expandedPaths.filter((item) => item !== path);

    if (isExpanding) {
      void refreshTree({ markHighlights: false, valuesOnly: true });
    }
  }

  function requestedExpandedPaths(): string[] {
    return [...expandedPaths];
  }

  async function refreshTree(options?: { markHighlights?: boolean; valuesOnly?: boolean; includeAll?: boolean }) {
    if (treeRefreshBusy) {
      return;
    }

    treeRefreshBusy = true;
    try {
      const nextTree = await getMqttMonitorTree({
        expanded_paths: requestedExpandedPaths(),
        include_all: options?.includeAll ?? false,
      });
      applyTreeUpdate(nextTree, options);
    } finally {
      treeRefreshBusy = false;
    }
  }

  async function refreshSelectedDetail() {
    if (!selectedPath || detailRefreshBusy) {
      return;
    }

    detailRefreshBusy = true;
    try {
      selectedDetail = await getMqttMonitorTopicDetail({
        full_path: selectedPath,
      });
    } finally {
      detailRefreshBusy = false;
    }
  }

  async function refreshStatus() {
    if (statusRefreshBusy) {
      return;
    }

    statusRefreshBusy = true;
    try {
      const [publisherItems, currentStatus] = await Promise.all([
        listMqttMonitorPublishers(),
        getMqttMonitorStatus(),
      ]);

      publishers = publisherItems;
      status = currentStatus;

      const preferredId = currentStatus.publisher_id ?? selectedPublisherId ?? publisherItems[0]?.id ?? '';
      selectedPublisherId = preferredId;

      const selectedPublisher = publisherItems.find((item) => item.id === preferredId) ?? publisherItems[0] ?? null;
      if (!currentStatus.connected && !topicFilter.trim()) {
        topicFilter = defaultTopicFilter(selectedPublisher);
      } else if (currentStatus.topic_filter) {
        topicFilter = currentStatus.topic_filter;
      }
      includeSys = currentStatus.connected ? currentStatus.include_sys : includeSys;
    } finally {
      statusRefreshBusy = false;
    }
  }

  async function refresh(options?: { includeAll?: boolean }) {
    await Promise.all([
      refreshStatus(),
      refreshTree({ includeAll: options?.includeAll ?? false }),
      refreshSelectedDetail(),
    ]);
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
      await refreshStatus();
      await refreshTree({ markHighlights: false, valuesOnly: false, includeAll: true });
      await refreshSelectedDetail();
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
      selectedDetail = null;
      highlightedPaths = [];
      await refreshStatus();
      await refreshTree({ markHighlights: false, valuesOnly: false });
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
      selectedPath = null;
      selectedDetail = null;
      highlightedPaths = [];
      await refreshStatus();
      await refreshTree({ markHighlights: false, valuesOnly: false });
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
        await refresh({ includeAll: true });
        if (!status.connected && !status.publisher_id && (selectedPublisherId || publishers[0]?.id)) {
          if (!selectedPublisherId && publishers[0]) {
            selectedPublisherId = publishers[0].id;
            topicFilter = defaultTopicFilter(publishers[0]);
          }
          await startMonitor(true);
          await refresh({ includeAll: true });
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
    const statusTimerId = window.setInterval(() => {
      void (async () => {
        if (disposed || actionBusy || loading) {
          return;
        }
        try {
          await refreshStatus();
        } catch (e) {
          if (!disposed) {
            localError = e instanceof Error ? e.message : 'MQTT モニタの更新に失敗しました';
          }
        }
      })();
    }, 2000);

    const treeTimerId = window.setInterval(() => {
      void (async () => {
        if (disposed || actionBusy || loading) {
          return;
        }
        try {
          await refreshTree({ markHighlights: true, valuesOnly: true });
        } catch (e) {
          if (!disposed) {
            localError = e instanceof Error ? e.message : 'MQTT ツリーの更新に失敗しました';
          }
        }
      })();
    }, 3500);

    const detailTimerId = window.setInterval(() => {
      void (async () => {
        if (disposed || actionBusy || loading || !selectedPath) {
          return;
        }
        try {
          await refreshSelectedDetail();
        } catch (e) {
          if (!disposed) {
            localError = e instanceof Error ? e.message : 'MQTT 詳細の更新に失敗しました';
          }
        }
      })();
    }, 2000);

    return () => {
      disposed = true;
      window.clearInterval(statusTimerId);
      window.clearInterval(treeTimerId);
      window.clearInterval(detailTimerId);
      if (highlightTimerId !== null) {
        window.clearTimeout(highlightTimerId);
      }
    };
  });
</script>

<div class="window-shell">
  <header class="window-header">
    <div>
      <h1>MQTT モニタ</h1>
      <p>本体が publish している topic と broker の <code>$SYS</code> を、落ち着いたツリービューで確認します。</p>
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
    <span class="status-chip">Broker: {status.broker || '-' }:{status.port || 0}</span>
    <span class="status-chip">Filter: {status.topic_filter || topicFilter}</span>
    <span class="status-chip">Messages: {status.message_count}</span>
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
      <div class="panel-title-row">
        <div>
          <div class="panel-title">Topics</div>
          <div class="panel-subtitle">{status.broker || 'broker'}</div>
        </div>
      </div>
      {#if loading}
        <p class="muted">読込中...</p>
      {:else if treeNodes.length === 0}
        <p class="muted">まだ受信メッセージはありません</p>
      {:else}
        <div class="tree-root">
          {#each treeNodes as node (node.id)}
            <MqttTopicTreeNode
              node={node}
              {selectedPath}
              {expandedPaths}
              {highlightedPaths}
              onSelect={handleSelectPath}
              onToggle={handleTogglePath}
            />
          {/each}
        </div>
      {/if}
    </aside>

    <section class="detail-panel">
      <div class="panel-title-row">
        <div>
          <div class="panel-title">Inspect</div>
          <div class="panel-subtitle">選択した topic の最新メッセージ</div>
        </div>
      </div>
      {#if displayedDetail?.latestMessage}
        <div class="detail-grid">
          <div>
            <span class="detail-label">Topic</span>
            <div class="detail-value code">{displayedDetail.fullPath}</div>
          </div>
          <div>
            <span class="detail-label">Timestamp</span>
            <div class="detail-value">{displayedDetail.latestMessage.timestamp}</div>
          </div>
          <div>
            <span class="detail-label">QoS</span>
            <div class="detail-value">{displayedDetail.latestMessage.qos}</div>
          </div>
          <div>
            <span class="detail-label">Retain</span>
            <div class="detail-value">{displayedDetail.latestMessage.retain ? 'true' : 'false'}</div>
          </div>
          <div class="full-width">
            <span class="detail-label">Payload</span>
            <pre class="payload-box">{displayedDetail.latestMessage.payload}</pre>
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
    gap: 10px;
    padding: 14px;
    box-sizing: border-box;
    background: #e9edf2;
  }

  .window-header {
    padding: 2px 2px 4px;
  }

  .window-header h1 {
    margin: 0 0 4px;
    font-size: 1.05rem;
    font-weight: 700;
    color: #1f2937;
  }

  .window-header p {
    margin: 0;
    font-size: 0.8rem;
    color: #64748b;
  }

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

  .status-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    font-size: 0.82rem;
    color: #475569;
  }

  .status-chip {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    padding: 0 10px;
    background: #f8fafc;
    border: 1px solid #dbe2ea;
    border-radius: 999px;
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
    grid-template-columns: 430px minmax(0, 1fr);
    gap: 10px;
  }

  .tree-panel,
  .detail-panel {
    min-height: 0;
    background: #ffffff;
    border: 1px solid #cfd8e3;
    border-radius: 8px;
    padding: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 1px 2px rgb(15 23 42 / 0.06);
  }

  .panel-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%);
    border-bottom: 1px solid #e2e8f0;
  }

  .panel-title {
    font-size: 0.78rem;
    font-weight: 700;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-subtitle {
    margin-top: 4px;
    font-size: 0.78rem;
    color: #64748b;
  }

  .tree-root {
    min-height: 0;
    overflow: auto;
    padding: 10px;
    font-size: 0.8rem;
    font-family: Consolas, monospace;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    padding: 14px;
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
    padding: 14px;
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
