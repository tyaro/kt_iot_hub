<script lang="ts">
  let {
    title,
    message,
  }: {
    title: string;
    message: string;
  } = $props();
</script>

<div class="content">
  <h2>{title}</h2>
  <p class="placeholder">{message}</p>
</div>

<style>
  .content {
    padding: 24px 28px;
  }

  .content h2 {
    margin: 0 0 20px;
    font-size: 1.1rem;
    color: #2c3e50;
    border-bottom: 2px solid #2e86c1;
    padding-bottom: 8px;
  }

  .placeholder {
    color: #95a5a6;
    font-size: 0.9rem;
  }
</style>
