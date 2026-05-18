import {
  clearMessageElements,
  formatError,
  invokeDirect,
  normalizeId
} from '../../../common/ui-assets/tauri.js';

const invoke = (cmd, args = {}) => invokeDirect(cmd, args);

let launchContext = null;
let tables = [];
let columns = [];
let scanGroups = [];
let existingDriverIds = [];
let activeTableKey = '';
let isEditMode = false;
let selectedTimestampField = '';
const selectedFields = new Set();

const el = (id) => document.getElementById(id);
const msgOk = el('msgOk');
const msgErr = el('msgErr');
const outOk = el('outOk');
const outErr = el('outErr');

function clearMessages() {
  clearMessageElements([msgOk, msgErr, outOk, outErr]);
}

function conn() {
  return {
    host: el('host').value.trim(),
    port: Number(el('port').value || 5432),
    database: el('database').value.trim(),
    username: el('username').value.trim(),
    password: el('password').value
  };
}

function contextData() {
  return launchContext?.context || {};
}

function tableKey(table) {
  return `${table.schema}.${table.name}`;
}

function selectedTable() {
  return tables.find((table) => tableKey(table) === activeTableKey) || null;
}

function findGroupByKey(key) {
  return scanGroups.find((group) => group.key === key) || null;
}

function mapPgTypeToTagType(dataType) {
  const t = String(dataType || '').toLowerCase();
  if (['bool', 'i32', 'i64', 'f32', 'f64', 'string'].includes(t)) return t;
  if (t.includes('bool')) return 'bool';
  if (t.includes('int2') || t.includes('smallint') || t.includes('int4') || t.includes('integer')) return 'i32';
  if (t.includes('int8') || t.includes('bigint')) return 'i64';
  if (t.includes('real') || t.includes('float4')) return 'f32';
  if (t.includes('double') || t.includes('float8') || t.includes('numeric') || t.includes('decimal')) return 'f64';
  return 'string';
}

function generateDefaultDriverId() {
  let num = 1;
  while (existingDriverIds.includes(`postgresql${num}`)) {
    num += 1;
  }
  return `postgresql${num}`;
}

function groupIdForTable(table) {
  return table ? table.name : '';
}

function isLikelyTimestampColumn(column) {
  const type = String(column?.dataType || '').toLowerCase();
  const name = String(column?.name || '').toLowerCase();
  return (
    type.includes('timestamp')
    || type === 'date'
    || type.includes('time')
    || /(timestamp|datetime|created_at|updated_at|recorded_at|measured_at|event_time|date|time|_ts|^ts$)/.test(name)
  );
}

function guessTimestampColumn(columnList) {
  const guessed = columnList.find((column) => isLikelyTimestampColumn(column));
  return guessed ? guessed.name : '';
}

function restoreScanGroups(rawScanGroups) {
  if (!Array.isArray(rawScanGroups)) {
    return [];
  }

  return rawScanGroups
    .filter((group) => group && group.schema && group.table)
    .map((group) => ({
      key: `${group.schema}.${group.table}`,
      id: group.table,
      schema: group.schema,
      table: group.table,
      scanRateMs: Number(group.scanRateMs || 1000),
      timestampColumn: group.timestampColumn || '',
      columnTypes: Array.isArray(group.tags)
        ? Object.fromEntries(
            group.tags
              .map((tag) => [tag?.driverSpec?.valueColumn || tag?.driverSpec?.value_column, tag?.dataType || 'string'])
              .filter(([fieldName]) => Boolean(fieldName))
          )
        : {},
      selectedFields: Array.isArray(group.tags)
        ? group.tags
            .map((tag) => tag?.driverSpec?.valueColumn || tag?.driverSpec?.value_column || null)
            .filter(Boolean)
        : []
    }));
}

function totalTagCount() {
  return scanGroups.reduce((sum, group) => sum + group.selectedFields.length, 0);
}

function setStep(step) {
  document.querySelectorAll('[data-step-panel]').forEach((panel) => {
    panel.classList.toggle('hidden', Number(panel.dataset.stepPanel) !== step);
  });

  document.querySelectorAll('.stepper .step').forEach((button) => {
    const buttonStep = Number(button.dataset.step);
    button.classList.toggle('active', buttonStep === step);
    button.classList.toggle('completed', buttonStep < step);
  });

  if (step === 3) {
    renderReview();
  }
}

