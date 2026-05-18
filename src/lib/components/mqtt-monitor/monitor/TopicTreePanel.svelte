<script lang="ts">
  import type { TopicTreeNode } from '../tree';
  import MqttTopicTreeNode from '../MqttTopicTreeNode.svelte';

  let {
    loading,
    broker,
    treeNodes,
    selectedPath,
    expandedPaths,
    highlightedPaths,
    onSelectPath,
    onTogglePath,
  }: {
    loading: boolean;
    broker: string;
    treeNodes: TopicTreeNode[];
    selectedPath: string | null;
    expandedPaths: string[];
    highlightedPaths: string[];
    onSelectPath: (path: string) => void;
    onTogglePath: (path: string) => void;
  } = $props();
</script>

<aside class="tree-panel">
  <div class="panel-title-row">
    <div>
      <div class="panel-title">Topics</div>
      <div class="panel-subtitle">{broker || 'broker'}</div>
    </div>
  </div>
  {#if loading}
    <p class="muted">読込中...</p>
  {:else if treeNodes.length === 0}
    <p class="muted">まだ受信メッセージはありません</p>
  {:else}
    <div class="tree-root">
      {#each treeNodes as node (node.id)}
        <MqttTopicTreeNode
          node={node}
          {selectedPath}
          {expandedPaths}
          {highlightedPaths}
          onSelect={onSelectPath}
          onToggle={onTogglePath}
        />
      {/each}
    </div>
  {/if}
</aside>

<style>
  .tree-panel {
    min-height: 0;
    background: #ffffff;
    border: 1px solid #cfd8e3;
    border-radius: 8px;
    padding: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 1px 2px rgb(15 23 42 / 0.06);
  }

  .panel-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%);
    border-bottom: 1px solid #e2e8f0;
  }

  .panel-title {
    font-size: 0.78rem;
    font-weight: 700;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-subtitle {
    margin-top: 4px;
    font-size: 0.78rem;
    color: #64748b;
  }

  .tree-root {
    min-height: 0;
    overflow: auto;
    padding: 10px;
    font-size: 0.8rem;
    font-family: Consolas, monospace;
  }

  .muted {
    margin: 0;
    padding: 14px;
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
