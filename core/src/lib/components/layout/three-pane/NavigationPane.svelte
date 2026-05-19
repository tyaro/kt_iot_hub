<script lang="ts">
  type Page = {
    id: string;
    label: string;
    icon: string;
  };

  let {
    grpcRunning,
    pages,
    currentPage,
    onSelect,
  }: {
    grpcRunning: boolean;
    pages: Page[];
    currentPage: string;
    onSelect: (pageId: string) => void;
  } = $props();
</script>

<div class="left-pane">
  <div class="header">
    <h1>IoT Hub</h1>
    <p class="version">v{__APP_VERSION__}</p>
  </div>
  <nav class="nav-menu">
    {#each pages as page (page.id)}
      <button
        class="nav-button"
        class:active={currentPage === page.id}
        onclick={() => onSelect(page.id)}
      >
        <span class="nav-icon">{page.icon}</span>
        <span class="nav-label">{page.label}</span>
      </button>
    {/each}
  </nav>

  <div class="status-footer">
    <div class="status-label">gRPC (IPC)</div>
    <div class="status-badge" class:running={grpcRunning}>
      {grpcRunning ? 'ON' : 'OFF'}
    </div>
  </div>
</div>

<style>
  .left-pane {
    width: 200px;
    min-width: 200px;
    background-color: #1e2d3d;
    color: #ecf0f1;
    border-right: 1px solid #253545;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .header {
    padding: 18px 16px 14px;
    border-bottom: 1px solid #253545;
    text-align: center;
  }

  .header h1 {
    margin: 0;
    font-size: 1.2rem;
    letter-spacing: 0.05em;
  }

  .version {
    margin: 4px 0 0;
    font-size: 0.7rem;
    color: #7f8c8d;
  }

  .nav-menu {
    flex: 1;
  }

  .status-footer {
    border-top: 1px solid #253545;
    padding: 12px 14px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    background: #182533;
  }

  .status-label {
    font-size: 0.72rem;
    color: #94a3b8;
    font-weight: 600;
  }

  .status-badge {
    font-size: 0.72rem;
    font-weight: 700;
    color: #fecaca;
    background: #7f1d1d;
    border-radius: 999px;
    padding: 2px 8px;
  }

  .status-badge.running {
    color: #dcfce7;
    background: #166534;
  }

  .nav-button {
    width: 100%;
    padding: 11px 16px;
    border: none;
    background: none;
    color: #b0bec5;
    cursor: pointer;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.875rem;
    transition: background-color 0.15s;
  }

  .nav-button:hover {
    background-color: #263545;
  }

  .nav-button.active {
    background-color: #2e86c1;
    color: #fff;
  }

  .nav-icon {
    font-size: 1.1rem;
    width: 1.4em;
    text-align: center;
  }
</style>
