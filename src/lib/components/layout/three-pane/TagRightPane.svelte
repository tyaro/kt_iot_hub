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
    onTagEditorDone: () => void | Promise<void>;
    onTagEditorCancel: () => void;
    onTagDetailEdit: (tag: TagDto) => void;
    onTagDetailDelete: (tag: TagDto) => void | Promise<void>;
    onTagDetailClose: () => void;
    onDriverDelete: (driverId: string) => void | Promise<void>;
    onDriverDone: (driverId?: string) => void | Promise<DriverDto | null>;
  } = $props();
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
        <DriverDetailPanel
          driver={selectedDriver}
          mode="detail"
          onRequestDelete={onDriverDelete}
          onDone={onDriverDone}
        />
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
          <p class="helper-text">Scanグループの追加・変更は接続先ドライバ専用UI側で行います。</p>
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

  .mono {
    font-family: monospace;
  }

</style>
