<script lang="ts">
  import type { PostgresColumnDto } from '$lib/ipc';

  let {
    columns,
    loadingColumns,
    columnError,
    timestampColumn,
    selectedFields,
    onTimestampColumnChange,
    onToggleField,
  }: {
    columns: PostgresColumnDto[];
    loadingColumns: boolean;
    columnError: string;
    timestampColumn: string;
    selectedFields: string[];
    onTimestampColumnChange: (value: string) => void;
    onToggleField: (fieldName: string, checked: boolean) => void;
  } = $props();
</script>

<label>
  時系列フィールド
  <select
    value={timestampColumn}
    disabled={columns.length === 0}
    onchange={(event) => onTimestampColumnChange((event.currentTarget as HTMLSelectElement).value)}
  >
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
              onchange={(event) => onToggleField(column.name, (event.currentTarget as HTMLInputElement).checked)}
            />
            <span>{column.name}</span>
            <small>{column.data_type}</small>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  label { display: grid; gap: 4px; font-size: 0.82rem; color: #334155; margin-bottom: 8px; }
  select {
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
  .error { color: #b91c1c; font-size: 0.8rem; margin: 4px 0; }
  .muted { color: #64748b; font-size: 0.8rem; margin: 0; }
</style>