function updateModeUi() {
  el('modeBadge').textContent = isEditMode ? '既存接続先の編集' : '新規登録';
  el('pageTitle').textContent = isEditMode ? 'PostgreSQL 接続先編集UI' : 'PostgreSQL 登録UI';
  el('pageDesc').textContent = isEditMode
    ? '既存接続先の情報を見直しながら、グループとタグ構成を更新します。'
    : '接続先設定 → グループ設定 → 確認して保存 の3ステップで登録します。';
}

function renderTables() {
  const tablesList = el('tablesList');
  const tablesHint = el('tablesHint');
  const filter = el('tableSearch').value.trim().toLowerCase();
  const filteredTables = tables.filter((table) => {
    const key = tableKey(table);
    const group = findGroupByKey(key);
    const scanRate = group ? `${group.scanRateMs} ms` : '-';
    const timestamp = group?.timestampColumn || '-';
    const tagCount = group ? String(group.selectedFields.length) : '0';
    const status = activeTableKey === key ? '選択' : group ? '済' : '未';
    const searchable = [
      table.name,
      table.schema,
      `${table.schema}.${table.name}`,
      scanRate,
      timestamp,
      tagCount,
      status
    ].join(' ').toLowerCase();
    return !filter || searchable.includes(filter);
  });

  tablesList.innerHTML = '';

  if (tables.length === 0) {
    tablesHint.textContent = 'まずは接続設定画面でテーブルを取得してください。';
    tablesHint.style.display = 'block';
    return;
  }

  if (filteredTables.length === 0) {
    tablesHint.textContent = '条件に一致するテーブルがありません。';
    tablesHint.style.display = 'block';
    return;
  }

  tablesHint.style.display = 'none';

  for (const table of filteredTables) {
    const key = tableKey(table);
    const group = findGroupByKey(key);
    const item = document.createElement('div');
    item.className = `table-item${activeTableKey === key ? ' active' : ''}`;
    const scanRate = group ? `${group.scanRateMs} ms` : '-';
    const timestamp = group?.timestampColumn || '-';
    const tagCount = group ? `${group.selectedFields.length}` : '0';
    item.innerHTML = `
      <button type="button" class="table-item-trigger table-cell table-name">${table.name}</button>
      <button type="button" class="table-item-trigger table-cell table-muted">${table.schema}</button>
      <button type="button" class="table-item-trigger table-cell table-period">${scanRate}</button>
      <button type="button" class="table-item-trigger table-cell table-muted">${timestamp}</button>
      <button type="button" class="table-item-trigger table-cell table-count">${tagCount}</button>
      <div class="table-item-actions">
        <span class="badge ${activeTableKey === key ? 'active' : group ? 'success' : 'neutral'}">${activeTableKey === key ? '選択' : group ? '済' : '未'}</span>
        ${group ? `
          <button type="button" class="btn ghost tiny icon-btn table-edit" title="編集" aria-label="編集">編</button>
          <button type="button" class="btn ghost tiny icon-btn table-remove" title="削除" aria-label="削除">削</button>
        ` : ''}
      </div>
    `;
    item.querySelectorAll('.table-item-trigger').forEach((trigger) => trigger.addEventListener('click', async () => {
      activeTableKey = key;
      const existingGroup = findGroupByKey(key);
      await loadColumnsForSelectedTable(table, existingGroup);
      renderTables();
    }));

    if (group) {
      item.querySelector('.table-edit').addEventListener('click', async (event) => {
        event.stopPropagation();
        setStep(2);
        await loadGroupIntoEditor(group.key);
      });
      item.querySelector('.table-remove').addEventListener('click', (event) => {
        event.stopPropagation();
        scanGroups = scanGroups.filter((entry) => entry.key !== group.key);
        if (activeTableKey === group.key) {
          selectedFields.clear();
          selectedTimestampField = '';
          columns = [];
          activeTableKey = key;
          el('groupName').value = groupIdForTable(table);
          el('groupRate').value = '1000';
          renderColumns();
        }
        renderTables();
        refreshSummary();
      });
    }

    tablesList.appendChild(item);
  }
}

