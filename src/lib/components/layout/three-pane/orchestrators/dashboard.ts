import type { AppMetricsDto, DriverMetricsDto, RuntimeStatusDto, ScanGroupDto } from '$lib/ipc/index';

export type ScanCycleHealthSummary = {
  observedGroupCount: number;
  delayedGroupCount: number;
  avgDeltaRatio: number | null;
  worstGroupLabel: string | null;
  worstDeltaRatio: number | null;
};

export type DashboardMetrics = AppMetricsDto & {
  webview_memory_used_bytes: number | null;
  webview_memory_total_bytes: number | null;
  webview_memory_limit_bytes: number | null;
};

export function createInitialDashboardMetrics(): DashboardMetrics {
  return {
    sampled_at: new Date().toISOString(),
    process_cpu_percent: null,
    process_memory_bytes: null,
    system_cpu_percent: null,
    system_memory_used_bytes: null,
    system_memory_total_bytes: null,
    network_rx_bytes_per_sec: null,
    network_tx_bytes_per_sec: null,
    webview_memory_used_bytes: null,
    webview_memory_total_bytes: null,
    webview_memory_limit_bytes: null,
  };
}

export function computeScanCycleHealthSummary(scanGroups: ScanGroupDto[]): ScanCycleHealthSummary {
  const observed = scanGroups.filter((group) => group.cycle_delta_ratio != null);
  if (observed.length === 0) {
    return {
      observedGroupCount: 0,
      delayedGroupCount: 0,
      avgDeltaRatio: null,
      worstGroupLabel: null,
      worstDeltaRatio: null,
    };
  }

  const delayed = observed.filter((group) => (group.cycle_delta_ratio ?? 0) > 0.30);
  let worst = observed[0];
  for (const item of observed) {
    if ((item.cycle_delta_ratio ?? 0) > (worst.cycle_delta_ratio ?? 0)) {
      worst = item;
    }
  }
  const avgDeltaRatio =
    observed.reduce((sum, group) => sum + (group.cycle_delta_ratio ?? 0), 0) / observed.length;

  return {
    observedGroupCount: observed.length,
    delayedGroupCount: delayed.length,
    avgDeltaRatio,
    worstGroupLabel: `${worst.driver_id} / ${worst.id}`,
    worstDeltaRatio: worst.cycle_delta_ratio ?? null,
  };
}

type RuntimeRefresh = (getRuntimeStatus: () => Promise<RuntimeStatusDto>) => Promise<void>;
type MetricsRefreshParams = {
  currentPage: 'dashboard' | 'tags' | 'drivers' | 'publishers' | 'logs' | 'settings';
  reloadScanGroups: () => Promise<void>;
  refreshRuntimeStatus: RuntimeRefresh;
  getRuntimeStatus: () => Promise<RuntimeStatusDto>;
  getAppMetrics: () => Promise<AppMetricsDto>;
  getDriverMetrics: () => Promise<DriverMetricsDto[]>;
  setAppMetrics: (metrics: DashboardMetrics) => void;
  setDriverMetrics: (metrics: DriverMetricsDto[]) => void;
};

export async function refreshDashboardMetrics(params: MetricsRefreshParams): Promise<void> {
  if (params.currentPage === 'dashboard' || params.currentPage === 'tags') {
    await params.reloadScanGroups();
  }

  await params.refreshRuntimeStatus(params.getRuntimeStatus);

  try {
    const metrics = await params.getAppMetrics();
    const dMetrics = await params.getDriverMetrics();
    const perf = (
      globalThis.performance as unknown as {
        memory?: {
          usedJSHeapSize?: number;
          totalJSHeapSize?: number;
          jsHeapSizeLimit?: number;
        };
      }
    ).memory;

    params.setAppMetrics({
      ...metrics,
      webview_memory_used_bytes: perf?.usedJSHeapSize ?? null,
      webview_memory_total_bytes: perf?.totalJSHeapSize ?? null,
      webview_memory_limit_bytes: perf?.jsHeapSizeLimit ?? null,
    });
    params.setDriverMetrics(dMetrics);
  } catch {
    // ダッシュボード表示に影響しないよう、メトリクス取得失敗は握りつぶす
  }
}
