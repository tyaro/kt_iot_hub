<script lang="ts">
  import type { ParsedLogLine } from './logFilters';

  let { lines }: { lines: ParsedLogLine[] } = $props();
</script>

<div class="log-panel" role="log" aria-live="polite">
  {#if lines.length === 0}
    <p class="empty">ログはまだありません。</p>
  {:else}
    {#each lines as line (line.raw + line.target + line.message)}
      <div class="log-line" data-level={line.level}>
        <span class="time">{line.timestamp}</span>
        <span class="tag level">{line.level}</span>
        <span class="text" title={line.message}>{line.message}</span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .log-panel {
    border: 1px solid #dbe2ea;
    background: #0f172a;
    color: #e2e8f0;
    border-radius: 8px;
    height: calc(100vh - 230px);
    min-height: 300px;
    overflow: auto;
    padding: 10px;
  }

  .log-line {
    display: grid;
    grid-template-columns: max-content auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    font-family: Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.4;
    padding: 4px 0;
    border-bottom: 1px solid rgba(148, 163, 184, 0.15);
  }

  .time {
    color: #94a3b8;
    white-space: nowrap;
  }

  .log-line[data-level='ERROR'] .text {
    color: #fecaca;
  }

  .log-line[data-level='WARN'] .text {
    color: #fde68a;
  }

  .tag {
    border-radius: 4px;
    padding: 0 6px;
    font-size: 10px;
    font-weight: 700;
  }

  .tag.level {
    background: #334155;
    color: #e2e8f0;
  }

  .text {
    color: #e2e8f0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: 0;
    color: #94a3b8;
    font-size: 0.85rem;
  }
</style>
