<script lang="ts">
  import { listPublishers, savePublisher, type PublisherDto, type SavePublisherRequest } from '$lib/ipc';

  function createDefaultForm(): SavePublisherRequest {
    return {
      id: 'mqtt-main',
      publisher_type: 'mqtt',
      enabled: false,
      broker: '127.0.0.1',
      port: 1883,
      username: '',
      password: '',
      client_id: 'kt_iot_hub',
      qos: 1,
      retain: false,
      topic: '',
    };
  }

  function toForm(publisher: PublisherDto): SavePublisherRequest {
    return {
      ...publisher,
      password: '',
      topic: publisher.topic ?? '',
      qos: (publisher.qos as 0 | 1 | 2) ?? 1,
    };
  }

  let publishers = $state<PublisherDto[]>([]);
  let selectedPublisherId = $state<string | null>(null);
  let form = $state<SavePublisherRequest>(createDefaultForm());
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let errorMsg = $state('');
  let initialized = $state(false);

  async function loadPublishers(preferredId?: string | null) {
    loading = true;
    errorMsg = '';

    try {
      const items = await listPublishers();
      publishers = items;

      const targetId = preferredId ?? selectedPublisherId ?? items[0]?.id ?? null;
      const selected = items.find((item) => item.id === targetId) ?? items[0] ?? null;

      selectedPublisherId = selected?.id ?? null;
      form = selected ? toForm(selected) : createDefaultForm();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'パブリッシャ設定の読込に失敗しました';
      publishers = [];
      selectedPublisherId = null;
      form = createDefaultForm();
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (initialized) {
      return;
    }
    initialized = true;
    void loadPublishers();
  });

  function selectPublisher(publisher: PublisherDto) {
    selectedPublisherId = publisher.id;
    form = toForm(publisher);
    message = '';
    errorMsg = '';
  }

  function createNewPublisher() {
    selectedPublisherId = null;
    form = createDefaultForm();
    message = '';
    errorMsg = '';
  }

  async function save() {
    saving = true;
    message = '';
    errorMsg = '';

    try {
      await savePublisher({
        ...form,
        id: form.id.trim(),
        broker: form.broker.trim(),
        username: form.username.trim(),
        client_id: form.client_id.trim(),
        topic: form.topic.trim(),
      });
      await loadPublishers(form.id.trim());
      message = selectedPublisherId ? 'パブリッシャ設定を保存しました' : 'パブリッシャを作成しました';
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : '保存に失敗しました';
    } finally {
      saving = false;
    }
  }
</script>

<div class="content">
  <div class="content-header">
    <h2>パブリッシャ管理</h2>
    <button class="btn-primary" onclick={createNewPublisher}>＋ MQTT パブリッシャ</button>
  </div>

  <div class="layout-grid">
    <aside class="publisher-list-panel">
      <div class="panel-title">登録済み</div>
      {#if loading}
        <p class="muted">読込中...</p>
      {:else if publishers.length === 0}
        <p class="muted">まだパブリッシャはありません</p>
      {:else}
        <div class="publisher-list">
          {#each publishers as publisher (publisher.id)}
            <button
              class="publisher-item"
              class:selected={selectedPublisherId === publisher.id}
              onclick={() => selectPublisher(publisher)}
            >
              <span class="item-main">{publisher.id}</span>
              <span class="item-sub">{publisher.broker}:{publisher.port}</span>
            </button>
          {/each}
        </div>
      {/if}
    </aside>

    <section class="editor-panel">
      <div class="panel-title">設定</div>
      <div class="form-grid">
        <label>
          ID
          <input bind:value={form.id} placeholder="mqtt-main" />
        </label>

        <label>
          種別
          <select bind:value={form.publisher_type} disabled>
            <option value="mqtt">mqtt</option>
          </select>
        </label>

        <label>
          Broker Host
          <input bind:value={form.broker} placeholder="127.0.0.1" />
        </label>

        <label>
          Port
          <input type="number" bind:value={form.port} min="1" max="65535" />
        </label>

        <label>
          Username
          <input bind:value={form.username} placeholder="任意" />
        </label>

        <label>
          Password
          <input type="password" bind:value={form.password} placeholder="変更時のみ入力" />
        </label>

        <label>
          Client ID
          <input bind:value={form.client_id} placeholder="kt_iot_hub" />
        </label>

        <label>
          QoS
          <select bind:value={form.qos}>
            <option value={0}>0</option>
            <option value={1}>1</option>
            <option value={2}>2</option>
          </select>
        </label>

        <label class="full-width">
          Topic
          <input bind:value={form.topic} placeholder="plant" />
        </label>

        <p class="hint full-width">
          配信先は <code>&lt;topic&gt;/&lt;接続先ID&gt;/tags/&lt;groupId&gt;/&lt;tagName&gt;</code> です。<br />
          例: <code>plant/postgresql/tags/bte1w/w0400</code><br />
          Topic が空欄なら <code>&lt;接続先ID&gt;/tags/&lt;groupId&gt;/&lt;tagName&gt;</code> を使い、payload はスカラー値を送ります。<br />
          下のチェックを入れると、アプリ起動時にドライバと MQTT 配信を自動開始します。未チェック時はダッシュボードの開始ボタンで起動します。
        </p>

        <label class="check-label">
          <input type="checkbox" bind:checked={form.enabled} />
          起動時に MQTT 配信を開始
        </label>

        <label class="check-label">
          <input type="checkbox" bind:checked={form.retain} />
          Retain
        </label>

        <div class="form-actions full-width">
          <button class="btn-primary" onclick={save} disabled={saving || !form.id.trim() || !form.broker.trim() || !form.client_id.trim()}>
            {saving ? '保存中...' : '保存'}
          </button>
        </div>

        {#if message}<p class="ok full-width">{message}</p>{/if}
        {#if errorMsg}<p class="error full-width">{errorMsg}</p>{/if}
      </div>
    </section>
  </div>
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .content-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }

  .content h2 {
    margin: 0;
    font-size: 1.1rem;
    color: #2c3e50;
  }

  .layout-grid {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: 16px;
    min-height: 420px;
  }

  .publisher-list-panel,
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

  .publisher-list {
    display: grid;
    gap: 8px;
  }

  .publisher-item {
    display: grid;
    gap: 4px;
    text-align: left;
    border: 1px solid #d5deea;
    background: #fff;
    border-radius: 6px;
    padding: 10px 12px;
    cursor: pointer;
  }

  .publisher-item.selected {
    border-color: #2e86c1;
    background: #eff6ff;
  }

  .item-main {
    font-size: 0.88rem;
    font-weight: 700;
    color: #1f2937;
  }

  .item-sub {
    font-size: 0.78rem;
    color: #64748b;
    word-break: break-all;
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

  .muted {
    margin: 0;
    color: #94a3b8;
    font-size: 0.82rem;
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