function renderColumns() {
  const fieldList = el('fieldList');
  const fieldHint = el('fieldHint');
  const fieldActions = el('fieldActions');
  const fieldCountText = el('fieldCountText');
  const fieldSearch = el('fieldSearch');
  const fieldFilter = fieldSearch.value.trim().toLowerCase();
  const filteredColumns = columns.filter((column) => {
    const value = `${column.name} ${column.dataType || ''}`.toLowerCase();
    return !fieldFilter || value.includes(fieldFilter);
  });

  if (selectedTimestampField && selectedFields.has(selectedTimestampField)) {
    selectedFields.delete(selectedTimestampField);
  }

  fieldList.innerHTML = '';
  fieldActions.innerHTML = '';
  fieldCountText.textContent = `${columns.length}件 / タグ ${selectedFields.size}件`;

  if (columns.length === 0) {
    fieldHint.textContent = 'テーブル選択後に、フィールド一覧が表示されます。';
    fieldHint.style.display = 'block';
    fieldCountText.textContent = '0件 / タグ 0件';
    return;
  }

  const selectAllButton = document.createElement('button');
  selectAllButton.type = 'button';
  selectAllButton.className = 'btn secondary';
  selectAllButton.textContent = '全選択';
  selectAllButton.addEventListener('click', () => {
    for (const column of filteredColumns) {
      if (column.name !== selectedTimestampField) {
        selectedFields.add(column.name);
      }
    }
    renderColumns();
    refreshSummary();
  });

  const clearAllButton = document.createElement('button');
  clearAllButton.type = 'button';
  clearAllButton.className = 'btn ghost';
  clearAllButton.textContent = '全解除';
  clearAllButton.addEventListener('click', () => {
    for (const column of filteredColumns) {
      selectedFields.delete(column.name);
    }
    renderColumns();
    refreshSummary();
  });

  fieldActions.appendChild(selectAllButton);
  fieldActions.appendChild(clearAllButton);

  if (filteredColumns.length === 0) {
    fieldHint.textContent = '条件に一致するフィールドがありません。';
    fieldHint.style.display = 'block';
    fieldList.innerHTML = '<li class="empty-state muted">一致するフィールドがありません。</li>';
    return;
  }

  fieldHint.style.display = 'none';

  for (const column of filteredColumns) {
    const li = document.createElement('li');
    const isTimestamp = selectedTimestampField === column.name;
    li.className = 'field-row';
    li.innerHTML = `
      <span class="field-row-name" title="${column.name}">${column.name}</span>
      <span class="field-row-type" title="${column.dataType || ''}">${column.dataType || '-'}</span>
      <label class="field-row-check radio-check" aria-label="${column.name} を時系列列にする"><input type="radio" name="timestampField" class="timestamp-check" ${isTimestamp ? 'checked' : ''} /></label>
      ${isTimestamp
        ? '<span class="field-row-tag-disabled">除外</span>'
        : `<label class="field-row-check" aria-label="${column.name} をタグ化する"><input type="checkbox" class="tag-check" ${selectedFields.has(column.name) ? 'checked' : ''} /></label>`}
    `;

    const timestampCheck = li.querySelector('.timestamp-check');
    timestampCheck.addEventListener('change', () => {
      selectedTimestampField = column.name;
      selectedFields.delete(column.name);
      renderColumns();
      refreshSummary();
    });

    const tagCheck = li.querySelector('.tag-check');
    if (tagCheck) {
      tagCheck.addEventListener('change', () => {
        if (tagCheck.checked) {
          selectedFields.add(column.name);
        } else {
          selectedFields.delete(column.name);
        }
        renderColumns();
        refreshSummary();
      });
    }

    fieldList.appendChild(li);
  }
}

function renderGroups() {
  el('groupsCountText').textContent = `登録済み ${scanGroups.length}件`;
}

