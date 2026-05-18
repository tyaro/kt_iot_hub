<script lang="ts">
  import type { MetricItem } from './types';

  let {
    icon,
    title,
    items,
    columns,
  }: { icon: string; title: string; items: MetricItem[]; columns: 4 | 5 } = $props();
</script>

<div class="card runtime-card metrics-card">
  <div class="runtime-strip metric-strip">
    <div class="runtime-strip-title">
      <span class="card-icon">{icon}</span>
      <h3>{title}</h3>
    </div>
    <div class="runtime-strip-content">
      <div class="inline-list metrics-inline-list" style={`grid-template-columns: repeat(${columns}, minmax(0, 1fr));`}>
        {#each items as item}
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

  .metrics-card {
    grid-column: 1 / -1;
  }

  .runtime-strip {
    display: grid;
    grid-template-columns: 122px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
  }

  .runtime-strip-title {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-height: 38px;
  }

  .card-icon {
    font-size: 1.35rem;
  }

  h3 {
    margin: 0;
    font-size: 0.86rem;
    letter-spacing: 0.02em;
    line-height: 1.2;
    color: #7f8c8d;
    font-weight: 600;
    text-transform: uppercase;
  }

  .runtime-strip-content {
    min-width: 0;
  }

  .metrics-inline-list {
    margin-top: 0;
    display: grid;
    gap: 8px;
  }

  .inline-item {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 36px;
    padding: 0.4rem 0.58rem;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    background: #f8fafc;
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
    font-size: 0.72rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .inline-value {
    color: #334155;
    font-size: 0.82rem;
    font-weight: 700;
    margin-left: auto;
    min-width: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 1180px) {
    .runtime-strip {
      grid-template-columns: 1fr;
      gap: 6px;
    }
  }

  @media (max-width: 760px) {
    .metrics-inline-list {
      grid-template-columns: 1fr !important;
    }
  }
</style>
