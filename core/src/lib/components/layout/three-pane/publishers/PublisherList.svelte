<script lang="ts">
  import type { PublisherDto } from '$lib/ipc';

  let {
    loading,
    publishers,
    selectedPublisherId,
    onSelect,
  }: {
    loading: boolean;
    publishers: PublisherDto[];
    selectedPublisherId: string | null;
    onSelect: (publisher: PublisherDto) => void;
  } = $props();
</script>

<aside class="publisher-list-panel">
  <div class="panel-title">登録済み</div>
  {#if loading}
    <p class="muted">読込中...</p>
  {:else if publishers.length === 0}
    <p class="muted">まだパブリッシャはありません</p>
  {:else}
    <div class="publisher-list">
      {#each publishers as publisher (publisher.id)}
        <button
          class="publisher-item"
          class:selected={selectedPublisherId === publisher.id}
          onclick={() => onSelect(publisher)}
        >
          <span class="item-main">{publisher.id}</span>
          <span class="item-sub">{publisher.broker}:{publisher.port}</span>
        </button>
      {/each}
    </div>
  {/if}
</aside>

<style>
  .publisher-list-panel {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  .panel-title {
    margin-bottom: 12px;
    font-size: 0.85rem;
    font-weight: 700;
    color: #475569;
  }

  .publisher-list {
    display: grid;
    gap: 8px;
  }

  .publisher-item {
    display: grid;
    gap: 4px;
    text-align: left;
    border: 1px solid #d5deea;
    background: #fff;
    border-radius: 6px;
    padding: 10px 12px;
    cursor: pointer;
  }

  .publisher-item.selected {
    border-color: #2e86c1;
    background: #eff6ff;
  }

  .item-main {
    font-size: 0.88rem;
    font-weight: 700;
    color: #1f2937;
  }

  .item-sub {
    font-size: 0.78rem;
    color: #64748b;
    word-break: break-all;
  }

  .muted {
    margin: 0;
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
