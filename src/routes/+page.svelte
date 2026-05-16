<script lang="ts">
  import { onMount } from 'svelte';
  import ThreePane from '$lib/components/layout/ThreePane.svelte';
  import DriverUiHost from '$lib/components/driver/DriverUiHost.svelte';
  import { getDriverUiLaunchContext, type DriverUiLaunchContextDto } from '$lib/ipc/index';

  let context = $state<DriverUiLaunchContextDto | null>(null);
  let ready = $state(false);

  onMount(async () => {
    try {
      context = await getDriverUiLaunchContext();
    } catch {
      context = null;
    }
    ready = true;
  });
</script>

{#if !ready}
  <!-- 起動モード判定中 -->
{:else if context?.launchedAsDriverUi}
  <DriverUiHost {context} />
{:else}
  <ThreePane />
{/if}

<style>
  :global(html, body) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }
</style>
