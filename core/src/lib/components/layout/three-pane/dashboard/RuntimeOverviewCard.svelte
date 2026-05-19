<script lang="ts">
  import type { MetricItem } from './types';

  let {
    primaryItems,
    secondaryItems,
  }: { primaryItems: MetricItem[]; secondaryItems: MetricItem[] } = $props();
</script>

<div class="card runtime-card runtime-overview-card">
  <div class="runtime-strip">
    <div class="runtime-strip-title">
      <span class="card-icon">🧩</span>
      <h3>アプリ稼働状態</h3>
    </div>
    <div class="runtime-strip-content">
      <div class="inline-list runtime-inline-list primary">
        {#each primaryItems as item}
          <div class={`inline-item ${item.tone ?? 'normal'}`}>
            <span class="inline-icon" aria-hidden="true">{item.icon ?? '•'}</span>
            <span class="inline-label">{item.label}</span>
            <span class="inline-value">{item.value}</span>
          </div>
        {/each}
      </div>
      <div class="inline-list runtime-inline-list secondary">
        {#each secondaryItems as item}
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

  .runtime-overview-card {
    grid-column: 1 / -1;
  }

  .card-icon {
    font-size: 1.35rem;
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

  .inline-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .runtime-inline-list.secondary {
    margin-top: 8px;
  }

  .runtime-inline-list.primary .inline-item {
    flex: 1 1 250px;
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

  @media (max-width: 1180px) {
    .runtime-strip {
      grid-template-columns: 1fr;
      gap: 6px;
    }
  }

  @media (max-width: 760px) {
    .inline-item,
    .runtime-inline-list.primary .inline-item,
    .runtime-inline-list.secondary .inline-item {
      flex-basis: 100%;
    }
  }
</style>
