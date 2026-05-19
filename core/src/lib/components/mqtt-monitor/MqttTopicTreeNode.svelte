<script lang="ts">
  import MqttTopicTreeNode from './MqttTopicTreeNode.svelte';
  import type { TopicTreeNode } from './tree';

  let {
    node,
    selectedPath,
    expandedPaths,
    highlightedPaths,
    onSelect,
    onToggle,
  }: {
    node: TopicTreeNode;
    selectedPath: string | null;
    expandedPaths: string[];
    highlightedPaths: string[];
    onSelect: (path: string) => void;
    onToggle: (path: string) => void;
  } = $props();

  const hasChildren = $derived(node.hasChildren);
  const isSelected = $derived(selectedPath === node.fullPath);
  const isHighlighted = $derived(highlightedPaths.includes(node.fullPath));
  const isOpen = $derived(hasChildren && expandedPaths.includes(node.fullPath));
  const payloadPreview = $derived(
    !hasChildren && node.latestMessage
      ? node.latestMessage.payload.length > 48
        ? `${node.latestMessage.payload.slice(0, 48)}…`
        : node.latestMessage.payload
      : '',
  );
</script>

{#if hasChildren}
  <div class="tree-node branch">
    <div class="tree-row" class:selected={isSelected} class:highlighted={isHighlighted}>
      <button
        class="toggle-button"
        type="button"
        aria-label={isOpen ? `${node.label} を折りたたむ` : `${node.label} を展開する`}
        aria-expanded={isOpen}
        onclick={() => onToggle(node.fullPath)}
      >
        <span class:open={isOpen}>▸</span>
      </button>
      <button class="tree-button" type="button" class:selected={isSelected} onclick={() => onSelect(node.fullPath)}>
        <span class="node-label">{node.label}</span>
      </button>
    </div>

    {#if isOpen}
      <div class="tree-children">
        {#each node.children as child (child.id)}
          <MqttTopicTreeNode node={child} {selectedPath} {expandedPaths} {highlightedPaths} {onSelect} {onToggle} />
        {/each}
      </div>
    {/if}
  </div>
{:else}
  <div class="tree-node leaf">
    <div class="tree-row" class:selected={isSelected} class:highlighted={isHighlighted}>
      <span class="toggle-spacer"></span>
      <button class="tree-button leaf" type="button" class:selected={isSelected} onclick={() => onSelect(node.fullPath)}>
        <span class="node-label">{node.label}</span>
        {#if payloadPreview}
          <span class="payload-preview">{payloadPreview}</span>
        {/if}
      </button>
    </div>
  </div>
{/if}

<style>
  .tree-node {
    margin: 1px 0;
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: 6px;
  }

  .tree-row.selected {
    background: #dbeafe;
  }

  .tree-row.highlighted:not(.selected) {
    background: #eefbf3;
  }

  .tree-children {
    margin-left: 10px;
    padding-left: 10px;
    border-left: 1px solid #e2e8f0;
  }

  .toggle-button,
  .toggle-spacer {
    width: 18px;
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #64748b;
  }

  .toggle-button {
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
  }

  .toggle-button:hover {
    background: #e2e8f0;
  }

  .toggle-button span {
    display: inline-block;
    transition: transform 0.12s ease;
  }

  .toggle-button span.open {
    transform: rotate(90deg);
  }

  .tree-button {
    width: 100%;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    text-align: left;
    background: transparent;
    border: none;
    padding: 5px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    color: #1f2937;
  }

  .tree-button:hover {
    background: #eff6ff;
  }

  .tree-button.selected {
    color: #1d4ed8;
    font-weight: 700;
  }

  .tree-row.highlighted .tree-button:not(.selected) {
    color: #166534;
  }

  .node-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .payload-preview {
    flex: 0 0 auto;
    max-width: 48%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #64748b;
    font-size: 0.75rem;
    font-family: Consolas, monospace;
  }

  .leaf {
    font-family: Consolas, monospace;
  }
</style>
