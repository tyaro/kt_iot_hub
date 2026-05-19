<script lang="ts">
  import type { TagDto } from '$lib/ipc';
  import type { ScanGroupDto } from '$lib/ipc';
  import type { ContextMenuState } from './types';

  let {
    menu,
    onRequestNewTag,
    onRequestEditDriverScanRate,
    onRequestEditDriver,
    onRequestDeleteDriver,
    onRequestEditScanGroupRate,
    onRequestEditTag,
    onRequestDeleteTag,
  }: {
    menu: ContextMenuState;
    onRequestNewTag: (driverId: string) => void;
    onRequestEditDriverScanRate: (driverId: string) => void;
    onRequestEditDriver: (driverId: string) => void;
    onRequestDeleteDriver: (driverId: string) => void;
    onRequestEditScanGroupRate: (scanGroup: ScanGroupDto) => void;
    onRequestEditTag: (tag: TagDto) => void;
    onRequestDeleteTag: (tag: TagDto) => void;
  } = $props();
</script>

{#if menu.open && menu.kind === 'driver' && menu.driverId}
  <div
    class="context-menu"
    style={`left: ${menu.x}px; top: ${menu.y}px;`}
    role="menu"
    aria-label="タグツリー操作メニュー"
    tabindex="-1"
  >
    <button class="context-item" onclick={() => onRequestEditDriverScanRate(menu.driverId!)}>
      周期一括変更
    </button>
    <button class="context-item" onclick={() => onRequestNewTag(menu.driverId!)}>
      タグ追加
    </button>
    <button class="context-item" onclick={() => onRequestEditDriver(menu.driverId!)}>
      全タグ編集
    </button>
    <button class="context-item danger" onclick={() => onRequestDeleteDriver(menu.driverId!)}>
      接続先削除
    </button>
  </div>
{:else if menu.open && menu.kind === 'scan-group' && menu.scanGroup}
  <div
    class="context-menu"
    style={`left: ${menu.x}px; top: ${menu.y}px;`}
    role="menu"
    aria-label="タグツリー操作メニュー"
    tabindex="-1"
  >
    <button class="context-item" onclick={() => onRequestEditScanGroupRate(menu.scanGroup!)}>
      周期変更
    </button>
  </div>
{:else if menu.open && menu.kind === 'tag' && menu.tag}
  <div
    class="context-menu"
    style={`left: ${menu.x}px; top: ${menu.y}px;`}
    role="menu"
    aria-label="タグツリー操作メニュー"
    tabindex="-1"
  >
    <button class="context-item" onclick={() => onRequestEditTag(menu.tag!)}>
      編集
    </button>
    <button class="context-item danger" onclick={() => onRequestDeleteTag(menu.tag!)}>
      削除
    </button>
  </div>
{/if}

<style>
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
</style>
