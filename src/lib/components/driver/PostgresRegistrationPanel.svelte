<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    listDrivers,
    postgresListColumns,
    postgresListTables,
    saveDriverUiOutput,
    postgresTestConnection,
    type PostgresColumnDto,
    type PostgresConnectionParams,
    type DriverUiLaunchContextDto,
    type PostgresTableDto,
  } from '$lib/ipc/index';

  interface Props {
    context: DriverUiLaunchContextDto;
  }

  let { context }: Props = $props();

  // ---- 接続フォーム ----
  // context.driverId はフォームの初期値として一度だけ使用するため意図的にキャプチャ
  let driverIdInput = $state(untrack(() => context.driverId ?? ''));
  let host = $state('127.0.0.1');
  let portInput = $state('5432');
  let database = $state('');
  let username = $state('');
  let password = $state('');

  let conn = $derived<PostgresConnectionParams>({
    host,
    port: parseInt(portInput, 10) || 5432,
    database,
    username,
    password,
  });

  let testing = $state(false);
  let testMessage = $state('');
  let testError = $state('');

  let loadingTables = $state(false);
  let tableError = $state('');
  let tables = $state<PostgresTableDto[]>([]);
  let selectedTableKey = $state('');

  let scanGroupId = $state('');
  let scanRateMs = $state(1000);

  let loadingColumns = $state(false);
  let columnError = $state('');
  let columns = $state<PostgresColumnDto[]>([]);
  let timestampColumn = $state('');
  let selectedFields = $state<string[]>([]);
  let exporting = $state(false);
  let exportMessage = $state('');
  let exportError = $state('');

  onMount(async () => {
    // 既存ドライバ編集の場合は接続情報を pre-fill する
    if (context.driverId) {
      try {
        const drivers = await listDrivers();
        const existing = drivers.find((d) => d.id === context.driverId);
        if (existing) {
          driverIdInput = existing.id;
          host = existing.host;
          portInput = String(existing.port);
          database = existing.database;
          username = existing.username;
        }
      } catch {
        // pre-fill 失敗は無視して空フォームで続行
      }
    }
  });

  function selectedTable() {
    return tables.find((item) => `${item.schema}.${item.name}` === selectedTableKey) ?? null;
  }

  async function testConnection() {
    testing = true;
    testMessage = '';
    testError = '';
    try {
      const result = await postgresTestConnection(conn);
      testMessage = result.message;
    } catch (error) {
      testError = error instanceof Error ? error.message : '接続テストに失敗しました';
    } finally {
      testing = false;
    }
  }

  async function loadTables() {
    loadingTables = true;
    tableError = '';
    tables = [];
    selectedTableKey = '';
    columns = [];
    selectedFields = [];
    timestampColumn = '';
    try {
      tables = await postgresListTables(conn);
    } catch (error) {
      tableError = error instanceof Error ? error.message : 'テーブル取得に失敗しました';
    } finally {
      loadingTables = false;
    }
  }

  async function onTableChanged() {
    columns = [];
    columnError = '';
    selectedFields = [];
    timestampColumn = '';
    const table = selectedTable();
    if (!table) return;

    if (!scanGroupId.trim()) {
      scanGroupId = `${table.name}_${scanRateMs}ms`;
    }

    loadingColumns = true;
    try {
      columns = await postgresListColumns({
        conn,
        schema: table.schema,
        table: table.name,
      });
    } catch (error) {
      columnError = error instanceof Error ? error.message : 'フィールド取得に失敗しました';
    } finally {
      loadingColumns = false;
    }
  }

  function toggleField(fieldName: string, checked: boolean) {
    if (checked) {
      if (!selectedFields.includes(fieldName)) {
        selectedFields = [...selectedFields, fieldName];
      }
      return;
    }
    selectedFields = selectedFields.filter((item) => item !== fieldName);
  }

  function mapPgTypeToTagType(dataType: string): string {
    const t = dataType.toLowerCase();
    if (t.includes('bool')) return 'bool';
    if (t.includes('int2') || t.includes('smallint') || t.includes('int4') || t.includes('integer')) return 'i32';
    if (t.includes('int8') || t.includes('bigint')) return 'i64';
    if (t.includes('real') || t.includes('float4')) return 'f32';
    if (t.includes('double') || t.includes('float8') || t.includes('numeric') || t.includes('decimal')) return 'f64';
    return 'string';
  }

  function normalizeId(raw: string): string {
    return raw
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '');
  }

  function buildResponsePayload(): Record<string, unknown> {
    const table = selectedTable();
    if (!table) throw new Error('テーブルを選択してください');
    if (!driverIdInput.trim()) throw new Error('接続先IDを入力してください');
    if (!scanGroupId.trim()) throw new Error('周期グループIDを入力してください');
    if (!timestampColumn.trim()) throw new Error('時系列フィールドを選択してください');
    if (selectedFields.length === 0) throw new Error('タグ化するフィールドを1つ以上選択してください');

    const normalizedScanGroupId = scanGroupId.trim();
    const tags = selectedFields.map((fieldName) => {
      const column = columns.find((item) => item.name === fieldName);
      const dataType = mapPgTypeToTagType(column?.data_type ?? 'text');
      const id = normalizeId(`tag-${driverIdInput.trim()}-${normalizedScanGroupId}-${fieldName}`);
      return {
        id,
        name: fieldName,
        dataType,
        enabled: true,
        driverSpec: {
          kind: 'postgres',
          scanGroup: normalizedScanGroupId,
          schema: table.schema,
          table: table.name,
          timestampColumn,
          valueColumn: fieldName,
        },
      };
    });

    return {
      schemaVersion: 1,
      requestId: context.requestId ?? `req-${Date.now()}`,
      generatedAt: new Date().toISOString(),
      direction: 'driver-to-host',
      driver: {
        id: driverIdInput.trim(),
        driverType: 'postgres',
        enabled: true,
        settings: {
          host: conn.host,
          port: conn.port,
          database: conn.database,
          username: conn.username,
          password: conn.password,
        },
      },
      scanGroups: [{ id: normalizedScanGroupId, scanRateMs, schema: table.schema, table: table.name, timestampColumn }],
      tags,
    };
  }

  async function confirmAndClose() {
    exporting = true;
    exportMessage = '';
    exportError = '';
    try {
      const payload = buildResponsePayload();
      await saveDriverUiOutput({
        outputJsonPath: context.outputJsonPath ?? null,
        payload,
      });
      exportMessage = '確定しました。ウィンドウを閉じます...';
      await getCurrentWindow().close();
    } catch (error) {
      exportError = error instanceof Error ? error.message : '確定JSONの保存に失敗しました';
    } finally {
      exporting = false;
    }
  }

  let canConfirm = $derived(
    !exporting &&
      driverIdInput.trim().length > 0 &&
      selectedTableKey.length > 0 &&
      scanGroupId.trim().length > 0 &&
      timestampColumn.trim().length > 0 &&
      selectedFields.length > 0,
  );
