<script lang="ts">
  import '../app.css';
</script>

<main>
  <div class="layout">
    <slot />
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue',
      Arial, sans-serif;
    background-color: #f5f5f5;
    color: #333;
  }

  main {
    height: 100vh;
    width: 100%;
  }

  .layout {
    display: flex;
    height: 100%;
    width: 100%;
  }
</style>
