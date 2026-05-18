<script lang="ts">
  import { clearAppLogs, listAppLogs } from '$lib/ipc';
  import LogFilterBar from './logs/LogFilterBar.svelte';
  import LogTable from './logs/LogTable.svelte';
  import {
    filterLogLines,
    parseLogLine,
    type Destination,
    type ErrorFocus,
    type LogSource,
    type ParsedLogLine,
  } from './logs/logFilters';

  let loading = $state(false);
  let message = $state('');
  let error = $state('');
  let lines = $state<string[]>([]);
  let destination = $state<Destination>('lifecycle');
  let lifecycleSource = $state<LogSource | 'all'>('all');
  let errorFocus = $state<ErrorFocus>('all');
  let keyword = $state('');

  const parsedLines = $derived.by<ParsedLogLine[]>(() => lines.map(parseLogLine));

  const filteredLines = $derived.by<ParsedLogLine[]>(() =>
    filterLogLines(parsedLines, {
      destination,
      lifecycleSource,
      errorFocus,
      keyword,
    }),
  );

  async function refreshLogs() {
    loading = true;
    error = '';
    try {
      lines = await listAppLogs(1000);
      message = `最新 ${lines.length} 行を取得 (${filteredLines.length} 行を表示中)`;
    } catch (e) {
      error = e instanceof Error ? e.message : 'ログ取得に失敗しました';
    } finally {
      loading = false;
    }
  }

  async function clearLogs() {
    loading = true;
    error = '';
    try {
      await clearAppLogs();
      lines = [];
      message = 'ログをクリアしました';
    } catch (e) {
      error = e instanceof Error ? e.message : 'ログクリアに失敗しました';
    } finally {
      loading = false;
    }
  }

  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(filteredLines.map((line) => line.cleaned).join('\n'));
      message = 'ログをクリップボードへコピーしました';
      error = '';
    } catch {
      error = 'クリップボードへのコピーに失敗しました';
    }
  }

  $effect(() => {
    let disposed = false;

    const tick = async () => {
      if (disposed || loading) {
        return;
      }
      await refreshLogs();
    };

    void tick();
    const timerId = window.setInterval(() => {
      void tick();
    }, 2000);

    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  });
</script>

<div class="content">
  <LogFilterBar
    {destination}
    {lifecycleSource}
    {errorFocus}
    {keyword}
    {loading}
    hasLines={lines.length > 0}
    onDestinationChange={(next) => {
      destination = next;
    }}
    onLifecycleSourceChange={(next) => {
      lifecycleSource = next;
    }}
    onErrorFocusChange={(next) => {
      errorFocus = next;
    }}
    onKeywordChange={(next) => {
      keyword = next;
    }}
    onRefresh={refreshLogs}
    onCopy={copyLogs}
    onClear={clearLogs}
  />

  {#if message}
    <p class="action-message">{message}</p>
  {/if}
  {#if error}
    <p class="error-message">{error}</p>
  {/if}

  <LogTable lines={filteredLines} />
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .action-message {
    margin: 0 0 10px;
    font-size: 0.82rem;
    color: #2563eb;
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .error-message {
    margin: 0 0 10px;
    font-size: 0.82rem;
    color: #b91c1c;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 10px;
  }

</style>
