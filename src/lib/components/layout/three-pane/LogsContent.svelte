<script lang="ts">
  import { clearAppLogs, listAppLogs } from '$lib/ipc';

  type LogSource = 'grpc' | 'mqtt' | 'driver' | 'main' | 'registration-ui' | 'other';
  type Destination = 'lifecycle' | 'other';
  type ErrorFocus = 'all' | 'driver' | 'mqtt' | 'grpc';

  type ParsedLogLine = {
    raw: string;
    cleaned: string;
    timestamp: string;
    level: string;
    target: string;
    message: string;
    source: LogSource;
    lifecycle: boolean;
  };

  let loading = $state(false);
  let message = $state('');
  let error = $state('');
  let lines = $state<string[]>([]);
  let destination = $state<Destination>('lifecycle');
  let lifecycleSource = $state<LogSource | 'all'>('all');
  let errorFocus = $state<ErrorFocus>('all');
  let keyword = $state('');

  function cleanLogText(raw: string): string {
    return raw
      .replace(/\u001b\[[0-9;]*m/g, '')
      .replace(/\uFFFD\[[0-9;]*m/g, '')
      .replace(/\s+/g, ' ')
      .trim();
  }

  function parseLogLine(raw: string): ParsedLogLine {
    const cleaned = cleanLogText(raw);
    const levelMatch = cleaned.match(/\b(TRACE|DEBUG|INFO|WARN|ERROR)\b/);
    const level = levelMatch?.[1] ?? 'INFO';
    const structuredMatch = cleaned.match(
      /^(\d{4}-\d{2}-\d{2}T[^\s]+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+([a-zA-Z0-9_:\-]+):\s*(.*)$/,
    );
    const fallbackMatch = cleaned.match(
      /\b(?:TRACE|DEBUG|INFO|WARN|ERROR)\s+([a-zA-Z0-9_:\-]+):\s*(.*)$/,
    );
    const timestamp = structuredMatch?.[1] ?? '-';
    const target = structuredMatch?.[3] ?? fallbackMatch?.[1] ?? 'unknown';
    const message = structuredMatch?.[4] ?? fallbackMatch?.[2] ?? cleaned;
    const lowerTarget = target.toLowerCase();
    const lowerMessage = message.toLowerCase();

    let source: LogSource = 'other';
    if (
      lowerTarget.includes('ui_launcher') ||
      lowerMessage.includes('driver ui process') ||
      lowerMessage.includes('launching driver ui')
    ) {
      source = 'registration-ui';
    } else if (lowerTarget.includes('grpc') || lowerMessage.includes('grpc')) {
      source = 'grpc';
    } else if (lowerTarget.includes('mqtt') || lowerMessage.includes('tagbus')) {
      source = 'mqtt';
    } else if (lowerTarget.includes('drivers') || lowerMessage.includes('driver process')) {
      source = 'driver';
    } else if (
      lowerMessage.includes('kt_iot_hub starting') ||
      lowerMessage.includes('graceful shutdown') ||
      lowerMessage.includes('exiting application') ||
      lowerTarget.includes('main')
    ) {
      source = 'main';
    }

    const lifecycle =
      /\b(start|starting|started|stop|stopping|stopped|launch|launched|shutdown|exiting|exited|terminate|terminated)\b/i.test(
        lowerMessage,
      ) ||
      /\b(start|stop|launch|shutdown|exit)\b/i.test(lowerTarget);

    return {
      raw,
      cleaned,
      timestamp,
      level,
      target,
      message,
      source,
      lifecycle,
    };
  }

  const parsedLines = $derived.by<ParsedLogLine[]>(() => lines.map(parseLogLine));

  const filteredLines = $derived.by<ParsedLogLine[]>(() => {
    const kw = keyword.trim().toLowerCase();
    return parsedLines.filter((line) => {
      const keywordMatched =
        kw.length === 0 ||
        line.cleaned.toLowerCase().includes(kw) ||
        line.message.toLowerCase().includes(kw) ||
        line.target.toLowerCase().includes(kw);
      if (!keywordMatched) {
        return false;
      }

      if (destination === 'lifecycle') {
        if (!line.lifecycle) {
          return false;
        }
        if (lifecycleSource !== 'all' && line.source !== lifecycleSource) {
          return false;
        }
        return true;
      }

      if (line.lifecycle) {
        return false;
      }
      if (errorFocus === 'all') {
        return true;
      }

      const isErrorLike = line.level === 'WARN' || line.level === 'ERROR';
      if (!isErrorLike) {
        return false;
      }

      if (errorFocus === 'driver') {
        return line.source === 'driver';
      }
      if (errorFocus === 'mqtt') {
        return line.source === 'mqtt';
      }
      if (errorFocus === 'grpc') {
        return line.source === 'grpc';
      }

      return true;
    });
  });

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
  <div class="header-row">
    <h2>ログ</h2>
    <div class="actions">
      <button
        class="btn-outline"
        class:active={destination === 'lifecycle'}
        onclick={() => {
          destination = 'lifecycle';
        }}
      >
        起動/停止ログ
      </button>
      <button
        class="btn-outline"
        class:active={destination === 'other'}
        onclick={() => {
          destination = 'other';
        }}
      >
        その他ログ
      </button>
      <button class="btn-outline" onclick={refreshLogs} disabled={loading}>再読込</button>
      <button class="btn-outline" onclick={copyLogs} disabled={lines.length === 0}>コピー</button>
      <button class="btn-outline danger" onclick={clearLogs} disabled={loading}>クリア</button>
    </div>
  </div>

  <div class="filters">
    <input
      class="filter-input"
      placeholder="キーワード検索"
      bind:value={keyword}
    />
    {#if destination === 'lifecycle'}
      <div class="chips">
        <button class="chip" class:active={lifecycleSource === 'all'} onclick={() => (lifecycleSource = 'all')}>全体</button>
        <button class="chip" class:active={lifecycleSource === 'main'} onclick={() => (lifecycleSource = 'main')}>本体</button>
        <button class="chip" class:active={lifecycleSource === 'grpc'} onclick={() => (lifecycleSource = 'grpc')}>gRPC</button>
        <button class="chip" class:active={lifecycleSource === 'mqtt'} onclick={() => (lifecycleSource = 'mqtt')}>MQTT</button>
        <button class="chip" class:active={lifecycleSource === 'driver'} onclick={() => (lifecycleSource = 'driver')}>通信ドライバ</button>
        <button class="chip" class:active={lifecycleSource === 'registration-ui'} onclick={() => (lifecycleSource = 'registration-ui')}>登録UI</button>
      </div>
    {:else}
      <div class="chips">
        <button class="chip" class:active={errorFocus === 'all'} onclick={() => (errorFocus = 'all')}>全ログ</button>
        <button class="chip" class:active={errorFocus === 'driver'} onclick={() => (errorFocus = 'driver')}>ドライバエラー</button>
        <button class="chip" class:active={errorFocus === 'mqtt'} onclick={() => (errorFocus = 'mqtt')}>MQTTエラー</button>
        <button class="chip" class:active={errorFocus === 'grpc'} onclick={() => (errorFocus = 'grpc')}>gRPCエラー</button>
      </div>
    {/if}
  </div>

  {#if message}
    <p class="action-message">{message}</p>
  {/if}
  {#if error}
    <p class="error-message">{error}</p>
  {/if}

  <div class="log-panel" role="log" aria-live="polite">
    {#if filteredLines.length === 0}
      <p class="empty">ログはまだありません。</p>
    {:else}
      {#each filteredLines as line (line.raw + line.target + line.message)}
        <div class="log-line" data-level={line.level}>
          <span class="time">{line.timestamp}</span>
          <span class="tag level">{line.level}</span>
          <span class="text" title={line.message}>{line.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .content h2 {
    margin: 0;
    font-size: 1.1rem;
    color: #2c3e50;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .filters {
    display: grid;
    gap: 8px;
    margin-bottom: 10px;
  }

  .filter-input {
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 0.85rem;
  }

  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .chip {
    border: 1px solid #cbd5e1;
    background: #fff;
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
  }

  .chip.active {
    border-color: #2e86c1;
    color: #1d4ed8;
    background: #eff6ff;
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

  .btn-outline.active {
    background: #eff6ff;
    border-color: #1d4ed8;
    color: #1d4ed8;
  }

  .btn-outline.danger {
    color: #b91c1c;
    border-color: #fca5a5;
  }

  .btn-outline.danger:hover {
    background: #fef2f2;
  }

  .btn-outline:disabled {
    cursor: not-allowed;
    opacity: 0.55;
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
