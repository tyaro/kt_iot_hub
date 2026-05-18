<script lang="ts">
  import type { ScanGroupDto } from '$lib/ipc';

  let {
    scanGroup,
    isExpanded,
    isSelected,
    onToggle,
    onSelect,
  }: {
    scanGroup: ScanGroupDto;
    isExpanded: boolean;
    isSelected: boolean;
    onToggle: () => void;
    onSelect: () => void;
  } = $props();

  function formatCycleSummary(item: ScanGroupDto) {
    const configured = item.scan_rate_ms;
    const observed = item.observed_cycle_ms;
    if (!configured && !observed) {
      return '周期: -';
    }
    if (configured && !observed) {
      return `設定 ${configured}ms / 実測 -`;
    }
    if (!configured && observed) {
      return `実測 ${observed}ms`;
    }
    return `設定 ${configured}ms / 実測 ${observed}ms`;
  }

  function formatDeltaRatio(item: ScanGroupDto) {
    if (item.cycle_delta_ratio == null) {
      return null;
    }
    return `乖離 ${(item.cycle_delta_ratio * 100).toFixed(1)}%`;
  }

  const deltaLabel = $derived(formatDeltaRatio(scanGroup));
</script>

<div class="tree-node scan-group-node">
  <div class="indent-1">
    <button
      class="tree-toggle"
      onclick={onToggle}
      title={isExpanded ? '折畳む' : '展開'}
    >
      {isExpanded ? '▼' : '▶'}
    </button>
    <button
      type="button"
      class="tree-label tree-label-btn scan-group-label"
      class:selected={isSelected}
      onclick={onSelect}
    >
      📊 {scanGroup.id}
    </button>
    <span class="scan-meta">{formatCycleSummary(scanGroup)}</span>
    {#if deltaLabel}
      <span class={`scan-badge ${scanGroup.cycle_status ?? 'unknown'}`}>{deltaLabel}</span>
    {/if}
  </div>
</div>

<style>
  .tree-node {
    user-select: none;
    padding: 0.25rem 0;
  }

  .scan-group-node {
    background: #fafafa;
  }

  .indent-1 {
    display: flex;
    align-items: center;
    padding-left: 1.5rem;
  }

  .tree-toggle {
    width: 20px;
    height: 20px;
    padding: 0;
    margin: 0 0.25rem;
    border: none;
    background: none;
    cursor: pointer;
    color: #666;
    font-size: 0.75rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .tree-toggle:hover {
    color: #333;
  }

  .tree-label {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.4rem;
    cursor: pointer;
  }

  .tree-label-btn {
    border: none;
    background: transparent;
    font: inherit;
  }

  .scan-group-label {
    border: none;
    background: transparent;
    font-weight: 500;
    color: #555;
  }

  .scan-meta {
    margin-left: 0.35rem;
    color: #64748b;
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .scan-badge {
    margin-left: 0.35rem;
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .scan-badge.ok {
    color: #166534;
    background: #dcfce7;
  }

  .scan-badge.warn {
    color: #92400e;
    background: #fef3c7;
  }

  .scan-badge.danger {
    color: #991b1b;
    background: #fee2e2;
  }

  .scan-badge.unknown {
    color: #334155;
    background: #e2e8f0;
  }

  .tree-label-btn.selected {
    background: #dbeafe;
    border-radius: 4px;
  }
</style>
