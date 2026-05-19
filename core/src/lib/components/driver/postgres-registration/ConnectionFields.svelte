<script lang="ts">
  let {
    driverIdInput,
    host,
    portInput,
    database,
    username,
    password,
    testing,
    loadingTables,
    testMessage,
    testError,
    tableError,
    onDriverIdInput,
    onHostInput,
    onPortInput,
    onDatabaseInput,
    onUsernameInput,
    onPasswordInput,
    onTestConnection,
    onLoadTables,
  }: {
    driverIdInput: string;
    host: string;
    portInput: string;
    database: string;
    username: string;
    password: string;
    testing: boolean;
    loadingTables: boolean;
    testMessage: string;
    testError: string;
    tableError: string;
    onDriverIdInput: (value: string) => void;
    onHostInput: (value: string) => void;
    onPortInput: (value: string) => void;
    onDatabaseInput: (value: string) => void;
    onUsernameInput: (value: string) => void;
    onPasswordInput: (value: string) => void;
    onTestConnection: () => void;
    onLoadTables: () => void;
  } = $props();
</script>

<div class="grid2">
  <label>
    接続先ID
    <input value={driverIdInput} placeholder="pg_main" oninput={(event) => onDriverIdInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
  <label>
    ホスト
    <input value={host} placeholder="127.0.0.1" oninput={(event) => onHostInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
</div>
<div class="grid3">
  <label>
    ポート
    <input type="number" min="1" max="65535" value={portInput} oninput={(event) => onPortInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
  <label>
    データベース
    <input value={database} placeholder="mydb" oninput={(event) => onDatabaseInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
  <label>
    ユーザー名
    <input value={username} placeholder="postgres" oninput={(event) => onUsernameInput((event.currentTarget as HTMLInputElement).value)} />
  </label>
</div>
<label>
  パスワード
  <input type="password" value={password} placeholder="(任意)" oninput={(event) => onPasswordInput((event.currentTarget as HTMLInputElement).value)} />
</label>

<div class="row">
  <button class="btn" onclick={onTestConnection} disabled={testing}>
    {testing ? '接続テスト中...' : 'テスト接続'}
  </button>
  <button class="btn secondary" onclick={onLoadTables} disabled={loadingTables}>
    {loadingTables ? 'テーブル取得中...' : 'テーブル再取得'}
  </button>
</div>
{#if testMessage}<p class="ok">{testMessage}</p>{/if}
{#if testError}<p class="error">{testError}</p>{/if}
{#if tableError}<p class="error">{tableError}</p>{/if}

<style>
  .row { display: flex; gap: 8px; margin-bottom: 8px; }
  .btn {
    border: none;
    border-radius: 6px;
    background: #2563eb;
    color: #fff;
    padding: 6px 10px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .btn.secondary { background: #475569; }
  .btn:disabled { opacity: 0.7; cursor: not-allowed; }
  .grid2 { display: grid; gap: 8px; grid-template-columns: 1fr 1fr; margin-bottom: 8px; }
  .grid3 { display: grid; gap: 8px; grid-template-columns: 1fr 1fr 1fr; margin-bottom: 8px; }
  label { display: grid; gap: 4px; font-size: 0.82rem; color: #334155; margin-bottom: 8px; }
  input {
    padding: 6px 8px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.82rem;
    background: #fff;
  }
  .ok { color: #166534; font-size: 0.8rem; margin: 4px 0; }
  .error { color: #b91c1c; font-size: 0.8rem; margin: 4px 0; }
</style>
