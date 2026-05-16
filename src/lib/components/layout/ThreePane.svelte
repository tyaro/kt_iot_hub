<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import NavigationPane from './three-pane/NavigationPane.svelte';
  import DashboardContent from './three-pane/DashboardContent.svelte';
  import TagsContent from './three-pane/TagsContent.svelte';
  import SettingsContent from './three-pane/SettingsContent.svelte';
  import TagRightPane from './three-pane/TagRightPane.svelte';
  import PlaceholderContent from './three-pane/PlaceholderContent.svelte';
  import DriverUiDialogs from './three-pane/DriverUiDialogs.svelte';
  import {
    confirmAction,
    extractErrorMessage,
    loadDriverUiBaseDirFromStorage,
    normalizeDriverUiBaseDir,
    notify,
    saveDriverUiBaseDirToStorage,
    wait,
  } from './three-pane/helpers';
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

  let driverUiBaseDirInput = $state('');
  let driverUiBaseDirSaved = $state<string | null>(null);

  function saveDriverUiBaseDir() {
    const normalized = normalizeDriverUiBaseDir(driverUiBaseDirInput);
    try {
      saveDriverUiBaseDirToStorage(normalized);
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
    const loaded = loadDriverUiBaseDirFromStorage();
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
  <NavigationPane pages={pages} currentPage={currentPage} onSelect={selectPage} />

  <!-- 中央ペイン: コンテンツ一覧 -->
  <div class="center-pane">
    {#if currentPage === 'dashboard'}
      <DashboardContent
        tagCount={$tagsStore.items.length}
        driverCount={$driversStore.items.length}
        enabledDriverCount={$driversStore.items.filter((d: DriverDto) => d.enabled).length}
        {runtimeStatus}
        {runtimeBusy}
        {dashboardMessage}
        onNavigateTags={() => selectPage('tags')}
        onStartServers={startServers}
        onStopServers={stopServers}
      />

    {:else if currentPage === 'tags'}
      <TagsContent
        {driverUiPolling}
        {tagActionMessage}
        selectedTagId={selectedTag?.id ?? null}
        selectedDriverId={selectedDriver?.id ?? null}
        selectedScanGroupId={selectedScanGroup?.id ?? null}
        onNewDriver={newDriver}
        onNewTag={newTag}
        onSelectTag={onTagSelect}
        onSelectDriver={onDriverSelect}
        onSelectScanGroup={onScanGroupSelect}
        onRequestNewTag={requestNewTagForDriver}
        onRequestDeleteDriver={requestDeleteDriver}
        onRequestEditTag={requestEditTag}
        onRequestDeleteTag={requestDeleteTag}
      />

    {:else if currentPage === 'publishers'}
      <PlaceholderContent
        title="パブリッシャ管理"
        message="パブリッシャ管理画面は今後実装予定です。"
      />

    {:else if currentPage === 'logs'}
      <PlaceholderContent
        title="ログ"
        message="ログビューワは今後実装予定です。"
      />

    {:else if currentPage === 'settings'}
      <SettingsContent
        {driverUiBaseDirInput}
        {driverUiBaseDirSaved}
        {settingsMessage}
        onDriverUiBaseDirInput={(value) => {
          driverUiBaseDirInput = value;
        }}
        onPickDriverUiBaseDir={pickDriverUiBaseDir}
        onSaveDriverUiBaseDir={saveDriverUiBaseDir}
        onClearDriverUiBaseDir={clearDriverUiBaseDir}
      />
    {/if}
  </div>

  <!-- 右ペイン: 詳細/編集 -->
  <TagRightPane
    {currentPage}
    {tagMode}
    {selectedTag}
    {selectedDriver}
    {selectedScanGroup}
    {editorDriverId}
    onTagEditorDone={onTagEditorDone}
    onTagEditorCancel={() => {
      closeTagEditor();
      tagActionMessage = '';
    }}
    onTagDetailEdit={requestEditTag}
    onTagDetailDelete={requestDeleteTag}
    onTagDetailClose={() => {
      selectedTag = null;
      editorDriverId = null;
      tagActionMessage = '';
    }}
    onDriverDelete={requestDeleteDriver}
    onDriverDone={async () => {
      await reloadDrivers();
      await reloadScanGroups();
      selectedDriver = null;
    }}
  />
</div>

<DriverUiDialogs
  {driverPickerOpen}
  drivers={$driversStore.items}
  driversLoading={$driversStore.loading}
  driversError={$driversStore.error}
  {driverTypePickerOpen}
  {driverTypeOptions}
  onReloadDrivers={reloadDrivers}
  onCloseDriverPicker={closeDriverPicker}
  onSelectDriver={onDriverPicked}
  onCloseDriverTypePicker={closeDriverTypePicker}
  onSelectDriverType={onDriverTypePicked}
/>

<style>
  .three-pane {
    display: flex;
    height: 100%;
    width: 100%;
    background-color: #ffffff;
  }

  /* ─ 中央ペイン ───────────────────────── */
  .center-pane {
    flex: 1;
    overflow-y: auto;
    background-color: #f4f6f9;
  }

</style>
