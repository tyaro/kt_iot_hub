<script lang="ts">
  import ThreePane from './lib/components/layout/ThreePane.svelte';
</script>

<svelte:head>
  <title>kt_iot_hub</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</svelte:head>

<main class="app-shell">
  <ThreePane />
</main>

<style>
  :global(html, body, #app) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }

  .app-shell {
    width: 100%;
    height: 100%;
  }
</style>
