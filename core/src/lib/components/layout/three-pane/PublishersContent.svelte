<script lang="ts">
  import { listPublishers, savePublisher, type PublisherDto, type SavePublisherRequest } from '$lib/ipc';
  import PublisherEditor from './publishers/PublisherEditor.svelte';
  import PublisherList from './publishers/PublisherList.svelte';
  import { createDefaultForm, toForm } from './publishers/form';

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

  function setFormField(key: keyof SavePublisherRequest, value: string | number | boolean) {
    form = {
      ...form,
      [key]: value,
    } as SavePublisherRequest;
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
    <PublisherList
      {loading}
      {publishers}
      {selectedPublisherId}
      onSelect={selectPublisher}
    />

    <PublisherEditor
      {form}
      {saving}
      {message}
      {errorMsg}
      onFieldChange={setFormField}
      onSave={save}
    />
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
</style>
