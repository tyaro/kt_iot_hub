<script lang="ts">
  import TagTree from '../../tag/TagTree.svelte';
  import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';

  let {
    driverUiPolling,
    tagActionMessage,
    selectedTagId,
    selectedDriverId,
    selectedScanGroupId,
    onNewDriver,
    onSelectTag,
    onSelectDriver,
    onSelectScanGroup,
    onRequestNewTag,
    onRequestEditDriver,
    onRequestDeleteDriver,
    onRequestEditTag,
    onRequestDeleteTag,
  }: {
    driverUiPolling: boolean;
    tagActionMessage: string;
    selectedTagId: string | null;
    selectedDriverId: string | null;
    selectedScanGroupId: string | null;
    onNewDriver: () => void | Promise<void>;
    onSelectTag: (tag: TagDto | null) => void;
    onSelectDriver: (driver: DriverDto | null) => void;
    onSelectScanGroup: (scanGroup: ScanGroupDto | null) => void;
    onRequestNewTag: (driverId: string) => void;
    onRequestEditDriver: (driverId: string) => void;
    onRequestDeleteDriver: (driverId: string) => void | Promise<void>;
    onRequestEditTag: (tag: TagDto) => void;
    onRequestDeleteTag: (tag: TagDto) => void | Promise<void>;
  } = $props();
</script>

<div class="content">
  <div class="content-header">
    <h2>タグ管理</h2>
    <div class="header-actions">
      <button class="btn-primary" onclick={onNewDriver} disabled={driverUiPolling}>＋ 接続先</button>
    </div>
  </div>
  {#if tagActionMessage}
    <p class="action-message">{tagActionMessage}</p>
  {/if}
  <TagTree
    onSelect={onSelectTag}
    onSelectDriver={onSelectDriver}
    onSelectScanGroup={onSelectScanGroup}
    {selectedTagId}
    {selectedDriverId}
    {selectedScanGroupId}
    onRequestNewTag={onRequestNewTag}
    onRequestEditDriver={onRequestEditDriver}
    onRequestDeleteDriver={onRequestDeleteDriver}
    onRequestEditTag={onRequestEditTag}
    onRequestDeleteTag={onRequestDeleteTag}
  />
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .content-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .content h2 {
    margin: 0 0 20px;
    font-size: 1.1rem;
    color: #2c3e50;
    border-bottom: 2px solid #2e86c1;
    padding-bottom: 8px;
  }

  .content-header h2 {
    margin-bottom: 0;
    border-bottom: none;
    padding-bottom: 0;
  }

  .action-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #2563eb;
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 6px;
    padding: 8px 10px;
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

</style>
