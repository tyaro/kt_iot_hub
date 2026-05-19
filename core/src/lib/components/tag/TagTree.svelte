<script lang="ts">
  import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc/index';
  import { driversStore, reloadDrivers, reloadScanGroups, reloadTags, scanGroupsStore, tagsStore } from '$lib/stores/index';
  import ContextMenu from './tag-tree/ContextMenu.svelte';
  import DriverNode from './tag-tree/DriverNode.svelte';
  import ScanGroupNode from './tag-tree/ScanGroupNode.svelte';
  import TagNode from './tag-tree/TagNode.svelte';
  import { buildGroupedTree } from './tag-tree/treeBuilder';
  import type { ContextMenuState } from './tag-tree/types';

  interface Props {
    onSelect?: (tag: TagDto | null) => void;
    onSelectDriver?: (driver: DriverDto | null) => void;
    onSelectScanGroup?: (scanGroup: ScanGroupDto | null) => void;
    selectedTagId?: string | null;
    selectedDriverId?: string | null;
    selectedScanGroupId?: string | null;
    onRequestNewTag?: (driverId: string) => void;
    onRequestEditDriver?: (driverId: string) => void;
    onRequestDeleteDriver?: (driverId: string) => void;
    onRequestEditTag?: (tag: TagDto) => void;
    onRequestDeleteTag?: (tag: TagDto) => void;
  }

  let {
    onSelect = () => {},
    onSelectDriver = () => {},
    onSelectScanGroup = () => {},
    selectedTagId = null,
    selectedDriverId = null,
    selectedScanGroupId = null,
    onRequestNewTag = () => {},
    onRequestEditDriver = () => {},
    onRequestDeleteDriver = () => {},
    onRequestEditTag = () => {},
    onRequestDeleteTag = () => {},
  }: Props = $props();

  let expandedDrivers = $state(new Set<string>());
  let expandedScanGroups = $state(new Set<string>());
  let contextMenu = $state<ContextMenuState>({ open: false, x: 0, y: 0 });

  const groupedTree = $derived(
    buildGroupedTree($driversStore.items, $scanGroupsStore.items, $tagsStore.items),
  );

  function resolveDriverLabel(driverId: string) {
    const driver = $driversStore.items.find((item) => item.id === driverId);
    if (!driver) {
      return driverId;
    }
    return `${driver.id} (${driver.driver_type})`;
  }

  function toggleDriver(driverId: string) {
    if (expandedDrivers.has(driverId)) {
      expandedDrivers.delete(driverId);
    } else {
      expandedDrivers.add(driverId);
    }
    expandedDrivers = new Set(expandedDrivers);
  }

  function toggleScanGroup(scanGroupId: string) {
    if (expandedScanGroups.has(scanGroupId)) {
      expandedScanGroups.delete(scanGroupId);
    } else {
      expandedScanGroups.add(scanGroupId);
    }
    expandedScanGroups = new Set(expandedScanGroups);
  }

  function selectTag(tag: TagDto) {
    onSelect(tag);
    closeContextMenu();
  }

  function selectDriver(driver: DriverDto) {
    onSelectDriver(driver);
    closeContextMenu();
  }

  function selectScanGroup(scanGroup: ScanGroupDto) {
    onSelectScanGroup(scanGroup);
    closeContextMenu();
  }

  function openDriverContextMenu(event: MouseEvent, driverId: string) {
    event.preventDefault();
    contextMenu = {
      open: true,
      x: event.clientX,
      y: event.clientY,
      kind: 'driver',
      driverId,
    };
  }

  function openScanGroupContextMenu(event: MouseEvent, scanGroup: ScanGroupDto) {
    event.preventDefault();
    contextMenu = {
      open: true,
      x: event.clientX,
      y: event.clientY,
      kind: 'scan-group',
      scanGroup,
    };
  }

  function openTagContextMenu(event: MouseEvent, tag: TagDto) {
    event.preventDefault();
    selectTag(tag);
    contextMenu = {
      open: true,
      x: event.clientX,
      y: event.clientY,
      kind: 'tag',
      tag,
    };
  }

  function closeContextMenu() {
    if (!contextMenu.open) {
      return;
    }
    contextMenu = {
      open: false,
      x: 0,
      y: 0,
      kind: undefined,
      driverId: undefined,
      scanGroup: undefined,
      tag: undefined,
    };
  }

  function requestNewTag(driverId: string) {
    onRequestNewTag(driverId);
    closeContextMenu();
  }

  function requestDeleteDriver(driverId: string) {
    onRequestDeleteDriver(driverId);
    closeContextMenu();
  }

  function requestEditDriver(driverId: string) {
    onRequestEditDriver(driverId);
    closeContextMenu();
  }

  function requestEditDriverScanRate(driverId: string) {
    const driver = $driversStore.items.find((item) => item.id === driverId);
    if (driver) {
      onSelectDriver(driver);
    }
    closeContextMenu();
  }

  function requestEditScanGroupRate(scanGroup: ScanGroupDto) {
    onSelectScanGroup(scanGroup);
    closeContextMenu();
  }

  function requestEditTag(tag: TagDto) {
    onRequestEditTag(tag);
    closeContextMenu();
  }

  function requestDeleteTag(tag: TagDto) {
    onRequestDeleteTag(tag);
    closeContextMenu();
  }

  function handleDriverDoubleClick(event: MouseEvent, driver: DriverDto) {
    event.preventDefault();
    selectDriver(driver);
    requestEditDriver(driver.id);
  }

  $effect(() => {
    reloadDrivers();
    reloadScanGroups();
    reloadTags();
  });
