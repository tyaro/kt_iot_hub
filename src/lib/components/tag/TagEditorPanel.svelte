<script lang="ts">
  import {
    createTag,
    listScanGroups,
    type CreateTagRequest,
    type ScanGroupDto,
    type TagDto,
  } from '$lib/ipc/index';

  interface Props {
    mode?: 'new' | 'edit';
    tag?: TagDto | null;
    driverId?: string | null;
    onDone?: () => void;
    onCancel?: () => void;
  }

  let {
    mode = 'new',
    tag = null,
    driverId = null,
    onDone = () => {},
    onCancel = () => {},
  }: Props = $props();

  let form = $state<CreateTagRequest>({
    id: '',
    name: '',
    data_type: 'f32',
    driver_id: '',
    scan_group_id: '',
    driver_spec: {},
  });
  let selectedScanGroupId = $state('');
  let scanGroups = $state<ScanGroupDto[]>([]);
  let driverSpecText = $state('{}');
  let loading = $state(false);
  let saving = $state(false);
  let errorMessage = $state('');
  let lastInitKey = '';

  $effect(() => {
    const effectiveDriverId = mode === 'edit' ? tag?.driver_id ?? '' : driverId ?? '';
    const initialScanGroupId = tag?.scan_group_id ?? '';
    const initKey = `${mode}:${tag?.id ?? ''}:${effectiveDriverId}:${initialScanGroupId}`;

    if (initKey === lastInitKey) {
      return;
    }

    lastInitKey = initKey;
    const driverSpecValue = tag?.driver_spec ?? {};
    form = {
      id: tag?.id ?? '',
      name: tag?.name ?? '',
      data_type: tag?.data_type ?? 'f32',
      driver_id: effectiveDriverId,
      scan_group_id: initialScanGroupId,
      driver_spec: driverSpecValue,
    };
    driverSpecText = JSON.stringify(driverSpecValue, null, 2);
    selectedScanGroupId = initialScanGroupId;
    errorMessage = '';
    void loadScanGroups(effectiveDriverId, initialScanGroupId);
  });

  async function loadScanGroups(targetDriverId: string, initialScanGroupId = '') {
    scanGroups = [];
    selectedScanGroupId = initialScanGroupId;
    if (!targetDriverId) return;
    loading = true;
    try {
      const groups = await listScanGroups(targetDriverId);
      scanGroups = groups;
      if (!selectedScanGroupId && groups.length > 0) {
        selectedScanGroupId = groups[0].id;
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'スキャングループ取得に失敗しました';
    } finally {
      loading = false;
    }
  }

  async function save() {
    errorMessage = '';
    try {
      form.driver_spec = JSON.parse(driverSpecText) as Record<string, unknown>;
    } catch {
      errorMessage = 'ドライバ仕様(JSON)の形式が不正です';
      return;
    }

    saving = true;
    try {
      await createTag({
        ...form,
        scan_group_id: selectedScanGroupId,
      });
      onDone();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'タグ保存に失敗しました';
    } finally {
      saving = false;
    }
  }
</script>

<div class="editor-panel">
  <div class="panel-header">
    <h3>{mode === 'new' ? '手動でタグを追加' : '手動でタグを編集'}</h3>
    <span class="mode-label">手動編集</span>
  </div>

  <div class="form-grid">
    <label>
      ID
      <input bind:value={form.id} disabled={mode === 'edit'} placeholder="tag-0004" />
    </label>
    <label>
      名前
      <input bind:value={form.name} placeholder="temperature" />
    </label>
    <label>
      データ型
      <select bind:value={form.data_type}>
        <option value="bool">bool</option>
        <option value="i32">i32</option>
        <option value="i64">i64</option>
        <option value="f32">f32</option>
        <option value="f64">f64</option>
        <option value="string">string</option>
      </select>
    </label>
    <label>
      ドライバID
      <input bind:value={form.driver_id} disabled />
    </label>
    <label>
      スキャングループ
      <select bind:value={selectedScanGroupId} disabled={loading || scanGroups.length === 0}>
        {#if scanGroups.length === 0}
          <option value="">スキャングループなし</option>
        {:else}
          {#each scanGroups as group (group.id)}
            <option value={group.id}>{group.id}</option>
          {/each}
        {/if}
      </select>
    </label>
    <label class="full-width">
      ドライバ仕様(JSON)
      <textarea bind:value={driverSpecText} rows="8" spellcheck="false"></textarea>
    </label>
  </div>

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}

  <div class="actions">
    <button class="btn-primary" onclick={save} disabled={saving || !form.id || !form.name || !selectedScanGroupId}>
      {saving ? '保存中...' : '保存'}
    </button>
    <button class="btn-outline" onclick={onCancel}>キャンセル</button>
  </div>
</div>

<style>
  .editor-panel {
    padding: 16px;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    border-bottom: 1px solid #e3e8ef;
    padding-bottom: 10px;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #2c3e50;
  }

  .mode-label {
    color: #64748b;
    font-size: 0.78rem;
    background: #f1f5f9;
    border-radius: 999px;
    padding: 0.2rem 0.6rem;
  }

  .form-grid {
    display: grid;
    gap: 10px;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.85rem;
    color: #5a6776;
    font-weight: 600;
  }

  input,
  select,
  textarea {
    padding: 7px 8px;
    border: 1px solid #cfd8e3;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
    font-family: inherit;
  }

  textarea {
    font-family: Consolas, 'Courier New', monospace;
    resize: vertical;
  }

  input:disabled,
  select:disabled {
    background: #f6f8fb;
    color: #7f8c8d;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .error {
    margin: 12px 0 0;
    color: #b91c1c;
    font-size: 0.82rem;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 14px;
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

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-outline {
    background: #fff;
    color: #1e40af;
    border: 1px solid #bfdbfe;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }
</style>
