<script lang="ts">
  import type { DriverMetricsDto } from '$lib/ipc';
  import { cpuLevel, formatByteRate, formatBytes, formatPercent, ioLevel } from '$lib/utils/format';

  let { driverMetrics }: { driverMetrics: DriverMetricsDto[] } = $props();
</script>

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

<style>
  .card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 20px 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
    min-width: 0;
  }

  .runtime-card {
    text-align: left;
  }

  .driver-metrics-card {
    grid-column: 1 / -1;
  }

  .card-icon {
    font-size: 1.35rem;
  }

  h3 {
    margin: 8px 0 4px;
    font-size: 0.8rem;
    color: #7f8c8d;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
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

  .sub {
    margin: 2px 0 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }
</style>
