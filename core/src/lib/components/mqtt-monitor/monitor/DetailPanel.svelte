<script lang="ts">
  import type { MqttMonitorTopicDetailDto } from '$lib/ipc';

  let {
    displayedDetail,
  }: {
    displayedDetail: MqttMonitorTopicDetailDto | null;
  } = $props();
</script>

<section class="detail-panel">
  <div class="panel-title-row">
    <div>
      <div class="panel-title">Inspect</div>
      <div class="panel-subtitle">選択した topic の最新メッセージ</div>
    </div>
  </div>
  {#if displayedDetail?.latestMessage}
    <div class="detail-grid">
      <div>
        <span class="detail-label">Topic</span>
        <div class="detail-value code">{displayedDetail.fullPath}</div>
      </div>
      <div>
        <span class="detail-label">Timestamp</span>
        <div class="detail-value">{displayedDetail.latestMessage.timestamp}</div>
      </div>
      <div>
        <span class="detail-label">QoS</span>
        <div class="detail-value">{displayedDetail.latestMessage.qos}</div>
      </div>
      <div>
        <span class="detail-label">Retain</span>
        <div class="detail-value">{displayedDetail.latestMessage.retain ? 'true' : 'false'}</div>
      </div>
      <div class="full-width">
        <span class="detail-label">Payload</span>
        <pre class="payload-box">{displayedDetail.latestMessage.payload}</pre>
      </div>
    </div>
  {:else}
    <p class="muted">左の topic ツリーから項目を選択してください。</p>
  {/if}
</section>

<style>
  .detail-panel {
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

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    padding: 14px;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .detail-label {
    display: block;
    margin-bottom: 4px;
    font-size: 0.76rem;
    color: #64748b;
  }

  .detail-value {
    font-size: 0.84rem;
    color: #111827;
    word-break: break-all;
  }

  .code,
  .payload-box {
    font-family: Consolas, monospace;
  }

  .payload-box {
    margin: 0;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 10px;
    white-space: pre-wrap;
    word-break: break-all;
    overflow: auto;
  }

  .muted {
    margin: 0;
    padding: 14px;
    color: #94a3b8;
    font-size: 0.82rem;
  }
</style>
