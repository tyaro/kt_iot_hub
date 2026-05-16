<script lang="ts">
  import DriverSettingsForm from '../driver/DriverSettingsForm.svelte';
  import TagTable from '../tag/TagTable.svelte';

  // ページ状態
  let currentPage = $state('dashboard');
  let selectedItem = $state<string | null>(null);

  const pages = [
    { id: 'dashboard', label: 'ダッシュボード', icon: '📊' },
    { id: 'tags', label: 'タグ', icon: '🏷️' },
    { id: 'drivers', label: 'ドライバ', icon: '⚙️' },
    { id: 'publishers', label: 'パブリッシャ', icon: '📤' },
    { id: 'logs', label: 'ログ', icon: '📋' },
    { id: 'settings', label: '設定', icon: '⚡' },
  ];

  function selectPage(pageId: string) {
    currentPage = pageId;
    selectedItem = null;
  }
</script>

<div class="three-pane">
  <!-- 左ペイン: ナビゲーション -->
  <div class="left-pane">
    <div class="header">
      <h1>IoT Hub</h1>
      <p class="version">v0.1.0</p>
    </div>
    <nav class="nav-menu">
      {#each pages as page (page.id)}
        <button
          class="nav-button"
          class:active={currentPage === page.id}
          onclick={() => selectPage(page.id)}
        >
          <span class="icon">{page.icon}</span>
          <span class="label">{page.label}</span>
        </button>
      {/each}
    </nav>
  </div>

  <!-- 中央ペイン: コンテンツ -->
  <div class="center-pane">
    {#if currentPage === 'dashboard'}
      <div class="content">
        <h2>ダッシュボード</h2>
        <p>システムステータスがここに表示されます。</p>
        <div class="dashboard-grid">
          <div class="card">
            <h3>タグ</h3>
            <p class="value">0</p>
          </div>
          <div class="card">
            <h3>ドライバ</h3>
            <p class="value">0</p>
          </div>
          <div class="card">
            <h3>パブリッシャ</h3>
            <p class="value">0</p>
          </div>
        </div>
      </div>
    {:else if currentPage === 'tags'}
      <div class="content">
        <h2>タグ管理</h2>
        <TagTable />
      </div>
    {:else if currentPage === 'drivers'}
      <div class="content">
        <h2>ドライバ管理</h2>
        <DriverSettingsForm />
      </div>
    {:else if currentPage === 'publishers'}
      <div class="content">
        <h2>パブリッシャ管理</h2>
        <button class="btn-primary">新規パブリッシャを追加</button>
        <p>パブリッシャ一覧がここに表示されます。</p>
      </div>
    {:else if currentPage === 'logs'}
      <div class="content">
        <h2>ログ</h2>
        <p>システムログがここに表示されます。</p>
      </div>
    {:else if currentPage === 'settings'}
      <div class="content">
        <h2>設定</h2>
        <p>アプリケーション設定がここに表示されます。</p>
      </div>
    {/if}
  </div>

  <!-- 右ペイン: 詳細 -->
  <div class="right-pane">
    {#if selectedItem}
      <div class="detail-panel">
        <h3>詳細情報</h3>
        <p>{selectedItem}</p>
      </div>
    {:else}
      <div class="detail-panel empty">
        <p>項目を選択してください</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .three-pane {
    display: flex;
    height: 100%;
    width: 100%;
    background-color: #ffffff;
  }

  .left-pane {
    width: 200px;
    background-color: #2c3e50;
    color: #ecf0f1;
    border-right: 1px solid #34495e;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .header {
    padding: 20px;
    border-bottom: 1px solid #34495e;
    text-align: center;
  }

  .header h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  .version {
    margin: 5px 0 0 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }

  .nav-menu {
    flex: 1;
    padding: 0;
    margin: 0;
  }

  .nav-button {
    width: 100%;
    padding: 12px 15px;
    margin: 0;
    border: none;
    background: none;
    color: #bdc3c7;
    cursor: pointer;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.9rem;
    transition: background-color 0.2s;
  }

  .nav-button:hover {
    background-color: #34495e;
  }

  .nav-button.active {
    background-color: #3498db;
    color: #fff;
  }

  .icon {
    font-size: 1.2rem;
  }

  .center-pane {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }

  .content {
    padding: 30px;
  }

  .content h2 {
    margin-top: 0;
    color: #2c3e50;
    border-bottom: 2px solid #3498db;
    padding-bottom: 10px;
  }

  .dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 20px;
    margin-top: 20px;
  }

  .card {
    background-color: #ecf0f1;
    padding: 20px;
    border-radius: 5px;
    text-align: center;
  }

  .card h3 {
    margin: 0 0 10px 0;
    color: #2c3e50;
  }

  .value {
    margin: 0;
    font-size: 2rem;
    font-weight: bold;
    color: #3498db;
  }

  .btn-primary {
    background-color: #3498db;
    color: #fff;
    border: none;
    padding: 10px 20px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    margin-bottom: 20px;
    transition: background-color 0.2s;
  }

  .btn-primary:hover {
    background-color: #2980b9;
  }

  .right-pane {
    width: 250px;
    border-left: 1px solid #ecf0f1;
    background-color: #f8f9fa;
    overflow-y: auto;
  }

  .detail-panel {
    padding: 20px;
  }

  .detail-panel h3 {
    margin-top: 0;
    color: #2c3e50;
  }

  .detail-panel.empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #95a5a6;
  }

  .detail-panel.empty p {
    text-align: center;
  }
</style>
