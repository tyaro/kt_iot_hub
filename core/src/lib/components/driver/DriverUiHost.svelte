<script lang="ts">
  import type { DriverUiLaunchContextDto } from '$lib/ipc/index';

  interface Props {
    context: DriverUiLaunchContextDto;
  }

  let { context }: Props = $props();
</script>

<div class="driver-ui-host">
  <div class="unsupported">
    <p>外部プロセス UI を起動してください。</p>
    <p>ドライバタイプ: {context.driverType ?? '(不明)'}</p>
    <p class="hint">この画面は外部登録UIの起動先としてのみ使われます。</p>
  </div>
</div>

<style>
  .driver-ui-host {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .unsupported {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #475569;
    gap: 8px;
  }

  .hint {
    font-size: 0.82rem;
    color: #94a3b8;
  }
</style>
