<script lang="ts">
  import { tagsStore, reloadTags } from '$lib/stores/index';
  import type { TagDto } from '$lib/ipc/index';

  interface Props {
    onSelect?: (tag: TagDto | null) => void;
    selectedId?: string | null;
  }
  let { onSelect = () => {}, selectedId = null }: Props = $props();

  function selectTag(tag: TagDto) {
    onSelect(tag);
  }

  $effect(() => {
    reloadTags();
  });
</script>

<div class="tag-table-wrap">
  <div class="toolbar">
    <button
      class="btn-primary"
      onclick={() => reloadTags()}
      disabled={$tagsStore.loading}
    >
      {$tagsStore.loading ? '読み込み中...' : '再読み込み'}
    </button>
  </div>

  {#if $tagsStore.error}
    <p class="error">{$tagsStore.error}</p>
  {/if}

  <table class="tag-table">
    <thead>
      <tr>
        <th>ID</th>
        <th>名前</th>
        <th>型</th>
        <th>ドライバ</th>
        <th>スキャングループ</th>
      </tr>
    </thead>
    <tbody>
      {#if $tagsStore.items.length === 0 && !$tagsStore.loading}
        <tr>
          <td colspan="5" class="empty">タグがありません</td>
        </tr>
      {:else}
        {#each $tagsStore.items as tag (tag.id)}
          <tr
            class:selected={tag.id === selectedId}
            onclick={() => selectTag(tag)}
          >
            <td>{tag.id}</td>
            <td>{tag.name}</td>
            <td><span class="badge">{tag.data_type}</span></td>
            <td>{tag.driver_id}</td>
            <td>{tag.scan_group_id}</td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>

<style>
  .toolbar {
    margin-bottom: 10px;
  }

  .tag-table {
    width: 100%;
    border-collapse: collapse;
    background: #fff;
    border: 1px solid #dbe2ea;
    font-size: 0.85rem;
  }

  .tag-table th,
  .tag-table td {
    padding: 8px 10px;
    border-bottom: 1px solid #ecf0f1;
    text-align: left;
  }

  .tag-table th {
    background-color: #f6f8fb;
    font-weight: 600;
    color: #5a6776;
  }

  .tag-table tr:hover {
    background-color: #f0f7ff;
    cursor: pointer;
  }

  .tag-table tr.selected {
    background-color: #dbeafe;
  }

  .badge {
    background: #e8f0fe;
    color: #1a56db;
    border-radius: 3px;
    padding: 1px 6px;
    font-size: 0.8rem;
    font-family: monospace;
  }

  .empty {
    text-align: center;
    color: #95a5a6;
    padding: 20px;
  }

  .error {
    color: #c0392b;
    margin: 8px 0;
  }

  .btn-primary {
    background-color: #3498db;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
