<script lang="ts">
  import type { TagDto } from '$lib/ipc/index';

  interface Props {
    tag?: TagDto | null;
    onRequestEdit?: (tag: TagDto) => void;
    onRequestDelete?: (tag: TagDto) => void;
    onRequestClose?: () => void;
  }
  let {
    tag = null,
    onRequestEdit = () => {},
    onRequestDelete = () => {},
    onRequestClose = () => {},
  }: Props = $props();

  function requestEdit() {
    if (!tag) return;
    onRequestEdit(tag);
  }

  function requestDelete() {
    if (!tag) return;
    onRequestDelete(tag);
  }
</script>

{#if tag}
  <div class="tag-detail">
    <div class="panel-header">
      <h3>{tag.id}</h3>
      <span class="mode-label">表示モード</span>
    </div>

    <dl class="detail-list">
      <dt>ID</dt><dd class="mono">{tag.id}</dd>
      <dt>名前</dt><dd>{tag.name}</dd>
      <dt>データ型</dt><dd><span class="badge">{tag.data_type}</span></dd>
      <dt>ドライバ</dt><dd class="mono">{tag.driver_id}</dd>
      <dt>スキャングループ</dt><dd class="mono">{tag.scan_group_id}</dd>
      <dt>ドライバ仕様</dt>
      <dd><pre class="json">{JSON.stringify(tag.driver_spec, null, 2)}</pre></dd>
    </dl>

    <div class="actions">
      <button class="btn-primary" onclick={requestEdit}>編集</button>
      <button class="btn-danger" onclick={requestDelete}>削除</button>
      <button class="btn-outline" onclick={onRequestClose}>閉じる</button>
    </div>
  </div>
{:else}
  <div class="empty-state">
    <p>タグを選択してください</p>
    <p class="hint">新規追加は中央ペインの「＋ 新規タグ」から実行します。</p>
  </div>
{/if}

<style>
  .tag-detail {
    padding: 16px;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    border-bottom: 1px solid #e3e8ef;
    padding-bottom: 10px;
  }

  .mode-label {
    color: #64748b;
    font-size: 0.78rem;
    background: #f1f5f9;
    border-radius: 999px;
    padding: 0.2rem 0.6rem;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #2c3e50;
    word-break: break-all;
  }

  .detail-list {
    display: grid;
    grid-template-columns: 6em 1fr;
    gap: 6px 10px;
    margin: 0 0 16px 0;
    font-size: 0.85rem;
  }

  dt {
    color: #7f8c8d;
    font-weight: 600;
  }

  dd {
    margin: 0;
    word-break: break-all;
  }

  .mono {
    font-family: monospace;
    font-size: 0.85rem;
  }

  .json {
    background: #f6f8fb;
    border: 1px solid #e3e8ef;
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 0.78rem;
    white-space: pre-wrap;
    margin: 0;
    max-height: 120px;
    overflow-y: auto;
  }

  .badge {
    background: #e8f0fe;
    color: #1a56db;
    border-radius: 3px;
    padding: 1px 6px;
    font-size: 0.8rem;
    font-family: monospace;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 12px;
  }

  .btn-primary {
    background: #3498db;
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

  .btn-danger {
    background: #e74c3c;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-danger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-outline {
    background: #fff;
    color: #1e40af;
    border: 1px solid #bfdbfe;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #95a5a6;
    font-size: 0.85rem;
    text-align: center;
    padding: 20px;
  }

  .hint {
    margin-top: 0.4rem;
    font-size: 0.75rem;
    color: #a0aec0;
  }
</style>
