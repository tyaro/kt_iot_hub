<script lang="ts">
  import type { TagDto } from '$lib/ipc';

  let {
    tag,
    isSelected,
    onSelect,
    onContextMenu,
  }: {
    tag: TagDto;
    isSelected: boolean;
    onSelect: () => void;
    onContextMenu: (event: MouseEvent) => void;
  } = $props();
</script>

<div class="tree-node tag-node">
  <div class="indent-2">
    <button
      class="tree-tag-btn"
      class:selected={isSelected}
      onclick={onSelect}
      oncontextmenu={onContextMenu}
      title={`${tag.name} (${tag.data_type})`}
    >
      <span class="tag-icon">🏷️</span>
      <span class="tag-name">{tag.name}</span>
      <span class="tag-type">({tag.data_type})</span>
    </button>
  </div>
</div>

<style>
  .tree-node {
    user-select: none;
    padding: 0.25rem 0;
  }

  .tag-node {
    background: white;
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
</style>
