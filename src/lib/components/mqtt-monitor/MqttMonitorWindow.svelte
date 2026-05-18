<script lang="ts">
  import ControlBar from './monitor/ControlBar.svelte';
  import DetailPanel from './monitor/DetailPanel.svelte';
  import { startMonitorPolling } from './monitor/monitorPolling';
  import TopicTreePanel from './monitor/TopicTreePanel.svelte';
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

  function handleTopicFilterChange(value: string) {
    topicFilter = value;
  }

  function handleIncludeSysChange(value: boolean) {
    includeSys = value;
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
    const cleanupPolling = startMonitorPolling({
      shouldSkip: () => actionBusy || loading,
      hasSelectedPath: () => Boolean(selectedPath),
      onStatusTick: async () => {
        await refreshStatus();
      },
      onTreeTick: async () => {
        await refreshTree({ markHighlights: true, valuesOnly: true });
      },
      onDetailTick: async () => {
        await refreshSelectedDetail();
      },
      onError: (message) => {
        localError = message;
      },
    });

    return () => {
      cleanupPolling();
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

  <ControlBar
    {publishers}
    {selectedPublisherId}
    {topicFilter}
    {includeSys}
    {loading}
    {actionBusy}
    statusConnected={status.connected}
    onPublisherChange={handlePublisherChange}
    onTopicFilterChange={handleTopicFilterChange}
    onIncludeSysChange={handleIncludeSysChange}
    onStart={() => {
      void startMonitor(false);
    }}
    onStop={() => {
      void stopMonitorAction();
    }}
    onClear={() => {
      void clearMessagesAction();
    }}
  />

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
    <TopicTreePanel
      {loading}
      broker={status.broker}
      {treeNodes}
      {selectedPath}
      {expandedPaths}
      {highlightedPaths}
      onSelectPath={handleSelectPath}
      onTogglePath={handleTogglePath}
    />

    <DetailPanel {displayedDetail} />
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
