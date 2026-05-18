<script lang="ts">
  import type { AppMetricsDto, DriverMetricsDto, RuntimeStatusDto } from '$lib/ipc';

  function formatBytes(value?: number | null): string {
    if (value == null || !Number.isFinite(value)) {
      return '-';
    }
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = value;
    let index = 0;
    while (size >= 1024 && index < units.length - 1) {
      size /= 1024;
      index += 1;
    }
    return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
  }

  function formatPercent(value?: number | null): string {
    if (value == null || !Number.isFinite(value)) {
      return '-';
    }
    return `${value.toFixed(1)}%`;
  }

  function formatByteRate(value?: number | null): string {
    if (value == null || !Number.isFinite(value)) {
      return '-';
    }
    if (value < 1024) {
      return `${value.toFixed(1)} B`;
    }
    return formatBytes(value);
  }

  function cpuLevel(value?: number | null): 'normal' | 'warn' | 'danger' {
    if (value == null || !Number.isFinite(value)) {
      return 'normal';
    }
    if (value >= 90) {
      return 'danger';
    }
    if (value >= 70) {
      return 'warn';
    }
    return 'normal';
  }

  function ioLevel(value?: number | null): 'normal' | 'warn' | 'danger' {
    if (value == null || !Number.isFinite(value)) {
      return 'normal';
    }
    if (value >= 10 * 1024 * 1024) {
      return 'danger';
    }
    if (value >= 1024 * 1024) {
      return 'warn';
    }
    return 'normal';
  }

  type MetricItem = {
    icon?: string;
    label: string;
    value: string;
    tone?: 'normal' | 'warn' | 'danger';
  };

  type RuntimeStateTone = 'running' | 'partial' | 'stopped' | 'error';

  let {
    tagCount,
    driverCount,
    enabledDriverCount,
    runtimeStatus,
    appMetrics,
    driverMetrics,
    runtimeBusy,
    dashboardMessage,
    scanCycleHealthSummary,
    onNavigateTags,
    onOpenMqttMonitor,
    onStartServers,
    onStopServers,
  }: {
    tagCount: number;
    driverCount: number;
    enabledDriverCount: number;
    runtimeStatus: RuntimeStatusDto;
    appMetrics: AppMetricsDto & {
      webview_memory_used_bytes: number | null;
      webview_memory_total_bytes: number | null;
      webview_memory_limit_bytes: number | null;
    };
    driverMetrics: DriverMetricsDto[];
    runtimeBusy: boolean;
    dashboardMessage: string;
    scanCycleHealthSummary: {
      observedGroupCount: number;
      delayedGroupCount: number;
      avgDeltaRatio: number | null;
      worstGroupLabel: string | null;
      worstDeltaRatio: number | null;
    };
    onNavigateTags: () => void;
    onOpenMqttMonitor: () => void;
    onStartServers: () => void;
    onStopServers: () => void;
  } = $props();

  let appMetricItems = $derived.by<MetricItem[]>(() => [
    {
      icon: '🧠',
      label: 'CPU(プロセス)',
      value: formatPercent(appMetrics.process_cpu_percent),
      tone: cpuLevel(appMetrics.process_cpu_percent),
    },
    {
      icon: '💾',
      label: 'メモリ(プロセス)',
      value: formatBytes(appMetrics.process_memory_bytes),
    },
    {
      icon: '🌐',
      label: 'WebView使用量',
      value: formatBytes(appMetrics.webview_memory_used_bytes),
    },
    {
      icon: '📦',
      label: 'WebView確保量',
      value: formatBytes(appMetrics.webview_memory_total_bytes),
    },
    {
      icon: '🧱',
      label: 'WebView上限',
      value: formatBytes(appMetrics.webview_memory_limit_bytes),
    },
  ]);

  let systemMetricItems = $derived.by<MetricItem[]>(() => [
    {
      icon: '🧠',
      label: 'CPU(システム)',
      value: formatPercent(appMetrics.system_cpu_percent),
      tone: cpuLevel(appMetrics.system_cpu_percent),
    },
    {
      icon: '💽',
      label: 'メモリ(システム)',
      value: `${formatBytes(appMetrics.system_memory_used_bytes)} / ${formatBytes(appMetrics.system_memory_total_bytes)}`,
    },
    {
      icon: '📥',
      label: 'ネット受信',
      value: `${formatBytes(appMetrics.network_rx_bytes_per_sec)}/s`,
      tone: ioLevel(appMetrics.network_rx_bytes_per_sec),
    },
    {
      icon: '📤',
      label: 'ネット送信',
      value: `${formatBytes(appMetrics.network_tx_bytes_per_sec)}/s`,
      tone: ioLevel(appMetrics.network_tx_bytes_per_sec),
    },
  ]);

  let runtimeOverview = $derived.by<{
    tone: RuntimeStateTone;
  }>(() => {
    if (runtimeStatus.last_error) {
      return {
        tone: 'error',
      };
    }

    if (runtimeStatus.drivers_running && runtimeStatus.publishers_running) {
      return {
        tone: 'running',
      };
    }

    if (runtimeStatus.drivers_running || runtimeStatus.publishers_running) {
      return {
        tone: 'partial',
      };
    }

    return {
      tone: 'stopped',
    };
  });

  let runtimeBadge = $derived.by(() => {
    switch (runtimeOverview.tone) {
      case 'running':
        return { icon: '🟢', text: '稼働中' };
      case 'partial':
        return { icon: '🟡', text: '一部稼働' };
      case 'stopped':
        return { icon: '🔴', text: '停止中' };
      default:
        return { icon: '🔴', text: '要確認' };
    }
  });

  let scanHealthItems = $derived.by<MetricItem[]>(() => [
    {
      icon: '📈',
      label: '実測グループ',
      value: String(scanCycleHealthSummary.observedGroupCount),
    },
    {
      icon: '⚠️',
      label: '遅延グループ',
      value: String(scanCycleHealthSummary.delayedGroupCount),
      tone:
        scanCycleHealthSummary.delayedGroupCount > 0
          ? scanCycleHealthSummary.delayedGroupCount >= 3
            ? 'danger'
            : 'warn'
          : 'normal',
    },
    {
      icon: '📉',
      label: '平均乖離',
      value:
        scanCycleHealthSummary.avgDeltaRatio == null
          ? '-'
          : `${(scanCycleHealthSummary.avgDeltaRatio * 100).toFixed(1)}%`,
      tone:
        scanCycleHealthSummary.avgDeltaRatio == null
          ? 'normal'
          : scanCycleHealthSummary.avgDeltaRatio >= 0.5
            ? 'danger'
            : scanCycleHealthSummary.avgDeltaRatio >= 0.2
              ? 'warn'
              : 'normal',
    },
    {
      icon: '🐢',
      label: '最遅グループ',
      value:
        scanCycleHealthSummary.worstGroupLabel == null
          ? '-'
          : `${scanCycleHealthSummary.worstGroupLabel} (${((scanCycleHealthSummary.worstDeltaRatio ?? 0) * 100).toFixed(1)}%)`,
      tone:
        scanCycleHealthSummary.worstDeltaRatio == null
          ? 'normal'
          : scanCycleHealthSummary.worstDeltaRatio >= 0.7
            ? 'danger'
            : scanCycleHealthSummary.worstDeltaRatio >= 0.3
              ? 'warn'
              : 'normal',
    },
  ]);

  let runtimeInlineItems = $derived.by<MetricItem[]>(() => [
    {
      icon: runtimeBadge.icon,
      label: '全体状態',
      value: runtimeBadge.text,
      tone:
        runtimeOverview.tone === 'partial'
          ? 'warn'
          : runtimeOverview.tone === 'stopped' || runtimeOverview.tone === 'error'
            ? 'danger'
            : 'normal',
    },
    {
      icon: '⚙️',
      label: 'ドライバ',
      value: runtimeStatus.drivers_running ? '稼働中' : '停止中',
      tone: runtimeStatus.drivers_running ? 'normal' : 'warn',
    },
    {
      icon: '📡',
      label: 'MQTT',
      value: runtimeStatus.publishers_running ? '稼働中' : '停止中',
      tone: runtimeStatus.publishers_running ? 'normal' : 'warn',
    },
    ...scanHealthItems,
  ]);

  let runtimePrimaryItems = $derived.by<MetricItem[]>(() => runtimeInlineItems.slice(0, 3));
  let runtimeSecondaryItems = $derived.by<MetricItem[]>(() => runtimeInlineItems.slice(3));
