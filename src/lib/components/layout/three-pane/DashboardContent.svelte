<script lang="ts">
  import type { AppMetricsDto, DriverMetricsDto, RuntimeStatusDto } from '$lib/ipc';
  import { cpuLevel, formatBytes, formatPercent, ioLevel } from '$lib/utils/format';
  import SummaryCard from './dashboard/SummaryCard.svelte';
  import RuntimeOverviewCard from './dashboard/RuntimeOverviewCard.svelte';
  import MetricsCard from './dashboard/MetricsCard.svelte';
  import DriverMetricsCard from './dashboard/DriverMetricsCard.svelte';
  import QuickLinks from './dashboard/QuickLinks.svelte';
  import type { MetricItem, ScanCycleHealthSummary } from './dashboard/types';

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
    scanCycleHealthSummary: ScanCycleHealthSummary;
    onNavigateTags: () => void;
    onOpenMqttMonitor: () => void;
    onStartServers: () => void;
    onStopServers: () => void;
  } = $props();

  let appMetricItems = $derived.by<MetricItem[]>(() => [
    { icon: '🧠', label: 'CPU(プロセス)', value: formatPercent(appMetrics.process_cpu_percent), tone: cpuLevel(appMetrics.process_cpu_percent) },
    { icon: '💾', label: 'メモリ(プロセス)', value: formatBytes(appMetrics.process_memory_bytes) },
    { icon: '🌐', label: 'WebView使用量', value: formatBytes(appMetrics.webview_memory_used_bytes) },
    { icon: '📦', label: 'WebView確保量', value: formatBytes(appMetrics.webview_memory_total_bytes) },
    { icon: '🧱', label: 'WebView上限', value: formatBytes(appMetrics.webview_memory_limit_bytes) },
  ]);

  let systemMetricItems = $derived.by<MetricItem[]>(() => [
    { icon: '🧠', label: 'CPU(システム)', value: formatPercent(appMetrics.system_cpu_percent), tone: cpuLevel(appMetrics.system_cpu_percent) },
    { icon: '💽', label: 'メモリ(システム)', value: `${formatBytes(appMetrics.system_memory_used_bytes)} / ${formatBytes(appMetrics.system_memory_total_bytes)}` },
    { icon: '📥', label: 'ネット受信', value: `${formatBytes(appMetrics.network_rx_bytes_per_sec)}/s`, tone: ioLevel(appMetrics.network_rx_bytes_per_sec) },
    { icon: '📤', label: 'ネット送信', value: `${formatBytes(appMetrics.network_tx_bytes_per_sec)}/s`, tone: ioLevel(appMetrics.network_tx_bytes_per_sec) },
  ]);

  let runtimeOverview = $derived.by<{ tone: RuntimeStateTone }>(() => {
    if (runtimeStatus.last_error) return { tone: 'error' };
    if (runtimeStatus.drivers_running && runtimeStatus.publishers_running) return { tone: 'running' };
    if (runtimeStatus.drivers_running || runtimeStatus.publishers_running) return { tone: 'partial' };
    return { tone: 'stopped' };
  });

  let runtimeBadge = $derived.by(() => {
    switch (runtimeOverview.tone) {
      case 'running': return { icon: '🟢', text: '稼働中' };
      case 'partial': return { icon: '🟡', text: '一部稼働' };
      case 'stopped': return { icon: '🔴', text: '停止中' };
      default: return { icon: '🔴', text: '要確認' };
    }
  });

  let scanHealthItems = $derived.by<MetricItem[]>(() => [
    { icon: '📈', label: '実測グループ', value: String(scanCycleHealthSummary.observedGroupCount) },
    { icon: '⚠️', label: '遅延グループ', value: String(scanCycleHealthSummary.delayedGroupCount), tone: scanCycleHealthSummary.delayedGroupCount > 0 ? (scanCycleHealthSummary.delayedGroupCount >= 3 ? 'danger' : 'warn') : 'normal' },
    { icon: '📉', label: '平均乖離', value: scanCycleHealthSummary.avgDeltaRatio == null ? '-' : `${(scanCycleHealthSummary.avgDeltaRatio * 100).toFixed(1)}%`, tone: scanCycleHealthSummary.avgDeltaRatio == null ? 'normal' : scanCycleHealthSummary.avgDeltaRatio >= 0.5 ? 'danger' : scanCycleHealthSummary.avgDeltaRatio >= 0.2 ? 'warn' : 'normal' },
    { icon: '🐢', label: '最遅グループ', value: scanCycleHealthSummary.worstGroupLabel == null ? '-' : `${scanCycleHealthSummary.worstGroupLabel} (${((scanCycleHealthSummary.worstDeltaRatio ?? 0) * 100).toFixed(1)}%)`, tone: scanCycleHealthSummary.worstDeltaRatio == null ? 'normal' : scanCycleHealthSummary.worstDeltaRatio >= 0.7 ? 'danger' : scanCycleHealthSummary.worstDeltaRatio >= 0.3 ? 'warn' : 'normal' },
  ]);

  let runtimeInlineItems = $derived.by<MetricItem[]>(() => [
    {
      icon: runtimeBadge.icon,
      label: '全体状態',
      value: runtimeBadge.text,
      tone: runtimeOverview.tone === 'partial' ? 'warn' : runtimeOverview.tone === 'stopped' || runtimeOverview.tone === 'error' ? 'danger' : 'normal',
    },
    { icon: '⚙️', label: 'ドライバ', value: runtimeStatus.drivers_running ? '稼働中' : '停止中', tone: runtimeStatus.drivers_running ? 'normal' : 'warn' },
    { icon: '📡', label: 'MQTT', value: runtimeStatus.publishers_running ? '稼働中' : '停止中', tone: runtimeStatus.publishers_running ? 'normal' : 'warn' },
    ...scanHealthItems,
  ]);

  let runtimePrimaryItems = $derived.by<MetricItem[]>(() => runtimeInlineItems.slice(0, 3));
  let runtimeSecondaryItems = $derived.by<MetricItem[]>(() => runtimeInlineItems.slice(3));
</script>

<div class="content">
  <h2>ダッシュボード</h2>
  <div class="dashboard-grid">
    <SummaryCard icon="🏷️" heading="タグ" value={tagCount} sub="登録済み" />
    <SummaryCard icon="⚙️" heading="ドライバ" value={driverCount} sub="登録済み" />
    <SummaryCard icon="⚡" heading="有効ドライバ" value={enabledDriverCount} sub="利用可能" />
    <RuntimeOverviewCard primaryItems={runtimePrimaryItems} secondaryItems={runtimeSecondaryItems} />
    <MetricsCard icon="📈" title="本体メトリクス" items={appMetricItems} columns={5} />
    <MetricsCard icon="🖥️" title="PC全体メトリクス" items={systemMetricItems} columns={4} />
    <DriverMetricsCard {driverMetrics} />
  </div>

  {#if dashboardMessage}
    <p class="action-message">{dashboardMessage}</p>
  {/if}
  {#if runtimeStatus.last_error}
    <p class="error-message">{runtimeStatus.last_error}</p>
  {/if}

  <QuickLinks
    runtimeBusy={runtimeBusy}
    driversRunning={runtimeStatus.drivers_running}
    publishersRunning={runtimeStatus.publishers_running}
    {onNavigateTags}
    {onOpenMqttMonitor}
    {onStartServers}
    {onStopServers}
  />
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

  @media (max-width: 1180px) {
    .dashboard-grid {
      grid-template-columns: repeat(2, minmax(240px, 1fr));
    }
  }

  @media (max-width: 760px) {
    .dashboard-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
