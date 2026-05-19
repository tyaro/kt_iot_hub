<script lang="ts">
  import type { SaveDriverRequest } from '$lib/ipc';

  let {
    mode,
    form,
    saving,
    message,
    errorMsg,
    onFieldChange,
    onSave,
    onCancel,
  }: {
    mode: 'detail' | 'new';
    form: SaveDriverRequest;
    saving: boolean;
    message: string;
    errorMsg: string;
    onFieldChange: (key: keyof SaveDriverRequest, value: string | number | boolean | null) => void;
    onSave: () => void;
    onCancel: () => void;
  } = $props();
</script>

<div class="form">
  <label>
    ID
    <input
      value={form.id}
      placeholder="postgres-main"
      oninput={(event) => onFieldChange('id', (event.currentTarget as HTMLInputElement).value)}
    />
  </label>
  {#if mode !== 'new'}
    <p class="hint">接続先IDを変更すると、配下の Scan グループとタグ参照もまとめて更新します。</p>
  {/if}
  <label>
    種別
    <select
      value={form.driver_type}
      disabled={mode !== 'new'}
      onchange={(event) => onFieldChange('driver_type', (event.currentTarget as HTMLSelectElement).value)}
    >
      <option value="postgres">postgres</option>
    </select>
  </label>
  <label>
    Host
    <input
      value={form.host}
      placeholder="127.0.0.1"
      oninput={(event) => onFieldChange('host', (event.currentTarget as HTMLInputElement).value)}
    />
  </label>
  <label>
    Port
    <input
      type="number"
      value={form.port}
      min="1"
      max="65535"
      oninput={(event) => onFieldChange('port', Number((event.currentTarget as HTMLInputElement).value))}
    />
  </label>
  <label>
    Database
    <input
      value={form.database}
      placeholder="mydb"
      oninput={(event) => onFieldChange('database', (event.currentTarget as HTMLInputElement).value)}
    />
  </label>
  <label>
    Username
    <input
      value={form.username}
      placeholder="postgres"
      oninput={(event) => onFieldChange('username', (event.currentTarget as HTMLInputElement).value)}
    />
  </label>
  <label>
    Password
    <input
      type="password"
      value={form.password}
      placeholder="（変更する場合のみ入力）"
      oninput={(event) => onFieldChange('password', (event.currentTarget as HTMLInputElement).value)}
    />
  </label>
  <label class="check-label">
    <input
      type="checkbox"
      checked={form.enabled}
      onchange={(event) => onFieldChange('enabled', (event.currentTarget as HTMLInputElement).checked)}
    />
    有効
  </label>

  <div class="form-actions">
    <button class="btn-primary" onclick={onSave} disabled={saving || !form.id.trim()}>
      {saving ? '保存中...' : mode === 'new' ? '作成' : '更新'}
    </button>
    {#if mode !== 'new'}
      <button class="btn-secondary" onclick={onCancel} disabled={saving}>キャンセル</button>
    {/if}
  </div>

  {#if message}<p class="ok">{message}</p>{/if}
  {#if errorMsg}<p class="error">{errorMsg}</p>{/if}
</div>

<style>
  .form {
    display: grid;
    gap: 10px;
  }

  .hint {
    margin: -2px 0 0;
    font-size: 0.78rem;
    color: #64748b;
    line-height: 1.45;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.85rem;
    color: #5a6776;
    font-weight: 600;
  }

  input,
  select {
    padding: 7px 8px;
    border: 1px solid #cfd8e3;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
  }

  input:disabled {
    background: #f6f8fb;
    color: #7f8c8d;
  }

  .check-label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .form-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .btn-primary {
    background: #3498db;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-secondary {
    background: #fff;
    color: #475569;
    border: 1px solid #cbd5e1;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .ok {
    color: #2e7d32;
    font-size: 0.82rem;
    margin: 0;
  }

  .error {
    color: #c0392b;
    font-size: 0.82rem;
    margin: 0;
  }
</style>