</script>

<div class="content">
  <h2>ダッシュボード</h2>
  <div class="dashboard-grid">
    <div class="card summary-card">
      <span class="card-icon">🏷️</span>
      <div class="summary-body">
        <div class="summary-heading">タグ</div>
        <div class="summary-value-row">
          <p class="value">{tagCount}</p>
          <p class="sub">登録済み</p>
        </div>
      </div>
    </div>
    <div class="card summary-card">
      <span class="card-icon">⚙️</span>
      <div class="summary-body">
        <div class="summary-heading">ドライバ</div>
        <div class="summary-value-row">
          <p class="value">{driverCount}</p>
          <p class="sub">登録済み</p>
        </div>
      </div>
    </div>
    <div class="card summary-card">
      <span class="card-icon">⚡</span>
      <div class="summary-body">
        <div class="summary-heading">有効ドライバ</div>
        <div class="summary-value-row">
          <p class="value">{enabledDriverCount}</p>
          <p class="sub">利用可能</p>
        </div>
      </div>
    </div>
    <div class="card runtime-card runtime-overview-card">
      <div class="runtime-strip">
        <div class="runtime-strip-title">
          <span class="card-icon">🧩</span>
          <h3>アプリ稼働状態</h3>
        </div>
        <div class="runtime-strip-content">
          <div class="inline-list runtime-inline-list primary">
            {#each runtimePrimaryItems as item}
              <div class={`inline-item ${item.tone ?? 'normal'}`}>
                <span class="inline-icon" aria-hidden="true">{item.icon ?? '•'}</span>
                <span class="inline-label">{item.label}</span>
                <span class="inline-value">{item.value}</span>
              </div>
            {/each}
          </div>
          <div class="inline-list runtime-inline-list secondary">
            {#each runtimeSecondaryItems as item}
              <div class={`inline-item ${item.tone ?? 'normal'}`}>
                <span class="inline-icon" aria-hidden="true">{item.icon ?? '•'}</span>
                <span class="inline-label">{item.label}</span>
                <span class="inline-value">{item.value}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <div class="card runtime-card metrics-card app-metrics-card">
      <div class="runtime-strip metric-strip">
        <div class="runtime-strip-title">
          <span class="card-icon">📈</span>
          <h3>本体メトリクス</h3>
        </div>
        <div class="runtime-strip-content">
          <div class="inline-list metrics-inline-list">
            {#each appMetricItems as item}
              <div class={`inline-item metric-inline-item ${item.tone ?? 'normal'}`}>
                <span class="inline-icon" aria-hidden="true">{item.icon ?? '•'}</span>
                <span class="inline-label">{item.label}</span>
                <span class="inline-value">{item.value}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <div class="card runtime-card metrics-card system-metrics-card">
      <div class="runtime-strip metric-strip">
        <div class="runtime-strip-title">
          <span class="card-icon">🖥️</span>
          <h3>PC全体メトリクス</h3>
        </div>
        <div class="runtime-strip-content">
          <div class="inline-list metrics-inline-list">
            {#each systemMetricItems as item}
              <div class={`inline-item metric-inline-item ${item.tone ?? 'normal'}`}>
                <span class="inline-icon" aria-hidden="true">{item.icon ?? '•'}</span>
                <span class="inline-label">{item.label}</span>
                <span class="inline-value">{item.value}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <div class="card runtime-card driver-metrics-card">
      <span class="card-icon">🚚</span>
      <h3>通信ドライバ別 I/O 推定</h3>
      {#if driverMetrics.length === 0}
        <p class="sub">起動中の通信ドライバはありません</p>
      {:else}
        <table class="driver-metrics-table">
          <thead>
            <tr>
              <th>Driver</th>
              <th>PID</th>
              <th>CPU</th>
              <th>Memory</th>
              <th>I/O Read</th>
              <th>I/O Write</th>
            </tr>
          </thead>
          <tbody>
            {#each driverMetrics as metric (metric.driver_id)}
              <tr>
                <td>{metric.driver_id} ({metric.driver_type})</td>
                <td>{metric.pid}</td>
                <td>
                  <span class={`metric-badge ${cpuLevel(metric.cpu_percent)}`}>
                    {formatPercent(metric.cpu_percent)}
                  </span>
                </td>
                <td>{formatBytes(metric.memory_bytes)}</td>
                <td>
                  <span class={`metric-badge ${ioLevel(metric.network_rx_bytes_per_sec)}`}>
                    {formatByteRate(metric.network_rx_bytes_per_sec)}/s
                  </span>
                </td>
                <td>
                  <span class={`metric-badge ${ioLevel(metric.network_tx_bytes_per_sec)}`}>
                    {formatByteRate(metric.network_tx_bytes_per_sec)}/s
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      <p class="sub">※ CPU は Task Manager 風に 0〜100% へ正規化して表示</p>
      <p class="sub">※ I/O Read/Write は各通信ドライバが報告する累積送受信バイト差分（B/s）です</p>
      <p class="sub">※ ドライバ起動直後や初回サンプルでは 0 B/s になることがあります</p>
      <p class="sub">※ 色の目安: CPU 70%/90%、I/O 1MB/s / 10MB/s</p>
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
    <button class="btn-outline" onclick={onOpenMqttMonitor}>MQTT モニタを開く</button>
    <button
      class="btn-primary"
      onclick={onStartServers}
      disabled={runtimeBusy || runtimeStatus.drivers_running || runtimeStatus.publishers_running}
    >
      {runtimeBusy ? '実行中...' : 'ドライバ / MQTT 開始'}
    </button>
    <button
      class="btn-outline danger"
      onclick={onStopServers}
      disabled={runtimeBusy || (!runtimeStatus.drivers_running && !runtimeStatus.publishers_running)}
    >
      ドライバ / MQTT 停止
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
    grid-template-columns: repeat(3, minmax(240px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
    align-items: stretch;
  }

  .card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 20px 16px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
    min-width: 0;
  }

  .summary-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    text-align: left;
  }

  .card-icon {
    font-size: 1.8rem;
    flex: 0 0 auto;
  }

  .card h3 {
    margin: 8px 0 4px;
    font-size: 0.8rem;
    color: #7f8c8d;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .summary-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .summary-heading {
    color: #64748b;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .summary-value-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .value {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    color: #2e86c1;
  }

  .summary-card .value {
    font-size: 1.8rem;
    line-height: 1.1;
  }

  .sub {
    margin: 2px 0 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }

  .summary-card .sub {
    margin: 0;
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

  .runtime-overview-card {
    grid-column: 1 / -1;
  }

  .runtime-strip {
    display: grid;
    grid-template-columns: 122px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
  }

  .runtime-strip-title {
    display: flex;
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
      min-height: 38px;
  }

  .runtime-strip-title .card-icon {
    font-size: 1.35rem;
  }

  .runtime-strip-title h3 {
    margin: 0;
    font-size: 0.86rem;
    letter-spacing: 0.02em;
    line-height: 1.2;
  }

  .runtime-strip-content {
    min-width: 0;
  }

  .metric-strip {
    align-items: center;
  }

  .inline-list {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px 10px;
  }

  .runtime-inline-list {
    margin-top: 0;
    gap: 8px;
  }

  .runtime-inline-list.primary .inline-item {
    flex: 1 1 250px;
  }

  .runtime-inline-list.secondary {
    margin-top: 8px;
  }

  .runtime-inline-list.secondary .inline-item {
    flex: 1 1 220px;
  }

  .inline-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0.45rem 0.68rem;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    background: #f8fafc;
    min-height: 38px;
    min-width: 0;
    flex: 1 1 220px;
  }

  .inline-item.normal {
    border-color: #dbe2ea;
  }

  .inline-item.warn {
    border-color: #fcd34d;
    background: #fffbeb;
  }

  .inline-item.danger {
    border-color: #fca5a5;
    background: #fef2f2;
  }

  .inline-icon {
    font-size: 0.95rem;
    flex: 0 0 auto;
  }

  .inline-label {
    color: #64748b;
    font-size: 0.76rem;
    font-weight: 700;
    white-space: nowrap;
    flex: 0 0 auto;
  }

  .inline-value {
    color: #334155;
    font-size: 0.86rem;
    font-weight: 700;
    margin-left: auto;
    min-width: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .metrics-card,
  .driver-metrics-card {
    grid-column: 1 / -1;
  }

  .metrics-inline-list {
    margin-top: 0;
    display: grid;
    gap: 8px;
  }

  .app-metrics-card .metrics-inline-list {
    grid-template-columns: repeat(5, minmax(0, 1fr));
  }

  .system-metrics-card .metrics-inline-list {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .metrics-inline-list .inline-item {
    min-width: 0;
    min-height: 36px;
    padding: 0.4rem 0.58rem;
    gap: 6px;
  }

  .metrics-inline-list .inline-label {
    font-size: 0.72rem;
  }

  .metrics-inline-list .inline-value {
    font-size: 0.82rem;
  }

  .driver-metrics-card {
    grid-column: 1 / -1;
  }

  .driver-metrics-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
    color: #334155;
  }

  .driver-metrics-table th,
  .driver-metrics-table td {
    border-bottom: 1px solid #e2e8f0;
    padding: 6px 8px;
    text-align: left;
  }

  .driver-metrics-table th {
    color: #475569;
    font-weight: 600;
    background: #f8fafc;
  }

  .metric-badge {
    display: inline-block;
    min-width: 78px;
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    font-weight: 600;
    text-align: center;
  }

  .metric-badge.normal {
    color: #334155;
    background: #e2e8f0;
  }

  .metric-badge.warn {
    color: #92400e;
    background: #fef3c7;
  }

  .metric-badge.danger {
    color: #991b1b;
    background: #fee2e2;
  }

  @media (max-width: 1180px) {
    .dashboard-grid {
      grid-template-columns: repeat(2, minmax(240px, 1fr));
    }

    .runtime-strip {
      grid-template-columns: 1fr;
      gap: 6px;
    }

    .app-metrics-card .metrics-inline-list {
      grid-template-columns: repeat(5, minmax(0, 1fr));
    }

    .system-metrics-card .metrics-inline-list {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .dashboard-grid {
      grid-template-columns: 1fr;
    }

    .summary-card {
      align-items: flex-start;
    }

    .inline-item {
      min-height: 36px;
      padding: 0.4rem 0.62rem;
      flex-basis: 100%;
    }

    .runtime-inline-list.primary .inline-item,
    .runtime-inline-list.secondary .inline-item {
      flex-basis: 100%;
    }

    .app-metrics-card .metrics-inline-list,
    .system-metrics-card .metrics-inline-list {
      grid-template-columns: 1fr;
    }
  }
</style>

