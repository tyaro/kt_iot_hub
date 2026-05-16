<script lang="ts">
  import { listTags, type TagDto } from '../../ipc/index';

  let tags = $state<TagDto[]>([]);
  let loading = $state(false);
  let errorMessage = $state('');

  async function reload() {
    loading = true;
    errorMessage = '';
    try {
      tags = await listTags();
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : 'タグの取得に失敗しました';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    reload();
  });
</script>

<div class="tag-table-wrap">
  <div class="toolbar">
    <button class="btn-primary" onclick={reload} disabled={loading}>
      {loading ? '読み込み中...' : '再読み込み'}
    </button>
  </div>

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
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
      {#if tags.length === 0}
        <tr>
          <td colspan="5" class="empty">タグがありません</td>
        </tr>
      {:else}
        {#each tags as tag (tag.id)}
          <tr>
            <td>{tag.id}</td>
            <td>{tag.name}</td>
            <td>{tag.data_type}</td>
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
    margin-bottom: 12px;
  }

  .tag-table {
    width: 100%;
    border-collapse: collapse;
    background: #fff;
    border: 1px solid #dbe2ea;
  }

  .tag-table th,
  .tag-table td {
    padding: 8px 10px;
    border-bottom: 1px solid #ecf0f1;
    text-align: left;
    font-size: 0.85rem;
  }

  .tag-table th {
    background-color: #f6f8fb;
    font-weight: 600;
  }

  .empty {
    text-align: center;
    color: #95a5a6;
  }

  .error {
    color: #c0392b;
    margin: 8px 0;
  }

  .btn-primary {
    background-color: #3498db;
    color: #fff;
    border: none;
    padding: 8px 14px;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-primary:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
</style>
