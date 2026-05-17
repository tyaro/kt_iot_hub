<script lang="ts">
  import { driversStore, scanGroupsStore, tagsStore, reloadDrivers, reloadScanGroups, reloadTags } from '$lib/stores/index';
  import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc/index';

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

  interface ContextMenuState {
    open: boolean;
    x: number;
    y: number;
    kind?: 'driver' | 'tag';
    driverId?: string;
    tag?: TagDto;
  }

  // 展開状態管理
  let expandedDrivers = $state(new Set<string>());
  let expandedScanGroups = $state(new Set<string>());
  let contextMenu = $state<ContextMenuState>({ open: false, x: 0, y: 0 });

  function resolveDriverLabel(driverId: string) {
    const driver = $driversStore.items.find((item) => item.id === driverId);
    if (!driver) {
      return driverId;
    }
    return `${driver.id} (${driver.driver_type})`;
  }

  let groupedTree = $derived.by(() => {
    const tagsByScanGroup = new Map<string, TagDto[]>();

    $tagsStore.items.forEach((tag) => {
      if (!tagsByScanGroup.has(tag.scan_group_id)) {
        tagsByScanGroup.set(tag.scan_group_id, []);
      }
      tagsByScanGroup.get(tag.scan_group_id)!.push(tag);
    });

    const scanGroupsByDriver = new Map<string, ScanGroupDto[]>();
    $scanGroupsStore.items.forEach((scanGroup) => {
      if (!scanGroupsByDriver.has(scanGroup.driver_id)) {
        scanGroupsByDriver.set(scanGroup.driver_id, []);
      }
      scanGroupsByDriver.get(scanGroup.driver_id)!.push(scanGroup);
    });

    return [...$driversStore.items]
      .sort((a, b) => a.id.localeCompare(b.id))
      .map((driver) => {
        const scanGroups = [...(scanGroupsByDriver.get(driver.id) ?? [])]
          .sort((a, b) => a.id.localeCompare(b.id))
          .map((scanGroup) => ({
            scanGroup,
            tags: [...(tagsByScanGroup.get(scanGroup.id) ?? [])]
              .sort((a, b) => a.name.localeCompare(b.name)),
          }));

        return { driver, scanGroups };
      });
  });

  // ツリー操作
  function toggleDriver(driverId: string) {
    if (expandedDrivers.has(driverId)) {
      expandedDrivers.delete(driverId);
    } else {
      expandedDrivers.add(driverId);
    }
    expandedDrivers = new Set(expandedDrivers);
  }

  function toggleScanGroup(scanGroupId: string) {
    const key = scanGroupId;
    if (expandedScanGroups.has(key)) {
      expandedScanGroups.delete(key);
    } else {
      expandedScanGroups.add(key);
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
    if (!contextMenu.open) return;
    contextMenu = { open: false, x: 0, y: 0, kind: undefined, driverId: undefined, tag: undefined };
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

  function requestEditTag(tag: TagDto) {
    onRequestEditTag(tag);
    closeContextMenu();
  }

  function requestDeleteTag(tag: TagDto) {
    onRequestDeleteTag(tag);
    closeContextMenu();
  }

  // 初期ロード
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
    <button
      class="btn-reload"
      onclick={() => reloadTags()}
      disabled={$tagsStore.loading}
    >
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
        {@const driverId = driver.id}
        {@const isExpanded = expandedDrivers.has(driverId)}

        <div class="tree-node driver-node">
          <button
            class="tree-toggle"
            onclick={() => toggleDriver(driverId)}
            title={isExpanded ? '折畳む' : '展開'}
          >
            {isExpanded ? '▼' : '▶'}
          </button>
          <button
            type="button"
            class="tree-label tree-label-btn driver-label"
            class:selected={selectedDriverId === driverId}
            onclick={() => selectDriver(driver)}
            oncontextmenu={(event) => openDriverContextMenu(event, driverId)}
          >
              🧩 {resolveDriverLabel(driverId)}
          </button>
        </div>

        {#if isExpanded}
          {#if node.scanGroups.length === 0}
            <div class="tree-node scan-group-node empty-node">
              <div class="indent-1">
                <span class="tree-label scan-group-label muted">Scanグループ未登録</span>
              </div>
            </div>
          {/if}

          {#each node.scanGroups as scanNode (scanNode.scanGroup.id)}
            {@const scanGroup = scanNode.scanGroup}
            {@const scanGroupId = scanGroup.id}
            {@const isGroupExpanded = expandedScanGroups.has(scanGroupId)}
            {@const tags = scanNode.tags}

            <div class="tree-node scan-group-node">
              <div class="indent-1">
                <button
                  class="tree-toggle"
                  onclick={() => toggleScanGroup(scanGroupId)}
                  title={isGroupExpanded ? '折畳む' : '展開'}
                >
                  {isGroupExpanded ? '▼' : '▶'}
                </button>
                <button
                  type="button"
                  class="tree-label tree-label-btn scan-group-label"
                  class:selected={selectedScanGroupId === scanGroupId}
                  onclick={() => selectScanGroup(scanGroup)}
                >
                  📊 {scanGroupId}
                </button>
              </div>
            </div>

            {#if isGroupExpanded}
              {#if tags.length === 0}
                <div class="tree-node tag-node empty-node">
                  <div class="indent-2">
                    <span class="tree-label muted">タグ未登録</span>
                  </div>
                </div>
              {:else}
                {#each tags as tag (tag.id)}
                  <div class="tree-node tag-node">
                    <div class="indent-2">
                      <button
                        class="tree-tag-btn"
                        class:selected={tag.id === selectedTagId}
                        onclick={() => selectTag(tag)}
                        oncontextmenu={(event) => openTagContextMenu(event, tag)}
                        title={`${tag.name} (${tag.data_type})`}
                      >
                        <span class="tag-icon">🏷️</span>
                        <span class="tag-name">{tag.name}</span>
                        <span class="tag-type">({tag.data_type})</span>
                      </button>
                    </div>
                  </div>
                {/each}
              {/if}
            {/if}
          {/each}
        {/if}
      {/each}
    </div>
  {/if}

  {#if contextMenu.open && contextMenu.kind === 'driver' && contextMenu.driverId}
    <div
      class="context-menu"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
      role="menu"
      aria-label="タグツリー操作メニュー"
      tabindex="-1"
    >
      <button class="context-item" onclick={() => requestNewTag(contextMenu.driverId!)}>
        タグ追加
      </button>
      <button class="context-item" onclick={() => requestEditDriver(contextMenu.driverId!)}>
        全タグ編集
      </button>
      <button class="context-item danger" onclick={() => requestDeleteDriver(contextMenu.driverId!)}>
        接続先削除
      </button>
    </div>
  {:else if contextMenu.open && contextMenu.kind === 'tag' && contextMenu.tag}
    <div
      class="context-menu"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
      role="menu"
      aria-label="タグツリー操作メニュー"
      tabindex="-1"
    >
      <button class="context-item" onclick={() => requestEditTag(contextMenu.tag!)}>
        編集
      </button>
      <button class="context-item danger" onclick={() => requestDeleteTag(contextMenu.tag!)}>
        削除
      </button>
    </div>
  {/if}
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

  .tree-node {
    user-select: none;
    padding: 0.25rem 0;
  }

  .driver-node {
    background: #f5f5f5;
  }

  .scan-group-node {
    background: #fafafa;
  }

  .tag-node {
    background: white;
  }

  .tree-toggle {
    width: 20px;
    height: 20px;
    padding: 0;
    margin: 0 0.25rem;
    border: none;
    background: none;
    cursor: pointer;
    color: #666;
    font-size: 0.75rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .tree-toggle:hover {
    color: #333;
  }

  .tree-label {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.4rem;
    cursor: pointer;
  }

  .tree-label-btn {
    border: none;
    background: transparent;
    font: inherit;
  }

  .driver-label {
    font-weight: 600;
    color: #333;
  }

  .scan-group-label {
    border: none;
    background: transparent;
    font-weight: 500;
    color: #555;
  }

  .tree-label-btn.selected {
    background: #dbeafe;
    border-radius: 4px;
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

  .tree-tag-btn {
    flex: 1;
    text-align: left;
    padding: 0.3rem 0.4rem;
    border: none;
    background: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: #333;
    font-size: 0.875rem;
  }

  .tree-tag-btn:hover {
    background: #e3f2fd;
  }

  .tree-tag-btn.selected {
    background: #bbdefb;
    font-weight: 500;
  }

  .context-menu {
    position: fixed;
    z-index: 1200;
    min-width: 150px;
    background: #fff;
    border: 1px solid #d0d7de;
    border-radius: 6px;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.18);
    padding: 0.3rem;
    display: grid;
    gap: 0.2rem;
  }

  .context-item {
    border: none;
    background: transparent;
    text-align: left;
    font-size: 0.82rem;
    padding: 0.4rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    color: #334155;
  }

  .context-item:hover {
    background: #eff6ff;
  }

  .context-item.danger {
    color: #b91c1c;
  }

  .context-item.danger:hover {
    background: #fef2f2;
  }

  .tag-icon {
    display: inline-block;
    width: 1.2em;
    text-align: center;
  }

  .tag-name {
    font-weight: 500;
  }

  .tag-type {
    color: #999;
    font-size: 0.8rem;
  }

  .empty-node {
    color: #94a3b8;
  }

  .muted {
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
