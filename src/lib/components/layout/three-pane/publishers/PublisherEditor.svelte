<script lang="ts">
  import type { SavePublisherRequest } from '$lib/ipc';

  let {
    form,
    saving,
    message,
    errorMsg,
    onFieldChange,
    onSave,
  }: {
    form: SavePublisherRequest;
    saving: boolean;
    message: string;
    errorMsg: string;
    onFieldChange: (key: keyof SavePublisherRequest, value: string | number | boolean) => void;
    onSave: () => void;
  } = $props();
</script>

<section class="editor-panel">
  <div class="panel-title">設定</div>
  <div class="form-grid">
    <label>
      ID
      <input
        value={form.id}
        placeholder="mqtt-main"
        oninput={(event) => onFieldChange('id', (event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <label>
      種別
      <select value={form.publisher_type} disabled>
        <option value="mqtt">mqtt</option>
      </select>
    </label>

    <label>
      Broker Host
      <input
        value={form.broker}
        placeholder="127.0.0.1"
        oninput={(event) => onFieldChange('broker', (event.currentTarget as HTMLInputElement).value)}
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
      Username
      <input
        value={form.username}
        placeholder="任意"
        oninput={(event) => onFieldChange('username', (event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <label>
      Password
      <input
        type="password"
        value={form.password}
        placeholder="変更時のみ入力"
        oninput={(event) => onFieldChange('password', (event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <label>
      Client ID
      <input
        value={form.client_id}
        placeholder="kt_iot_hub"
        oninput={(event) => onFieldChange('client_id', (event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <label>
      QoS
      <select
        value={form.qos}
        onchange={(event) => onFieldChange('qos', Number((event.currentTarget as HTMLSelectElement).value))}
      >
        <option value={0}>0</option>
        <option value={1}>1</option>
        <option value={2}>2</option>
      </select>
    </label>

    <label class="full-width">
      Topic
      <input
        value={form.topic}
        placeholder="plant"
        oninput={(event) => onFieldChange('topic', (event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <p class="hint full-width">
      配信先は <code>&lt;topic&gt;/&lt;接続先ID&gt;/tags/&lt;groupId&gt;/&lt;tagName&gt;</code> です。<br />
      例: <code>plant/postgresql/tags/bte1w/w0400</code><br />
      Topic が空欄なら <code>&lt;接続先ID&gt;/tags/&lt;groupId&gt;/&lt;tagName&gt;</code> を使い、payload はスカラー値を送ります。<br />
      下のチェックを入れると、アプリ起動時にドライバと MQTT 配信を自動開始します。未チェック時はダッシュボードの開始ボタンで起動します。
    </p>

    <label class="check-label">
      <input
        type="checkbox"
        checked={form.enabled}
        onchange={(event) => onFieldChange('enabled', (event.currentTarget as HTMLInputElement).checked)}
      />
      起動時に MQTT 配信を開始
    </label>

    <label class="check-label">
      <input
        type="checkbox"
        checked={form.retain}
        onchange={(event) => onFieldChange('retain', (event.currentTarget as HTMLInputElement).checked)}
      />
      Retain
    </label>

    <div class="form-actions full-width">
      <button class="btn-primary" onclick={onSave} disabled={saving || !form.id.trim() || !form.broker.trim() || !form.client_id.trim()}>
        {saving ? '保存中...' : '保存'}
      </button>
    </div>

    {#if message}<p class="ok full-width">{message}</p>{/if}
    {#if errorMsg}<p class="error full-width">{errorMsg}</p>{/if}
  </div>
</section>

<style>
  .editor-panel {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  .panel-title {
    margin-bottom: 12px;
    font-size: 0.85rem;
    font-weight: 700;
    color: #475569;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .full-width {
    grid-column: 1 / -1;
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

  input:disabled,
  select:disabled {
    background: #f6f8fb;
    color: #7f8c8d;
  }

  .check-label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .hint {
    margin: 0;
    color: #64748b;
    font-size: 0.8rem;
    line-height: 1.6;
  }

  code {
    font-family: Consolas, monospace;
    font-size: 0.78rem;
  }

  .form-actions {
    display: flex;
    gap: 8px;
  }

  .btn-primary {
    background-color: #2e86c1;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .btn-primary:hover {
    background-color: #2471a3;
  }

  .btn-primary:disabled {
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
