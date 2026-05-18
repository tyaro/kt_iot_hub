<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import ColumnMapping from './postgres-registration/ColumnMapping.svelte';
  import ConnectionFields from './postgres-registration/ConnectionFields.svelte';
  import TableSelector from './postgres-registration/TableSelector.svelte';
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

  function setDriverIdInput(value: string) {
    driverIdInput = value;
  }

  function setHost(value: string) {
    host = value;
  }

  function setPortInput(value: string) {
    portInput = value;
  }

  function setDatabase(value: string) {
    database = value;
  }

  function setUsername(value: string) {
    username = value;
  }

  function setPassword(value: string) {
    password = value;
  }

  function setScanGroupId(value: string) {
    scanGroupId = value;
  }

  function setScanRateMs(value: number) {
    scanRateMs = value;
  }

  async function handleTableChange(value: string) {
    selectedTableKey = value;
    await onTableChanged();
  }

  function setTimestampColumn(value: string) {
    timestampColumn = value;
  }
</script>

<section class="panel">
  <h4>PostgreSQL 登録UI</h4>
  <p class="desc">接続情報 → テスト接続 → テーブル選択 → フィールド選択 → 確定 の順で設定します。</p>

  <ConnectionFields
    {driverIdInput}
    {host}
    {portInput}
    {database}
    {username}
    {password}
    {testing}
    {loadingTables}
    {testMessage}
    {testError}
    {tableError}
    onDriverIdInput={setDriverIdInput}
    onHostInput={setHost}
    onPortInput={setPortInput}
    onDatabaseInput={setDatabase}
    onUsernameInput={setUsername}
    onPasswordInput={setPassword}
    onTestConnection={testConnection}
    onLoadTables={loadTables}
  />

  <TableSelector
    {scanGroupId}
    {scanRateMs}
    {selectedTableKey}
    {tables}
    onScanGroupIdInput={setScanGroupId}
    onScanRateMsInput={setScanRateMs}
    onTableChange={handleTableChange}
  />

  <ColumnMapping
    {columns}
    {loadingColumns}
    {columnError}
    {timestampColumn}
    {selectedFields}
    onTimestampColumnChange={setTimestampColumn}
    onToggleField={toggleField}
  />

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
  .btn {
    border: none;
    border-radius: 6px;
    background: #2563eb;
    color: #fff;
    padding: 6px 10px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .btn.confirm { width: 100%; padding: 8px; font-size: 0.85rem; }
  .btn:disabled { opacity: 0.7; cursor: not-allowed; }
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
  .error { color: #b91c1c; font-size: 0.8rem; margin: 4px 0; }
  .ok { color: #166534; font-size: 0.8rem; margin: 4px 0; }
</style>
