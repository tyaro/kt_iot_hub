<script lang="ts">
  interface DriverTypeOption {
    driverType: string;
    label: string;
    available: boolean;
    description?: string;
    statusMessage?: string;
  }

  interface Props {
    open?: boolean;
    options?: DriverTypeOption[];
    onSelect?: (driverType: string) => void;
    onClose?: () => void;
  }

  let {
    open = false,
    options = [],
    onSelect = () => {},
    onClose = () => {},
  }: Props = $props();
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onClose}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="ドライバ種別選択"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          onClose();
        }
      }}
    >
      <div class="header">
        <h3>新規接続先のドライバ種別を選択</h3>
        <button type="button" class="btn-close" onclick={onClose} aria-label="閉じる">✕</button>
      </div>

      <p class="description">
        選択したドライバ専用UIを別ウィンドウで起動します。接続先・Scanグループ・タグはその画面でまとめて登録します。
      </p>

      <div class="list">
        {#each options as option (option.driverType)}
          <button
            type="button"
            class="driver-type-item"
            disabled={!option.available}
            onclick={() => onSelect(option.driverType)}
          >
            <span class="main-row">
              <strong>{option.label}</strong>
              <span class="type-chip">{option.driverType}</span>
            </span>
            {#if option.description}
              <span class="sub-row">{option.description}</span>
            {/if}
            {#if option.statusMessage}
              <span class="status-message">{option.statusMessage}</span>
            {/if}
            {#if !option.available}
              <span class="warning">起動パス未設定のため選択できません</span>
            {/if}
          </button>
        {/each}
      </div>

      <div class="actions">
        <button type="button" class="btn-cancel" onclick={onClose}>キャンセル</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1400;
    padding: 16px;
  }

  .dialog {
    width: min(560px, 100%);
    max-height: 80vh;
    overflow: auto;
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.22);
    padding: 16px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .header h3 {
    margin: 0;
    font-size: 1rem;
    color: #1e293b;
  }

  .btn-close {
    border: none;
    background: transparent;
    font-size: 1rem;
    color: #64748b;
    cursor: pointer;
  }

  .description {
    margin: 0 0 12px;
    font-size: 0.85rem;
    color: #475569;
    line-height: 1.5;
  }

  .list {
    display: grid;
    gap: 8px;
  }

  .driver-type-item {
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    background: #fff;
    padding: 12px;
    text-align: left;
    cursor: pointer;
    display: grid;
    gap: 6px;
  }

  .driver-type-item:hover:not(:disabled) {
    background: #eff6ff;
    border-color: #93c5fd;
  }

  .driver-type-item:disabled {
    cursor: not-allowed;
    opacity: 0.7;
    background: #f8fafc;
  }

  .main-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .type-chip {
    font-family: monospace;
    font-size: 0.75rem;
    background: #fef3c7;
    color: #92400e;
    border-radius: 999px;
    padding: 1px 8px;
  }

  .sub-row {
    color: #64748b;
    font-size: 0.8rem;
  }

  .warning {
    color: #b91c1c;
    font-size: 0.8rem;
  }

  .status-message {
    color: #92400e;
    font-size: 0.78rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 12px;
  }

  .btn-cancel {
    border: none;
    border-radius: 6px;
    padding: 7px 12px;
    font-size: 0.82rem;
    cursor: pointer;
    background: #e2e8f0;
    color: #334155;
  }
</style>