</script>

<svelte:window onclick={closeContextMenu} onblur={closeContextMenu} />

<div class="tag-tree">
  <div class="toolbar">
    <button
      class="btn-reload secondary"
      onclick={() => {
        reloadDrivers();
        reloadScanGroups();
      }}
      disabled={$driversStore.loading || $scanGroupsStore.loading}
    >
      {($driversStore.loading || $scanGroupsStore.loading) ? '同期中...' : '接続先を同期'}
    </button>
    <button class="btn-reload" onclick={() => reloadTags()} disabled={$tagsStore.loading}>
      {$tagsStore.loading ? '読み込み中...' : '再読み込み'}
    </button>
  </div>

  {#if $tagsStore.error}
    <p class="error">{$tagsStore.error}</p>
  {/if}

  {#if $driversStore.items.length === 0 && !$driversStore.loading}
    <p class="empty">接続先がありません</p>
  {:else}
    <div class="tree-container">
      {#each groupedTree as node (node.driver.id)}
        {@const driver = node.driver}
        {@const isExpanded = expandedDrivers.has(driver.id)}

        <DriverNode
          {isExpanded}
          isSelected={selectedDriverId === driver.id}
          label={resolveDriverLabel(driver.id)}
          onToggle={() => toggleDriver(driver.id)}
          onSelect={() => selectDriver(driver)}
          onDoubleClick={(event) => handleDriverDoubleClick(event, driver)}
          onContextMenu={(event) => openDriverContextMenu(event, driver.id)}
        />

        {#if isExpanded}
          {#if node.scanGroups.length === 0}
            <div class="empty-node scan-empty-node">
              <div class="indent-1">
                <span class="muted">Scanグループ未登録</span>
              </div>
            </div>
          {/if}

          {#each node.scanGroups as scanNode (scanNode.scanGroup.id)}
            {@const scanGroup = scanNode.scanGroup}
            {@const isGroupExpanded = expandedScanGroups.has(scanGroup.id)}

            <ScanGroupNode
              {scanGroup}
              isExpanded={isGroupExpanded}
              isSelected={selectedScanGroupId === scanGroup.id}
              onToggle={() => toggleScanGroup(scanGroup.id)}
              onSelect={() => selectScanGroup(scanGroup)}
              onContextMenu={(event) => openScanGroupContextMenu(event, scanGroup)}
            />

            {#if isGroupExpanded}
              {#if scanNode.tags.length === 0}
                <div class="empty-node tag-empty-node">
                  <div class="indent-2">
                    <span class="muted">タグ未登録</span>
                  </div>
                </div>
              {:else}
                {#each scanNode.tags as tag (tag.id)}
                  <TagNode
                    {tag}
                    isSelected={tag.id === selectedTagId}
                    onSelect={() => selectTag(tag)}
                    onContextMenu={(event) => openTagContextMenu(event, tag)}
                  />
                {/each}
              {/if}
            {/if}
          {/each}
        {/if}
      {/each}
    </div>
  {/if}

  <ContextMenu
    menu={contextMenu}
    onRequestNewTag={requestNewTag}
    onRequestEditDriverScanRate={requestEditDriverScanRate}
    onRequestEditDriver={requestEditDriver}
    onRequestDeleteDriver={requestDeleteDriver}
    onRequestEditScanGroupRate={requestEditScanGroupRate}
    onRequestEditTag={requestEditTag}
    onRequestDeleteTag={requestDeleteTag}
  />
</div>

<style>
  .tag-tree {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0.5rem;
    padding: 0.5rem;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    gap: 0.5rem;
  }

  .btn-reload {
    padding: 0.4rem 0.8rem;
    background: #0066cc;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .btn-reload:hover:not(:disabled) {
    background: #0052a3;
  }

  .btn-reload.secondary {
    background: #475569;
  }

  .btn-reload.secondary:hover:not(:disabled) {
    background: #334155;
  }

  .btn-reload:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    color: #d32f2f;
    font-size: 0.875rem;
    margin: 0.5rem 0;
  }

  .empty {
    color: #666;
    font-size: 0.875rem;
    text-align: center;
    padding: 1rem;
  }

  .tree-container {
    flex: 1;
    overflow-y: auto;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    font-size: 0.875rem;
  }

  .indent-1 {
    display: flex;
    align-items: center;
    padding-left: 1.5rem;
  }

  .indent-2 {
    display: flex;
    align-items: center;
    padding-left: 3rem;
  }

  .empty-node {
    color: #94a3b8;
    padding: 0.25rem 0;
  }

  .scan-empty-node {
    background: #fafafa;
  }

  .tag-empty-node {
    background: white;
  }

  .muted {
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
