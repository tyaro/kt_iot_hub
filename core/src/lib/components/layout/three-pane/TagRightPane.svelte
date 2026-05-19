<script lang="ts">
  import TagDetailPanel from '../../tag/TagDetailPanel.svelte';
  import TagEditorPanel from '../../tag/TagEditorPanel.svelte';
  import DriverDetailPanel from '../../driver/DriverDetailPanel.svelte';
  import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';

  let {
    currentPage,
    tagMode,
    selectedTag,
    selectedDriver,
    selectedScanGroup,
    editorDriverId,
    onUpdateScanGroupRate,
    onBulkUpdateDriverScanGroupRate,
    onTagEditorDone,
    onTagEditorCancel,
    onTagDetailEdit,
    onTagDetailDelete,
    onTagDetailClose,
    onDriverDelete,
    onDriverDone,
  }: {
    currentPage: string;
    tagMode: 'detail' | 'new' | 'edit';
    selectedTag: TagDto | null;
    selectedDriver: DriverDto | null;
    selectedScanGroup: ScanGroupDto | null;
    editorDriverId: string | null;
    onUpdateScanGroupRate: (scanGroup: ScanGroupDto, scanRateMs: number) => Promise<void>;
    onBulkUpdateDriverScanGroupRate: (driver: DriverDto, scanRateMs: number) => Promise<void>;
    onTagEditorDone: () => void | Promise<void>;
    onTagEditorCancel: () => void;
    onTagDetailEdit: (tag: TagDto) => void;
    onTagDetailDelete: (tag: TagDto) => void | Promise<void>;
    onTagDetailClose: () => void;
    onDriverDelete: (driverId: string) => void | Promise<void>;
    onDriverDone: (driverId?: string) => void | Promise<DriverDto | null>;
  } = $props();

  const MIN_SCAN_RATE_MS = 100;

  let driverBulkScanRateMs = $state(1000);
  let scanGroupRateMs = $state(1000);
  let updatingDriverBulkRate = $state(false);
  let updatingScanGroupRate = $state(false);
  let localErrorMessage = $state('');

  $effect(() => {
    if (selectedDriver) {
      const relatedGroups = selectedScanGroup?.driver_id === selectedDriver.id
        ? [selectedScanGroup]
        : [];
      driverBulkScanRateMs = relatedGroups[0]?.scan_rate_ms ?? driverBulkScanRateMs;
    }
  });

  $effect(() => {
    if (selectedScanGroup?.scan_rate_ms != null) {
      scanGroupRateMs = selectedScanGroup.scan_rate_ms;
    }
  });

  function validateScanRateMs(value: number): string | null {
    if (!Number.isFinite(value) || Number.isNaN(value)) {
      return '周期は数値で入力してください。';
    }
    if (!Number.isInteger(value)) {
      return '周期は整数で入力してください。';
    }
    if (value < MIN_SCAN_RATE_MS) {
      return `周期は ${MIN_SCAN_RATE_MS} ms 以上で入力してください。`;
    }
    return null;
  }

  async function submitDriverBulkRate() {
    if (!selectedDriver) {
      return;
    }

    const validationError = validateScanRateMs(driverBulkScanRateMs);
    if (validationError) {
      localErrorMessage = validationError;
      return;
    }

    localErrorMessage = '';
    updatingDriverBulkRate = true;
    try {
      await onBulkUpdateDriverScanGroupRate(selectedDriver, driverBulkScanRateMs);
    } finally {
      updatingDriverBulkRate = false;
    }
  }

  async function submitScanGroupRate() {
    if (!selectedScanGroup) {
      return;
    }

    const validationError = validateScanRateMs(scanGroupRateMs);
    if (validationError) {
      localErrorMessage = validationError;
      return;
    }

    localErrorMessage = '';
    updatingScanGroupRate = true;
    try {
      await onUpdateScanGroupRate(selectedScanGroup, scanGroupRateMs);
    } finally {
      updatingScanGroupRate = false;
    }
  }
</script>