</script>

<section class="panel">
  <h4>PostgreSQL 登録UI</h4>
  <p class="desc">接続情報 → テスト接続 → テーブル選択 → フィールド選択 → 確定 の順で設定します。</p>

  <!-- 接続情報フォーム -->
  <div class="grid2">
    <label>
      接続先ID
      <input bind:value={driverIdInput} placeholder="pg_main" />
    </label>
    <label>
      ホスト
      <input bind:value={host} placeholder="127.0.0.1" />
    </label>
  </div>
  <div class="grid3">
    <label>
      ポート
      <input type="number" min="1" max="65535" bind:value={portInput} />
    </label>
    <label>
      データベース
      <input bind:value={database} placeholder="mydb" />
    </label>
    <label>
      ユーザー名
      <input bind:value={username} placeholder="postgres" />
    </label>
  </div>
  <label>
    パスワード
    <input type="password" bind:value={password} placeholder="(任意)" />
  </label>

  <div class="row">
    <button class="btn" onclick={testConnection} disabled={testing}>
      {testing ? '接続テスト中...' : 'テスト接続'}
    </button>
    <button class="btn secondary" onclick={loadTables} disabled={loadingTables}>
      {loadingTables ? 'テーブル取得中...' : 'テーブル再取得'}
    </button>
  </div>
  {#if testMessage}<p class="ok">{testMessage}</p>{/if}
  {#if testError}<p class="error">{testError}</p>{/if}
  {#if tableError}<p class="error">{tableError}</p>{/if}

  <div class="grid2">
    <label>
      周期グループID
      <input bind:value={scanGroupId} placeholder="line1_sensors_1000ms" />
    </label>
    <label>
      周期(ms)
      <input type="number" min="100" step="100" bind:value={scanRateMs} />
    </label>
  </div>

  <label>
    テーブル選択
    <select bind:value={selectedTableKey} onchange={onTableChanged}>
      <option value="">テーブルを選択してください</option>
      {#each tables as table (`${table.schema}.${table.name}`)}
        <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
      {/each}
    </select>
  </label>

  <label>
    時系列フィールド
    <select bind:value={timestampColumn} disabled={columns.length === 0}>
      <option value="">時系列フィールドを選択</option>
      {#each columns as column (column.name)}
        <option value={column.name}>{column.name} ({column.data_type})</option>
      {/each}
    </select>
  </label>

  <div class="field-box">
    <div class="field-header">
      <strong>フィールド選択（タグ化対象）</strong>
      {#if loadingColumns}<span>読込中...</span>{/if}
    </div>
    {#if columnError}
      <p class="error">{columnError}</p>
    {:else if columns.length === 0}
      <p class="muted">テーブル選択後にフィールドが表示されます。</p>
    {:else}
      <ul class="field-list">
        {#each columns as column (column.name)}
          <li>
            <label class="field-item">
              <input
                type="checkbox"
                checked={selectedFields.includes(column.name)}
                onchange={(event) => toggleField(column.name, (event.currentTarget as HTMLInputElement).checked)}
              />
              <span>{column.name}</span>
              <small>{column.data_type}</small>
            </label>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="summary">
    <p>接続先ID: <strong>{driverIdInput || '-'}</strong></p>
    <p>選択テーブル: <strong>{selectedTableKey || '-'}</strong></p>
    <p>時系列フィールド: <strong>{timestampColumn || '-'}</strong></p>
    <p>タグ化フィールド数: <strong>{selectedFields.length}</strong></p>
  </div>

  <div class="confirm-box">
    <button class="btn confirm" onclick={confirmAndClose} disabled={!canConfirm}>
      {exporting ? '確定中...' : '確定してウィンドウを閉じる'}
    </button>
    {#if exportMessage}<p class="ok">{exportMessage}</p>{/if}
    {#if exportError}<p class="error">{exportError}</p>{/if}
  </div>
</section>

<style>
  .panel {
    padding: 16px;
    overflow-y: auto;
    height: 100%;
    box-sizing: border-box;
    background: #f8fafc;
  }
  h4 { margin: 0 0 6px; color: #1e293b; }
  .desc { margin: 0 0 12px; font-size: 0.8rem; color: #475569; }
  .row { display: flex; gap: 8px; margin-bottom: 8px; }
  .btn {
    border: none;
    border-radius: 6px;
    background: #2563eb;
    color: #fff;
    padding: 6px 10px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .btn.secondary { background: #475569; }
  .btn.confirm { width: 100%; padding: 8px; font-size: 0.85rem; }
  .btn:disabled { opacity: 0.7; cursor: not-allowed; }
  .grid2 { display: grid; gap: 8px; grid-template-columns: 1fr 1fr; margin-bottom: 8px; }
  .grid3 { display: grid; gap: 8px; grid-template-columns: 1fr 1fr 1fr; margin-bottom: 8px; }
  label { display: grid; gap: 4px; font-size: 0.82rem; color: #334155; margin-bottom: 8px; }
  input, select {
    padding: 6px 8px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.82rem;
    background: #fff;
  }
  .field-box {
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #fff;
    padding: 8px;
    margin-bottom: 8px;
  }
  .field-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
    font-size: 0.8rem;
  }
  .field-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
    max-height: 180px;
    overflow-y: auto;
  }
  .field-item { display: flex; align-items: center; gap: 8px; margin: 0; }
  .field-item small { color: #64748b; }
  .summary {
    margin-bottom: 10px;
    background: #eef2ff;
    border: 1px solid #c7d2fe;
    border-radius: 6px;
    padding: 8px;
    font-size: 0.8rem;
    color: #334155;
  }
  .summary p { margin: 2px 0; }
  .confirm-box {
    border: 1px dashed #2563eb;
    border-radius: 6px;
    background: #fff;
    padding: 10px;
    display: grid;
    gap: 6px;
  }
  .ok { color: #166534; font-size: 0.8rem; margin: 4px 0; }
  .error { color: #b91c1c; font-size: 0.8rem; margin: 4px 0; }
  .muted { color: #64748b; font-size: 0.8rem; margin: 0; }
</style>