function refreshSummary() {
  const currentTable = selectedTable();
  const totalFields = totalTagCount();
  const currentConn = conn();
  el('activeTableBadge').textContent = currentTable ? `${currentTable.schema}.${currentTable.name}` : '未選択';
  el('connectionHint').textContent = tables.length > 0
    ? `${tables.length}件のテーブルを取得済みです。`
    : 'テーブル取得後にグループ設定へ進めます。';
  el('summary').innerHTML = `
    <div class="summary-card">
      <span class="summary-card-label">接続先ID</span>
      <span class="summary-card-value">${el('driverId').value || '-'}</span>
    </div>
    <div class="summary-card">
      <span class="summary-card-label">データベース</span>
      <span class="summary-card-value">${currentConn.database || '-'}</span>
    </div>
    <div class="summary-card">
      <span class="summary-card-label">グループ数</span>
      <span class="summary-card-value">${scanGroups.length}</span>
    </div>
    <div class="summary-card">
      <span class="summary-card-label">タグ数</span>
      <span class="summary-card-value">${totalFields}</span>
    </div>
    <div class="summary-card">
      <span class="summary-card-label">接続先</span>
      <span class="summary-card-value">${currentConn.host || '-'}:${currentConn.port || '-'}</span>
    </div>
    <div class="summary-card">
      <span class="summary-card-label">編集中</span>
      <span class="summary-card-value">${currentTable ? `${currentTable.schema}.${currentTable.name}` : (selectedTimestampField || '-')}</span>
    </div>
  `;
}

function renderReview() {
  const currentConn = conn();
  const totalFields = totalTagCount();

  el('reviewConnection').innerHTML = `
    <div class="review-card">
      <span class="review-card-label">接続先ID</span>
      <span class="review-card-value">${el('driverId').value || '-'}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">接続先</span>
      <span class="review-card-value">${currentConn.host || '-'}:${currentConn.port || '-'}</span>
      <div class="review-sublist">
        <div class="review-subitem"><span>DB</span><strong>${currentConn.database || '-'}</strong></div>
      </div>
    </div>
  `;

  el('reviewCounts').innerHTML = `
    <div class="review-card">
      <span class="review-card-label">登録グループ</span>
      <span class="review-card-value">${scanGroups.length}</span>
      <div class="review-sublist">
        <div class="review-subitem"><span>取得テーブル</span><strong>${tables.length}</strong></div>
      </div>
    </div>
    <div class="review-card">
      <span class="review-card-label">タグ総数</span>
      <span class="review-card-value">${totalFields}</span>
      <div class="review-sublist">
        <div class="review-subitem"><span>ユーザー名</span><strong>${currentConn.username || '-'}</strong></div>
      </div>
    </div>
  `;

  const reviewGrid = document.querySelector('.review-grid');
  reviewGrid?.classList.add('compact-review');

  el('reviewGroupsCount').textContent = `${scanGroups.length}件`;

  const reviewGroups = el('reviewGroups');
  reviewGroups.innerHTML = '';

  if (scanGroups.length === 0) {
    reviewGroups.innerHTML = '<div class="empty-state muted">登録対象グループはまだありません。接続先設定のみ保存することもできます。</div>';
    return;
  }

  for (const group of scanGroups) {
    const card = document.createElement('div');
    card.className = 'group-card';
    card.innerHTML = `
      <div class="group-card-head">
        <div>
          <div class="group-card-title">${group.id}</div>
          <div class="muted">${group.schema}.${group.table}</div>
        </div>
        <span class="badge success">タグ ${group.selectedFields.length}件</span>
      </div>
      <ul class="group-meta">
        <li>周期: ${group.scanRateMs} ms</li>
        <li>時系列フィールド: ${group.timestampColumn || '-'}</li>
        <li>タグ対象: ${group.selectedFields.join(', ') || '-'}</li>
      </ul>
    `;
    reviewGroups.appendChild(card);
  }
}

function validateConnectionInputs() {
  if (!el('driverId').value.trim()) throw new Error('接続先IDを入力してください');
}

async function goToStep2() {
  validateConnectionInputs();

  const canLoadTables = Boolean(el('host').value.trim() && el('database').value.trim() && el('username').value.trim());

  if (canLoadTables && tables.length === 0) {
    await loadTables();
  }

  setStep(2);
}

function goToStep3() {
  setStep(3);
}

async function closeWindowSafely() {
  try {
    await invoke('close_driver_ui_window');
    return;
  } catch {
    // no-op
  }
  try {
    window.close();
  } catch {
    // no-op
  }
}

