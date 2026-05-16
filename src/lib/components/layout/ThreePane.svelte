<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import TagTree from '../tag/TagTree.svelte';
  import TagDetailPanel from '../tag/TagDetailPanel.svelte';
  import TagEditorPanel from '../tag/TagEditorPanel.svelte';
  import DriverDetailPanel from '../driver/DriverDetailPanel.svelte';
  import DriverPickerDialog from '../driver/DriverPickerDialog.svelte';
  import DriverTypePickerDialog from '../driver/DriverTypePickerDialog.svelte';
  import {
    scanGroupsStore,
    tagsStore,
    driversStore,
    reloadScanGroups,
    reloadTags,
    reloadDrivers,
  } from '$lib/stores/index';
  import {
    checkDriverUiAvailable,
    checkDriverUiResult,
    deleteDriver,
    deleteTag,
    getRuntimeStatus,
    importDriverUiResult,
    launchDriverUi,
    startRuntimeServices,
    stopRuntimeServices,
    type DriverDto,
    type LaunchDriverUiResponse,
    type RuntimeStatusDto,
    type ScanGroupDto,
    type TagDto,
  } from '$lib/ipc/index';

  // ─── ナビゲーション ───────────────────────────────────
  let currentPage = $state('dashboard');

  const pages = [
    { id: 'dashboard', label: 'ダッシュボード', icon: '📊' },
    { id: 'tags',      label: 'タグ管理',         icon: '🏷️' },
    { id: 'publishers',label: 'パブリッシャ',       icon: '📤' },
    { id: 'logs',      label: 'ログ',              icon: '📋' },
    { id: 'settings',  label: '設定',              icon: '🔧' },
  ];

  const defaultRuntimeStatus: RuntimeStatusDto = {
    drivers_running: false,
    publishers_running: false,
    grpc_running: false,
    last_error: null,
  };

  function selectPage(pageId: string) {
    currentPage = pageId;
    cancelDriverUiPolling();
    selectedTag = null;
    selectedScanGroup = null;
    tagMode = 'detail';
    editorDriverId = null;
    selectedDriver = null;
  }

  // ─── タグ選択 ─────────────────────────────────────────
  let selectedTag = $state<TagDto | null>(null);
  let selectedScanGroup = $state<ScanGroupDto | null>(null);
  let tagActionMessage = $state('');
  let deletingTag = $state(false);
  let driverUiPolling = $state(false);
  let pollingToken = $state(0);
  let driverPickerOpen = $state(false);
  let driverTypePickerOpen = $state(false);
  let tagMode = $state<'detail' | 'new' | 'edit'>('detail');
  let editorDriverId = $state<string | null>(null);
  let runtimeStatus = $state<RuntimeStatusDto>({ ...defaultRuntimeStatus });
  let runtimeBusy = $state(false);
  let dashboardMessage = $state('');
  let settingsMessage = $state('');

  const DRIVER_UI_BASE_DIR_KEY = 'kt_iot_hub.driverUiBaseDir';
  let driverUiBaseDirInput = $state('');
  let driverUiBaseDirSaved = $state<string | null>(null);

  function normalizeDriverUiBaseDir(value: string): string | null {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function loadDriverUiBaseDir(): string | null {
    if (typeof globalThis.localStorage === 'undefined') {
      return null;
    }
    const raw = globalThis.localStorage.getItem(DRIVER_UI_BASE_DIR_KEY);
    return raw && raw.trim().length > 0 ? raw.trim() : null;
  }

  function saveDriverUiBaseDir() {
    const normalized = normalizeDriverUiBaseDir(driverUiBaseDirInput);
    try {
      if (typeof globalThis.localStorage !== 'undefined') {
        if (normalized) {
          globalThis.localStorage.setItem(DRIVER_UI_BASE_DIR_KEY, normalized);
        } else {
          globalThis.localStorage.removeItem(DRIVER_UI_BASE_DIR_KEY);
        }
      }
      driverUiBaseDirSaved = normalized;
      settingsMessage = normalized
        ? `ドライバUI設置ベースパスを保存しました: ${normalized}`
        : 'ドライバUI設置ベースパス設定をクリアしました。';
    } catch (error) {
      settingsMessage = extractErrorMessage(error, '設定保存に失敗しました');
      notify(settingsMessage);
    }
  }

  function clearDriverUiBaseDir() {
    driverUiBaseDirInput = '';
    saveDriverUiBaseDir();
  }

  async function pickDriverUiBaseDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: driverUiBaseDirSaved ?? undefined,
      });

      if (!selected) {
        return;
      }

      if (typeof selected === 'string') {
        driverUiBaseDirInput = selected;
        settingsMessage = `フォルダを選択しました: ${selected}`;
      }
    } catch (error) {
      settingsMessage = extractErrorMessage(error, 'フォルダ選択に失敗しました');
      notify(settingsMessage);
    }
  }

  {
    const loaded = loadDriverUiBaseDir();
    driverUiBaseDirSaved = loaded;
    driverUiBaseDirInput = loaded ?? '';
  }

  function onTagSelect(tag: TagDto | null) {
    selectedTag = tag;
    if (tag) {
      selectedDriver = $driversStore.items.find((item) => item.id === tag.driver_id) ?? null;
      selectedScanGroup = $scanGroupsStore.items.find((item) => item.id === tag.scan_group_id) ?? null;
    }
    tagMode = 'detail';
    tagActionMessage = '';
  }

  function onDriverSelect(driver: DriverDto | null) {
    selectedDriver = driver;
    selectedTag = null;
    selectedScanGroup = null;
    tagMode = 'detail';
    tagActionMessage = '';
  }

  function onScanGroupSelect(scanGroup: ScanGroupDto | null) {
    selectedScanGroup = scanGroup;
    selectedTag = null;
    if (scanGroup) {
      selectedDriver = $driversStore.items.find((item) => item.id === scanGroup.driver_id) ?? null;
    }
    tagMode = 'detail';
    tagActionMessage = '';
  }

  function notify(message: string) {
    if (typeof globalThis.alert === 'function') {
      globalThis.alert(message);
    }
  }

  function extractErrorMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message) {
      return error.message;
    }
    if (typeof error === 'string' && error.length > 0) {
      return error;
    }
    if (error && typeof error === 'object') {
      const record = error as Record<string, unknown>;
      const nested = record.error;
      if (typeof nested === 'string' && nested.length > 0) {
        return nested;
      }
      const message = record.message;
      if (typeof message === 'string' && message.length > 0) {
        return message;
      }
    }
    return fallback;
  }

  async function refreshRuntimeStatus() {
    try {
      runtimeStatus = await getRuntimeStatus();
    } catch (error) {
      dashboardMessage = error instanceof Error ? error.message : 'ランタイム状態の取得に失敗しました';
    }
  }

  async function startServers() {
    if (runtimeBusy) return;
    runtimeBusy = true;
    try {
      runtimeStatus = await startRuntimeServices();
      dashboardMessage = 'バックグラウンドサービスを起動しました。';
    } catch (error) {
      dashboardMessage = extractErrorMessage(error, 'サービス起動に失敗しました');
      notify(dashboardMessage);
    } finally {
      runtimeBusy = false;
    }
  }

  async function stopServers() {
    if (runtimeBusy) return;
    runtimeBusy = true;
    try {
      runtimeStatus = await stopRuntimeServices();
      dashboardMessage = 'バックグラウンドサービスを停止しました。';
    } catch (error) {
      dashboardMessage = extractErrorMessage(error, 'サービス停止に失敗しました');
      notify(dashboardMessage);
    } finally {
      runtimeBusy = false;
    }
  }

  async function wait(ms: number): Promise<void> {
    await new Promise((resolve) => {
      if (typeof globalThis.setTimeout === 'function') {
        globalThis.setTimeout(resolve, ms);
        return;
      }
      resolve(undefined);
    });
  }

  function cancelDriverUiPolling() {
    pollingToken += 1;
    driverUiPolling = false;
  }

  async function monitorAndImportDriverUiResult(result: LaunchDriverUiResponse) {
    const token = pollingToken + 1;
    pollingToken = token;
    driverUiPolling = true;

    const maxAttempts = 90;
    const intervalMs = 1000;

    for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
      if (pollingToken !== token) {
        return;
      }

      try {
        const check = await checkDriverUiResult({ output_json_path: result.output_json_path });
        if (check.ready) {
          const imported = await importDriverUiResult({
            session_id: result.session_id,
            driver_id: result.driver_id,
            output_json_path: result.output_json_path,
          });
          await reloadDrivers();
          await reloadScanGroups();
          await reloadTags();
          selectedDriver = $driversStore.items.find((item) => item.id === imported.driver_id) ?? null;
          selectedScanGroup = null;
          selectedTag = null;
          tagActionMessage = `取込完了: ${imported.imported_tag_count}件のタグ / ${imported.imported_scan_group_count}件のスキャングループを反映しました`;
          driverUiPolling = false;
          return;
        }

        tagActionMessage = `ドライバUIの完了待機中... (${attempt}/${maxAttempts})`;
      } catch (error) {
        const message = error instanceof Error ? error.message : 'ドライバUI結果の取込に失敗しました';
        tagActionMessage = message;
        driverUiPolling = false;
        notify(message);
        return;
      }

      await wait(intervalMs);
    }

    if (pollingToken === token) {
      driverUiPolling = false;
      tagActionMessage = 'ドライバUI完了待機がタイムアウトしました。完了後に再度編集操作を実行してください。';
    }
  }

  function confirmAction(message: string): boolean {
    if (typeof globalThis.confirm === 'function') {
      return globalThis.confirm(message);
    }
    return true;
  }

  async function requestOpenDriverUi(driverId: string, actionLabel: '新規' | '編集') {
    if (driverUiPolling) {
      tagActionMessage = '前回のドライバUI結果を取込中です。完了までお待ちください。';
      return;
    }

    try {
      const result = await launchDriverUi({
        driver_id: driverId,
        driver_ui_base_dir: driverUiBaseDirSaved,
      });
      tagActionMessage = `${actionLabel}用ドライバUIを起動しました (driver_id=${result.driver_id}, session_id=${result.session_id})`;
      void monitorAndImportDriverUiResult(result);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'ドライバUI起動に失敗しました';
      tagActionMessage = message;
      notify(message);
    }
  }

  function openManualTagEditor(driverId: string, mode: 'new' | 'edit', tag?: TagDto | null) {
    selectedTag = tag ?? null;
    editorDriverId = driverId;
    tagMode = mode;
    tagActionMessage = mode === 'new'
      ? '手動タグ登録モードです。'
      : '外部ドライバUI未設定のため手動編集モードに切り替えました。';
  }

  function canUseDriverUi(driverId: string): boolean {
    const driver = $driversStore.items.find((item) => item.id === driverId);
    if (!driver) return false;
    return (
      driver.registration_ui_available ||
      (driverUiAvailableByType[driver.driver_type] ?? false)
    );
  }

  function requestEditTag(tag: TagDto) {
    if (canUseDriverUi(tag.driver_id)) {
      selectedTag = tag;
      void requestOpenDriverUi(tag.driver_id, '編集');
      return;
    }
    openManualTagEditor(tag.driver_id, 'edit', tag);
  }

  async function requestDeleteTag(tag: TagDto) {
    if (deletingTag) return;
    if (!confirmAction(`タグ「${tag.id}」を削除しますか？`)) return;
    deletingTag = true;
    try {
      await deleteTag(tag.id);
      await reloadTags();
      if (selectedTag?.id === tag.id) {
        selectedTag = null;
      }
      tagActionMessage = `タグ「${tag.id}」を削除しました`;
    } catch (error) {
      const message = extractErrorMessage(error, 'タグ削除に失敗しました');
      tagActionMessage = message;
      notify(message);
    } finally {
      deletingTag = false;
    }
  }

  async function requestDeleteDriver(driverId: string) {
    if (driverUiPolling) {
      tagActionMessage = '前回のドライバUI結果を取込中です。完了までお待ちください。';
      return;
    }

    if (!confirmAction(`接続先「${driverId}」を削除しますか？\n配下のScanグループとタグも削除されます。`)) {
      return;
    }

    try {
      await deleteDriver(driverId);
      await reloadDrivers();
      await reloadScanGroups();
      await reloadTags();
      if (selectedDriver?.id === driverId) {
        selectedDriver = null;
      }
      if (selectedTag?.driver_id === driverId) {
        selectedTag = null;
      }
      if (selectedScanGroup?.driver_id === driverId) {
        selectedScanGroup = null;
      }
      tagActionMessage = `接続先「${driverId}」を削除しました。`;
    } catch (error) {
      const message = extractErrorMessage(error, '接続先削除に失敗しました');
      tagActionMessage = message;
      notify(message);
    }
  }

  function requestNewTagForDriver(driverId: string) {
    if (canUseDriverUi(driverId)) {
      void requestOpenDriverUi(driverId, '新規');
      return;
    }
    openManualTagEditor(driverId, 'new');
  }

  async function newTag() {
    if (driverUiPolling) {
      tagActionMessage = '前回のドライバUI結果を取込中です。完了までお待ちください。';
      return;
    }

    driverPickerOpen = true;
    if ($driversStore.items.length === 0 && !$driversStore.loading) {
      await reloadDrivers();
    }
  }

  async function newDriver() {
    if (driverUiPolling) {
      tagActionMessage = '前回のドライバUI結果を取込中です。完了までお待ちください。';
      return;
    }

    if ($driversStore.items.length === 0 && !$driversStore.loading) {
      await reloadDrivers();
    }
    driverTypePickerOpen = true;
  }

  function closeDriverPicker() {
    driverPickerOpen = false;
  }

  function closeDriverTypePicker() {
    driverTypePickerOpen = false;
  }

  function onDriverPicked(driverId: string) {
    driverPickerOpen = false;
    requestNewTagForDriver(driverId);
  }

  async function onDriverTypePicked(driverType: string) {
    driverTypePickerOpen = false;
    try {
      const result = await launchDriverUi({
        driver_type: driverType,
        driver_ui_base_dir: driverUiBaseDirSaved,
      });
      tagActionMessage = `${driverType} 用のドライバUIを起動しました。接続先・Scanグループ・タグを外部画面で登録してください。`;
      void monitorAndImportDriverUiResult(result);
    } catch (error) {
      const message = extractErrorMessage(error, 'ドライバUI起動に失敗しました');
      tagActionMessage = message;
      notify(message);
    }
  }

  async function onTagEditorDone() {
    await reloadTags();
    tagMode = 'detail';
    tagActionMessage = 'タグ定義を保存しました。';
  }

  function closeTagEditor() {
    tagMode = 'detail';
    if (!selectedTag) {
      editorDriverId = null;
    }
  }

  // ─── ドライバ選択 ─────────────────────────────────────
  let selectedDriver = $state<DriverDto | null>(null);

  // ドライバタイプごとの UI 利用可否（EXE配置チェック）
  let driverUiAvailableByType = $state<Record<string, boolean>>({});

  const knownDriverTypes = ['postgres'];

  async function reloadDriverUiAvailability(baseDir: string | null) {
    const results = await Promise.all(
      knownDriverTypes.map(async (t) => [t, await checkDriverUiAvailable(t, baseDir)] as const),
    );
    const map: Record<string, boolean> = {};
    for (const [t, ok] of results) map[t] = ok;
    driverUiAvailableByType = map;
  }

  $effect(() => {
    const baseDir = driverUiBaseDirSaved;
    void reloadDriverUiAvailability(baseDir);
  });

  const driverTypeOptions = $derived.by(() => {
    const byType = new Map<string, DriverDto[]>();
    $driversStore.items.forEach((driver) => {
      if (!byType.has(driver.driver_type)) {
        byType.set(driver.driver_type, []);
      }
      byType.get(driver.driver_type)!.push(driver);
    });

    return knownDriverTypes.map((driverType) => {
      const samples = byType.get(driverType) ?? [];
      const available =
        samples.some((item) => item.registration_ui_available) ||
        (driverUiAvailableByType[driverType] ?? false);
      return {
        driverType,
        label: driverType === 'postgres' ? 'PostgreSQL 接続先' : driverType,
        available,
        description: driverType === 'postgres'
          ? '接続先情報、テーブル由来の Scan グループ、タグを専用UIで一括登録します。'
          : '専用UIで接続先・Scanグループ・タグを登録します。',
      };
    });
  });

  // ─── ダッシュボード用サマリ ──────────────────────────
  $effect(() => {
    if (currentPage === 'dashboard') {
      reloadTags();
      reloadDrivers();
      reloadScanGroups();
      void refreshRuntimeStatus();
    }
  });

  $effect(() => {
    if (currentPage === 'tags') {
      reloadDrivers();
      reloadScanGroups();
      reloadTags();
    }
  });
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
          <span class="nav-icon">{page.icon}</span>
          <span class="nav-label">{page.label}</span>
        </button>
      {/each}
    </nav>
  </div>

  <!-- 中央ペイン: コンテンツ一覧 -->
  <div class="center-pane">
    {#if currentPage === 'dashboard'}
      <div class="content">
        <h2>ダッシュボード</h2>
        <div class="dashboard-grid">
          <div class="card">
            <span class="card-icon">🏷️</span>
            <h3>タグ</h3>
            <p class="value">{$tagsStore.items.length}</p>
            <p class="sub">登録済み</p>
          </div>
          <div class="card">
            <span class="card-icon">⚙️</span>
            <h3>ドライバ</h3>
            <p class="value">{$driversStore.items.length}</p>
            <p class="sub">登録済み</p>
          </div>
          <div class="card">
            <span class="card-icon">⚡</span>
            <h3>有効ドライバ</h3>
            <p class="value">{$driversStore.items.filter((d: DriverDto) => d.enabled).length}</p>
            <p class="sub">稼働中</p>
          </div>
          <div class="card runtime-card">
            <span class="card-icon">🧩</span>
            <h3>サービス状態</h3>
            <p class="runtime-badge" class:running={runtimeStatus.drivers_running || runtimeStatus.publishers_running}>
              {runtimeStatus.drivers_running || runtimeStatus.publishers_running ? '起動中' : '停止中'}
            </p>
            <ul class="runtime-list">
              <li>Drivers: {runtimeStatus.drivers_running ? 'ON' : 'OFF'}</li>
              <li>Publishers: {runtimeStatus.publishers_running ? 'ON' : 'OFF'}</li>
              <li>gRPC (IPC): {runtimeStatus.grpc_running ? 'ON' : 'OFF'}</li>
            </ul>
          </div>
        </div>
        {#if dashboardMessage}
          <p class="action-message">{dashboardMessage}</p>
        {/if}
        {#if runtimeStatus.last_error}
          <p class="error-message">{runtimeStatus.last_error}</p>
        {/if}
        <div class="quicklinks">
          <button class="btn-outline" onclick={() => selectPage('tags')}>タグを管理</button>
          <button class="btn-primary" onclick={startServers} disabled={runtimeBusy || (runtimeStatus.drivers_running || runtimeStatus.publishers_running)}>
            {runtimeBusy ? '実行中...' : 'サーバ起動'}
          </button>
          <button class="btn-outline danger" onclick={stopServers} disabled={runtimeBusy || (!runtimeStatus.drivers_running && !runtimeStatus.publishers_running)}>
            サーバ停止
          </button>
        </div>
      </div>

    {:else if currentPage === 'tags'}
      <div class="content">
        <div class="content-header">
          <h2>タグ管理</h2>
          <div class="header-actions">
            <button class="btn-outline" onclick={newDriver} disabled={driverUiPolling}>＋ 新規ドライバ</button>
            <button class="btn-primary" onclick={newTag} disabled={driverUiPolling}>＋ 新規タグ</button>
          </div>
        </div>
        {#if tagActionMessage}
          <p class="action-message">{tagActionMessage}</p>
        {/if}
        <TagTree
          onSelect={onTagSelect}
          onSelectDriver={onDriverSelect}
          onSelectScanGroup={onScanGroupSelect}
          selectedTagId={selectedTag?.id ?? null}
          selectedDriverId={selectedDriver?.id ?? null}
          selectedScanGroupId={selectedScanGroup?.id ?? null}
          onRequestNewTag={requestNewTagForDriver}
          onRequestDeleteDriver={requestDeleteDriver}
          onRequestEditTag={requestEditTag}
          onRequestDeleteTag={requestDeleteTag}
        />
      </div>

    {:else if currentPage === 'publishers'}
      <div class="content">
        <h2>パブリッシャ管理</h2>
        <p class="placeholder">パブリッシャ管理画面は今後実装予定です。</p>
      </div>

    {:else if currentPage === 'logs'}
      <div class="content">
        <h2>ログ</h2>
        <p class="placeholder">ログビューワは今後実装予定です。</p>
      </div>

    {:else if currentPage === 'settings'}
      <div class="content">
        <h2>設定</h2>
        <div class="settings-card">
          <h3>ドライバUI実行ファイル配置</h3>
          <p class="settings-help">
            例: <code>D:\develop\kt_iot_hub</code> または <code>D:\develop\kt_iot_hub\driver-ui</code>
          </p>
          <label>
            ドライバUI設置ベースパス
            <input
              bind:value={driverUiBaseDirInput}
              placeholder="未指定時は自動探索（driver-ui/&lt;type&gt;/registration-ui.exe）"
            />
          </label>
          <div class="settings-actions">
            <button class="btn-outline" onclick={pickDriverUiBaseDir}>フォルダ選択...</button>
            <button class="btn-primary" onclick={saveDriverUiBaseDir}>保存</button>
            <button class="btn-outline" onclick={clearDriverUiBaseDir}>クリア</button>
          </div>
          {#if driverUiBaseDirSaved}
            <p class="settings-current">現在値: <code>{driverUiBaseDirSaved}</code></p>
          {/if}
          {#if settingsMessage}
            <p class="action-message">{settingsMessage}</p>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <!-- 右ペイン: 詳細/編集 -->
  <div class="right-pane">
    {#if currentPage === 'tags'}
      {#if tagMode === 'new' || tagMode === 'edit'}
        <TagEditorPanel
          mode={tagMode}
          tag={selectedTag}
          driverId={editorDriverId}
          onDone={onTagEditorDone}
          onCancel={() => {
            closeTagEditor();
            tagActionMessage = '';
          }}
        />
      {:else if selectedTag}
        <TagDetailPanel
          tag={selectedTag}
          onRequestEdit={requestEditTag}
          onRequestDelete={requestDeleteTag}
          onRequestClose={() => {
            selectedTag = null;
            editorDriverId = null;
            tagActionMessage = '';
          }}
        />
      {:else if selectedDriver}
        <DriverDetailPanel
          driver={selectedDriver}
          mode="detail"
          onRequestDelete={requestDeleteDriver}
          onDone={async () => {
            await reloadDrivers();
            await reloadScanGroups();
            selectedDriver = null;
          }}
        />
      {:else if selectedScanGroup}
        <div class="info-panel">
          <div class="panel-header compact">
            <h3>{selectedScanGroup.id}</h3>
            <span class="mode-label">ScanGroup</span>
          </div>
          <dl class="detail-list compact">
            <dt>ドライバ</dt><dd class="mono">{selectedScanGroup.driver_id}</dd>
            <dt>周期</dt><dd>{selectedScanGroup.scan_rate_ms ?? '-'} ms</dd>
            <dt>テーブル</dt><dd class="mono">{selectedScanGroup.table ?? '-'}</dd>
            <dt>時系列列</dt><dd class="mono">{selectedScanGroup.timestamp_column ?? '-'}</dd>
          </dl>
          <p class="helper-text">Scanグループの追加・変更は接続先ドライバ専用UI側で行います。</p>
        </div>
      {:else}
        <div class="empty-right">
          <p>ツリーから接続先 / Scanグループ / タグを選択してください</p>
        </div>
      {/if}
    {:else}
      <div class="empty-right">
        <p>左の一覧から<br />項目を選択してください</p>
      </div>
    {/if}
  </div>
</div>

<DriverPickerDialog
  open={driverPickerOpen}
  drivers={$driversStore.items}
  loading={$driversStore.loading}
  error={$driversStore.error}
  onReload={reloadDrivers}
  onClose={closeDriverPicker}
  onSelect={onDriverPicked}
/>

<DriverTypePickerDialog
  open={driverTypePickerOpen}
  options={driverTypeOptions}
  onClose={closeDriverTypePicker}
  onSelect={onDriverTypePicked}
/>

<style>
  .three-pane {
    display: flex;
    height: 100%;
    width: 100%;
    background-color: #ffffff;
  }

  /* ─ 左ペイン ─────────────────────────── */
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

  /* ─ 中央ペイン ───────────────────────── */
  .center-pane {
    flex: 1;
    overflow-y: auto;
    background-color: #f4f6f9;
  }

  .content {
    padding: 24px 28px;
  }

  .content-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .content h2 {
    margin: 0 0 20px;
    font-size: 1.1rem;
    color: #2c3e50;
    border-bottom: 2px solid #2e86c1;
    padding-bottom: 8px;
  }

  .content-header h2 {
    margin-bottom: 0;
    border-bottom: none;
    padding-bottom: 0;
  }

  /* ─ ダッシュボード ───────────────────── */
  .dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }

  .card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 20px 16px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  .card-icon {
    font-size: 1.8rem;
  }

  .card h3 {
    margin: 8px 0 4px;
    font-size: 0.8rem;
    color: #7f8c8d;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .value {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    color: #2e86c1;
  }

  .sub {
    margin: 2px 0 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }

  .quicklinks {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .placeholder {
    color: #95a5a6;
    font-size: 0.9rem;
  }

  .settings-card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 16px;
    max-width: 760px;
  }

  .settings-card h3 {
    margin: 0 0 10px;
    font-size: 0.95rem;
    color: #1f2937;
  }

  .settings-help {
    margin: 0 0 10px;
    font-size: 0.82rem;
    color: #64748b;
  }

  .settings-card label {
    display: grid;
    gap: 6px;
    font-size: 0.82rem;
    color: #334155;
    margin-bottom: 10px;
  }

  .settings-card input {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font-size: 0.84rem;
    box-sizing: border-box;
  }

  .settings-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }

  .settings-current {
    margin: 0 0 8px;
    font-size: 0.8rem;
    color: #334155;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .action-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #2563eb;
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .error-message {
    margin: 0 0 12px;
    font-size: 0.82rem;
    color: #b91c1c;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .runtime-card {
    text-align: left;
  }

  .runtime-badge {
    display: inline-block;
    margin: 0 0 8px;
    font-size: 0.8rem;
    font-weight: 700;
    color: #92400e;
    background: #fef3c7;
    border-radius: 999px;
    padding: 3px 10px;
  }

  .runtime-badge.running {
    color: #166534;
    background: #dcfce7;
  }

  .runtime-list {
    margin: 0;
    padding-left: 18px;
    color: #475569;
    font-size: 0.8rem;
  }

  /* ─ ボタン ───────────────────────────── */
  .btn-primary {
    background-color: #2e86c1;
    color: #fff;
    border: none;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .btn-primary:hover {
    background-color: #2471a3;
  }

  .btn-outline {
    background: #fff;
    color: #2e86c1;
    border: 1px solid #2e86c1;
    padding: 7px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-outline:hover {
    background: #ebf5fb;
  }

  .btn-outline.danger {
    color: #b91c1c;
    border-color: #fca5a5;
  }

  .btn-outline.danger:hover {
    background: #fef2f2;
  }

  /* ─ 右ペイン ─────────────────────────── */
  .right-pane {
    width: 280px;
    min-width: 240px;
    border-left: 1px solid #dbe2ea;
    background-color: #fff;
    overflow-y: auto;
  }

  .empty-right {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #b0bec5;
    font-size: 0.85rem;
    text-align: center;
    padding: 20px;
  }

  .info-panel {
    padding: 16px;
  }

  .panel-header.compact {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    border-bottom: 1px solid #e3e8ef;
    padding-bottom: 10px;
  }

  .panel-header.compact h3 {
    margin: 0;
    font-size: 1rem;
    color: #2c3e50;
    word-break: break-all;
  }

  .mode-label {
    color: #64748b;
    font-size: 0.78rem;
    background: #f1f5f9;
    border-radius: 999px;
    padding: 0.2rem 0.6rem;
  }

  .detail-list.compact {
    display: grid;
    grid-template-columns: 6em 1fr;
    gap: 6px 10px;
    margin: 0;
    font-size: 0.85rem;
  }

  .detail-list.compact dt {
    color: #7f8c8d;
    font-weight: 600;
  }

  .detail-list.compact dd {
    margin: 0;
    word-break: break-all;
  }

  .helper-text {
    margin-top: 12px;
    font-size: 0.8rem;
    color: #64748b;
    line-height: 1.5;
  }

  .mono {
    font-family: monospace;
  }
</style>
