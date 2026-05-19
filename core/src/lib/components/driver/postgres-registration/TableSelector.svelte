<script lang="ts">
  import type { PostgresTableDto } from '$lib/ipc';

  let {
    scanGroupId,
    scanRateMs,
    selectedTableKey,
    tables,
    onScanGroupIdInput,
    onScanRateMsInput,
    onTableChange,
  }: {
    scanGroupId: string;
    scanRateMs: number;
    selectedTableKey: string;
    tables: PostgresTableDto[];
    onScanGroupIdInput: (value: string) => void;
    onScanRateMsInput: (value: number) => void;
    onTableChange: (value: string) => void;
  } = $props();
</script>

<div class="grid2">
  <label>
    周期グループID
    <input value={scanGroupId} placeholder="line1_sensors_1000ms" oninput={(event) => onScanGroupIdInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
  <label>
    周期(ms)
    <input type="number" min="100" step="100" value={scanRateMs} oninput={(event) => onScanRateMsInput(Number((event.currentTarget as HTMLInputElement).value))} />
  </label>
</div>

<label>
  テーブル選択
  <select value={selectedTableKey} onchange={(event) => onTableChange((event.currentTarget as HTMLSelectElement).value)}>
    <option value="">テーブルを選択してください</option>
    {#each tables as table (`${table.schema}.${table.name}`)}
      <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
    {/each}
  </select>
</label>

<style>
  .grid2 { display: grid; gap: 8px; grid-template-columns: 1fr 1fr; margin-bottom: 8px; }
  label { display: grid; gap: 4px; font-size: 0.82rem; color: #334155; margin-bottom: 8px; }
  input,
  select {
    padding: 6px 8px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.82rem;
    background: #fff;
  }
</style>