async function loadColumnsForSelectedTable(table, existingGroup = null) {
  columns = [];
  selectedTimestampField = '';
  selectedFields.clear();
  renderColumns();

  const groupName = el('groupName');
  const groupRate = el('groupRate');
  groupName.value = table ? groupIdForTable(table) : '';
  groupRate.value = existingGroup ? String(existingGroup.scanRateMs) : '1000';
  activeTableKey = table ? tableKey(table) : '';

  if (!table) {
    refreshSummary();
    return;
  }

  try {
    columns = await invoke('postgres_list_columns', {
      req: { conn: conn(), schema: table.schema, table: table.name }
    });

    selectedTimestampField = existingGroup?.timestampColumn || guessTimestampColumn(columns);

    for (const fieldName of existingGroup?.selectedFields || []) {
      if (fieldName !== selectedTimestampField) {
        selectedFields.add(fieldName);
      }
    }

    renderColumns();
  } catch (error) {
    msgErr.textContent = formatError(error);
  }

  renderTables();
  refreshSummary();
}

async function loadGroupIntoEditor(groupKey) {
  const group = findGroupByKey(groupKey);
  if (!group) return;

  activeTableKey = group.key;
  const table = tables.find((item) => tableKey(item) === group.key) || null;
  await loadColumnsForSelectedTable(table, group);
}

async function loadTables() {
  clearMessages();
  tables = [];
  columns = [];
  selectedTimestampField = '';
  selectedFields.clear();
  renderColumns();

  try {
    tables = await invoke('postgres_list_tables', { conn: conn() });
    renderTables();
    msgOk.textContent = `${tables.length} 件のテーブルを取得しました`;

    const preferredKey = activeTableKey || scanGroups[0]?.key || (tables[0] ? tableKey(tables[0]) : '');
    if (preferredKey && tables.some((table) => tableKey(table) === preferredKey)) {
      activeTableKey = preferredKey;
      await loadGroupIntoEditor(preferredKey);
    }
  } catch (error) {
    msgErr.textContent = formatError(error);
  }

  refreshSummary();
}

function saveCurrentGroup() {
  clearMessages();
  const table = selectedTable();
  const timestampColumn = selectedTimestampField.trim();
  const scanRateMs = Number(el('groupRate').value || 1000);

  if (!table) {
    throw new Error('テーブルを選択してください');
  }
  if (!timestampColumn) {
    throw new Error('時系列フィールドを選択してください');
  }
  if (scanRateMs < 100) {
    throw new Error('周期は100ms以上である必要があります');
  }
  if (selectedFields.size === 0) {
    throw new Error('タグ化するフィールドを1つ以上選択してください');
  }

  const key = tableKey(table);
  const id = groupIdForTable(table);
  const duplicateId = scanGroups.find((group) => group.id === id && group.key !== key);
  if (duplicateId) {
    throw new Error(`同名テーブルのグループが既に存在します: ${id}`);
  }

  const group = {
    key,
    id,
    schema: table.schema,
    table: table.name,
    scanRateMs,
    timestampColumn,
    columnTypes: Object.fromEntries(columns.map((column) => [column.name, column.dataType || 'text'])),
    selectedFields: Array.from(selectedFields)
  };

  const existingIndex = scanGroups.findIndex((item) => item.key === key);
  if (existingIndex >= 0) {
    scanGroups.splice(existingIndex, 1, group);
    msgOk.textContent = `グループを更新しました: ${id}`;
  } else {
    scanGroups.push(group);
    msgOk.textContent = `グループを追加しました: ${id}`;
  }

  renderGroups();
  renderTables();
  refreshSummary();
}