<div class="right-pane">
  <div class="right-content">
    {#if currentPage === 'tags'}
      {#if tagMode === 'new' || tagMode === 'edit'}
        <TagEditorPanel
          mode={tagMode}
          tag={selectedTag}
          driverId={editorDriverId}
          onDone={onTagEditorDone}
          onCancel={onTagEditorCancel}
        />
      {:else if selectedTag}
        <TagDetailPanel
          tag={selectedTag}
          onRequestEdit={onTagDetailEdit}
          onRequestDelete={onTagDetailDelete}
          onRequestClose={onTagDetailClose}
        />
      {:else if selectedDriver}
        <div class="info-panel stacked-panel">
          <DriverDetailPanel
            driver={selectedDriver}
            mode="detail"
            onRequestDelete={onDriverDelete}
            onDone={onDriverDone}
          />

          <section class="edit-section">
            <h4>接続先配下の周期一括変更</h4>
            <p class="helper-text compact">この接続先に属する全 ScanGroup の周期を同一値へ更新します。</p>
            <label class="field-label" for="driver-bulk-scan-rate">周期 (ms)</label>
            <div class="inline-form">
              <input
                id="driver-bulk-scan-rate"
                type="number"
                min={MIN_SCAN_RATE_MS}
                step="100"
                bind:value={driverBulkScanRateMs}
                disabled={updatingDriverBulkRate}
              />
              <button
                class="btn-primary"
                onclick={submitDriverBulkRate}
                disabled={updatingDriverBulkRate}
              >
                {updatingDriverBulkRate ? '更新中...' : '全 ScanGroup に適用'}
              </button>
            </div>
          </section>

          {#if localErrorMessage}
            <p class="error-text">{localErrorMessage}</p>
          {/if}
        </div>
      {:else if selectedScanGroup}
        <div class="info-panel">
          <div class="panel-header compact">
            <h3>{selectedScanGroup.id}</h3>
            <span class="mode-label">ScanGroup</span>
          </div>
          <dl class="detail-list compact">
            <dt>ドライバ</dt><dd class="mono">{selectedScanGroup.driver_id}</dd>
            <dt>周期</dt><dd>{selectedScanGroup.scan_rate_ms ?? '-'} ms</dd>
            <dt>テーブル</dt><dd class="mono">{selectedScanGroup.table ?? '-'}</dd>
            <dt>時系列列</dt><dd class="mono">{selectedScanGroup.timestamp_column ?? '-'}</dd>
          </dl>
          <section class="edit-section">
            <h4>周期変更</h4>
            <p class="helper-text compact">構造変更はドライバ UI 側で行い、本体では周期のみ更新できます。</p>
            <label class="field-label" for="scan-group-rate">周期 (ms)</label>
            <div class="inline-form">
              <input
                id="scan-group-rate"
                type="number"
                min={MIN_SCAN_RATE_MS}
                step="100"
                bind:value={scanGroupRateMs}
                disabled={updatingScanGroupRate}
              />
              <button
                class="btn-primary"
                onclick={submitScanGroupRate}
                disabled={updatingScanGroupRate}
              >
                {updatingScanGroupRate ? '更新中...' : 'この ScanGroup に適用'}
              </button>
            </div>
          </section>
          {#if localErrorMessage}
            <p class="error-text">{localErrorMessage}</p>
          {/if}
          <p class="helper-text">Scanグループの追加・削除・構造変更は接続先ドライバ専用UI側で行います。</p>
        </div>
      {:else}
        <div class="empty-right">
          <p>ツリーから接続先 / Scanグループ / タグを選択してください</p>
        </div>
      {/if}
    {:else}
      <div class="empty-right">
        <p>左の一覧から<br />項目を選択してください</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .right-pane {
    width: 280px;
    min-width: 240px;
    border-left: 1px solid #dbe2ea;
    background-color: #fff;
    display: flex;
    flex-direction: column;
  }

  .right-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .empty-right {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #b0bec5;
    font-size: 0.85rem;
    text-align: center;
    padding: 20px;
  }

  .info-panel {
    padding: 16px;
  }

  .stacked-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .panel-header.compact {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    border-bottom: 1px solid #e3e8ef;
    padding-bottom: 10px;
  }

  .panel-header.compact h3 {
    margin: 0;
    font-size: 1rem;
    color: #2c3e50;
    word-break: break-all;
  }

  .mode-label {
    color: #64748b;
    font-size: 0.78rem;
    background: #f1f5f9;
    border-radius: 999px;
    padding: 0.2rem 0.6rem;
  }

  .detail-list.compact {
    display: grid;
    grid-template-columns: 6em 1fr;
    gap: 6px 10px;
    margin: 0;
    font-size: 0.85rem;
  }

  .detail-list.compact dt {
    color: #7f8c8d;
    font-weight: 600;
  }

  .detail-list.compact dd {
    margin: 0;
    word-break: break-all;
  }

  .helper-text {
    margin-top: 12px;
    font-size: 0.8rem;
    color: #64748b;
    line-height: 1.5;
  }

  .helper-text.compact {
    margin-top: 0;
    margin-bottom: 8px;
  }

  .edit-section {
    border-top: 1px solid #e3e8ef;
    padding-top: 12px;
  }

  .edit-section h4 {
    margin: 0 0 8px;
    font-size: 0.9rem;
    color: #334155;
  }

  .field-label {
    display: block;
    font-size: 0.8rem;
    color: #64748b;
    margin-bottom: 6px;
  }

  .inline-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .inline-form input {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font: inherit;
    box-sizing: border-box;
  }

  .btn-primary {
    background-color: #2563eb;
    color: #fff;
    border: none;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.84rem;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: #1d4ed8;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-text {
    margin: 0;
    color: #b91c1c;
    font-size: 0.8rem;
  }

  .mono {
    font-family: monospace;
  }

</style>
