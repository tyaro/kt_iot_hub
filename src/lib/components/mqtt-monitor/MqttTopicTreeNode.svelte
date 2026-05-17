<script lang="ts">
  import MqttTopicTreeNode from './MqttTopicTreeNode.svelte';
  import type { TopicTreeNode } from './tree';

  let {
    node,
    selectedPath,
    onSelect,
  }: {
    node: TopicTreeNode;
    selectedPath: string | null;
    onSelect: (path: string) => void;
  } = $props();

  const hasChildren = $derived(node.children.length > 0);
  const isSelected = $derived(selectedPath === node.fullPath);
  const payloadPreview = $derived(
    !hasChildren && node.latestMessage ? `${node.label} = ${node.latestMessage.payload}` : node.label,
  );
</script>

{#if hasChildren}
  <details open class="tree-node">
    <summary>
      <button class="tree-button" class:selected={isSelected} onclick={() => onSelect(node.fullPath)}>
        {node.label}
      </button>
    </summary>
    <div class="tree-children">
      {#each node.children as child (child.id)}
        <MqttTopicTreeNode node={child} {selectedPath} {onSelect} />
      {/each}
    </div>
  </details>
{:else}
  <button class="tree-button leaf" class:selected={isSelected} onclick={() => onSelect(node.fullPath)}>
    {payloadPreview}
  </button>
{/if}

<style>
  .tree-node {
    margin: 2px 0;
  }

  summary {
    list-style: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  .tree-children {
    margin-left: 14px;
    border-left: 1px solid #e2e8f0;
    padding-left: 8px;
  }

  .tree-button {
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    padding: 4px 6px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.82rem;
    color: #1f2937;
  }

  .tree-button:hover {
    background: #eff6ff;
  }

  .tree-button.selected {
    background: #dbeafe;
    color: #1d4ed8;
    font-weight: 700;
  }

  .leaf {
    font-family: Consolas, monospace;
    word-break: break-all;
  }
</style>