function buildPayload() {
  const driverId = el('driverId').value.trim();
  if (!driverId) {
    throw new Error('接続先IDを入力してください');
  }

  const groups = scanGroups.map((group) => {
    if (!group.timestampColumn) {
      throw new Error(`時系列フィールドが未設定のグループがあります: ${group.id}`);
    }
    if (!group.selectedFields.length) {
      throw new Error(`タグ化対象が未選択のグループがあります: ${group.id}`);
    }

    const groupTags = group.selectedFields.map((fieldName) => {
      const dataType = group.columnTypes?.[fieldName] || 'text';
      return {
        id: normalizeId(`tag-${driverId}-${group.id}-${fieldName}`),
        name: fieldName,
        dataType: mapPgTypeToTagType(dataType),
        enabled: true,
        driverSpec: {
          kind: 'postgres',
          scanGroup: group.id,
          schema: group.schema,
          table: group.table,
          timestampColumn: group.timestampColumn,
          valueColumn: fieldName
        }
      };
    });

    return {
      id: group.id,
      scanRateMs: group.scanRateMs,
      schema: group.schema,
      table: group.table,
      timestampColumn: group.timestampColumn,
      tags: groupTags
    };
  });

  return {
    schemaVersion: 1,
    requestId: launchContext?.requestId || `req-${Date.now()}`,
    generatedAt: new Date().toISOString(),
    direction: 'driver-to-host',
    driver: {
      id: driverId,
      driverType: 'postgres',
      enabled: true,
      settings: {
        host: conn().host,
        port: conn().port,
        database: conn().database,
        username: conn().username,
        password: conn().password
      },
      scanGroups: groups
    }
  };
}

async function init() {
  clearMessages();

  try {
    launchContext = await invoke('get_driver_ui_launch_context');
    const ctx = contextData();
    isEditMode = Boolean(launchContext?.driverId);
    existingDriverIds = Array.isArray(ctx.existingDriverIds) ? ctx.existingDriverIds : [];

    const settings = ctx.driverSettings && typeof ctx.driverSettings === 'object'
      ? ctx.driverSettings
      : {};

    el('driverId').value = launchContext?.driverId || generateDefaultDriverId();
    el('host').value = settings.host || '127.0.0.1';
    el('port').value = String(settings.port || 5432);
    el('database').value = settings.database || '';
    el('username').value = settings.username || '';
    el('password').value = settings.password || '';

    scanGroups = restoreScanGroups(ctx.scanGroups);
    updateModeUi();
    renderGroups();

    if (scanGroups.length > 0 && conn().database && conn().username) {
      await loadTables();
    }
  } catch (error) {
    msgErr.textContent = formatError(error);
  }

  refreshSummary();
  setStep(1);
}

el('btnTest').addEventListener('click', async () => {
  clearMessages();
  try {
    const result = await invoke('postgres_test_connection', { conn: conn() });
    msgOk.textContent = result.message;
  } catch (error) {
    msgErr.textContent = formatError(error);
  }
});

el('btnTables').addEventListener('click', async () => {
  await loadTables();
});

el('btnSaveGroup').addEventListener('click', () => {
  try {
    saveCurrentGroup();
  } catch (error) {
    msgErr.textContent = formatError(error);
  }
});

el('btnConfirm').addEventListener('click', async () => {
  clearMessages();
  try {
    const payload = buildPayload();
    await invoke('save_driver_ui_output', {
      req: { outputJsonPath: launchContext?.outputJsonPath || null, payload }
    });

    outOk.textContent = '確定しました。ウィンドウを閉じます...';
    setTimeout(() => {
      void closeWindowSafely();
    }, 200);
  } catch (error) {
    outErr.textContent = formatError(error);
  }
});

el('btnStep1Next').addEventListener('click', async () => {
  clearMessages();
  try {
    await goToStep2();
  } catch (error) {
    msgErr.textContent = formatError(error);
  }
});

el('btnStep2Prev').addEventListener('click', () => {
  clearMessages();
  setStep(1);
});

el('btnStep2Next').addEventListener('click', () => {
  clearMessages();
  try {
    goToStep3();
  } catch (error) {
    msgErr.textContent = formatError(error);
  }
});

el('btnStep3Prev').addEventListener('click', () => {
  clearMessages();
  setStep(2);
});

el('tableSearch').addEventListener('input', () => {
  renderTables();
});

el('fieldSearch').addEventListener('input', () => {
  renderColumns();
});

document.querySelectorAll('.stepper .step').forEach((button) => {
  button.addEventListener('click', async () => {
    clearMessages();
    const targetStep = Number(button.dataset.step);
    try {
      if (targetStep === 1) {
        setStep(1);
        return;
      }
      if (targetStep === 2) {
        await goToStep2();
        return;
      }
      goToStep3();
    } catch (error) {
      msgErr.textContent = formatError(error);
    }
  });
});

['driverId', 'groupRate', 'host', 'port', 'database', 'username'].forEach((id) => {
  el(id).addEventListener('input', refreshSummary);
});

void init();
