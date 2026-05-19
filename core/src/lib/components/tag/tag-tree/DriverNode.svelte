<script lang="ts">
  let {
    isExpanded,
    isSelected,
    label,
    onToggle,
    onSelect,
    onDoubleClick,
    onContextMenu,
  }: {
    isExpanded: boolean;
    isSelected: boolean;
    label: string;
    onToggle: () => void;
    onSelect: () => void;
    onDoubleClick: (event: MouseEvent) => void;
    onContextMenu: (event: MouseEvent) => void;
  } = $props();
</script>

<div class="tree-node driver-node">
  <button
    class="tree-toggle"
    onclick={onToggle}
    title={isExpanded ? '折畳む' : '展開'}
  >
    {isExpanded ? '▼' : '▶'}
  </button>
  <button
    type="button"
    class="tree-label tree-label-btn driver-label"
    class:selected={isSelected}
    onclick={onSelect}
    ondblclick={onDoubleClick}
    oncontextmenu={onContextMenu}
    title="クリックで選択 / ダブルクリックで接続先設定を編集"
  >
    🧩 {label}
  </button>
</div>

<style>
  .tree-node {
    user-select: none;
    padding: 0.25rem 0;
  }

  .driver-node {
    background: #f5f5f5;
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

  .tree-label-btn.selected {
    background: #dbeafe;
    border-radius: 4px;
  }
</style>
