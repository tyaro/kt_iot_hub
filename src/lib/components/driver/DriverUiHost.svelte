<script lang="ts">
  import PostgresRegistrationPanel from './PostgresRegistrationPanel.svelte';
  import type { DriverUiLaunchContextDto } from '$lib/ipc/index';

  interface Props {
    context: DriverUiLaunchContextDto;
  }

  let { context }: Props = $props();
</script>

<div class="driver-ui-host">
  {#if context.driverType === 'postgres'}
    <PostgresRegistrationPanel {context} />
  {:else}
    <div class="unsupported">
      <p>未対応のドライバタイプ: {context.driverType ?? '(不明)'}</p>
      <p class="hint">対応パネルが実装されると、ここに表示されます。</p>
    </div>
  {/if}
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
