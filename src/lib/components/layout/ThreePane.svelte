<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import NavigationPane from './three-pane/NavigationPane.svelte';
  import CenterPaneContent from './three-pane/CenterPaneContent.svelte';
  import ThreePaneOverlays from './three-pane/ThreePaneOverlays.svelte';
  import {
    defaultRuntimeStatus,
    isPageId,
    knownDriverTypes,
    pages,
    type PageId,
  } from './three-pane/constants';
  import {
    buildDriverUiAvailabilityByType,
    buildDriverTypeOptions,
    type DriverTypeOption,
  } from './three-pane/driverUiFlow';
  import { createThreePaneControllers } from './three-pane/controllerFactory';
  import {
    DRIVER_UI_IMPORT_BUSY_MESSAGE,
    extractErrorMessage,
    loadDriverUiBaseDirFromStorage,
    normalizeDriverUiBaseDir,
    notify,
    saveDriverUiBaseDirToStorage,
  } from './three-pane/helpers';
  import {
    resetSelectionForPage,
    type SelectionState,
  } from './three-pane/handlers';
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
    getAppMetrics,
    getDriverMetrics,
    importDriverUiResult,
    launchDriverUi,
    openMqttMonitorWindow,
    startRuntimeServices,
    stopRuntimeServices,
    type DriverDto,
    type RuntimeStatusDto,
    type AppMetricsDto,
    type DriverMetricsDto,
    type ScanGroupDto,
    type TagDto,
  } from '$lib/ipc/index';

  type ScanCycleHealthSummary = {
    observedGroupCount: number;
    delayedGroupCount: number;
    avgDeltaRatio: number | null;
    worstGroupLabel: string | null;
    worstDeltaRatio: number | null;
  };

  type DashboardMetrics = AppMetricsDto & {
    webview_memory_used_bytes: number | null;
    webview_memory_total_bytes: number | null;
    webview_memory_limit_bytes: number | null;
  };

  let currentPage = $state<PageId>('dashboard');

  function ensureDriverUiNotBusy(): boolean {
    if (!driverUiPolling) {
      return true;
    }
    tagActionMessage = DRIVER_UI_IMPORT_BUSY_MESSAGE;
    return false;
  }

  function setTagActionMessage(message: string) {
    tagActionMessage = message;
  }

  async function reloadTagManagementData() {
    await reloadDrivers();
    await reloadScanGroups();
    await reloadTags();
  }

  async function handleDriverSaved(driverId?: string): Promise<DriverDto | null> {
    await reloadTagManagementData();

    if (!driverId) {
      selectedDriver = null;
      return null;
    }

    const refreshedDriver = $driversStore.items.find((item: DriverDto) => item.id === driverId) ?? null;
    selectedDriver = refreshedDriver;

    if (selectedTag?.driver_id !== driverId) {
      selectedTag = null;
    }
    if (selectedScanGroup?.driver_id !== driverId) {
      selectedScanGroup = null;
    }

    return refreshedDriver;
  }

  let selectedTag = $state<TagDto | null>(null);
  let selectedScanGroup = $state<ScanGroupDto | null>(null);
  let tagActionMessage = $state('');
  let deletingTag = $state(false);
  let driverUiPolling = $state(false);
  let driverPickerOpen = $state(false);
  let driverTypePickerOpen = $state(false);
  let tagMode = $state<'detail' | 'new' | 'edit'>('detail');
  let editorDriverId = $state<string | null>(null);
  let runtimeStatus = $state<RuntimeStatusDto>({ ...defaultRuntimeStatus });
  let appMetrics = $state<DashboardMetrics>({
    sampled_at: new Date().toISOString(),
    process_cpu_percent: null,
    process_memory_bytes: null,
    system_cpu_percent: null,
    system_memory_used_bytes: null,
    system_memory_total_bytes: null,
    network_rx_bytes_per_sec: null,
    network_tx_bytes_per_sec: null,
    webview_memory_used_bytes: null,
    webview_memory_total_bytes: null,
    webview_memory_limit_bytes: null,
  });
  let driverMetrics = $state<DriverMetricsDto[]>([]);
  let runtimeBusy = $state(false);
  let dashboardMessage = $state('');
  let settingsMessage = $state('');

  const loadedDriverUiBaseDir = loadDriverUiBaseDirFromStorage();
  let driverUiBaseDirInput = $state(loadedDriverUiBaseDir ?? '');
  let driverUiBaseDirSaved = $state<string | null>(loadedDriverUiBaseDir);

  let selectedDriver = $state<DriverDto | null>(null);
  let driverUiAvailableByType = $state<Record<string, boolean>>({});
  let confirmDialogOpen = $state(false);
  let confirmDialogTitle = $state('確認');
  let confirmDialogMessage = $state('');
  let confirmDialogConfirmLabel = $state('実行する');
  let confirmDialogCancelLabel = $state('キャンセル');
  let confirmDialogResolver: ((result: boolean) => void) | null = null;

  function closeConfirmDialog(result: boolean) {
    confirmDialogOpen = false;
    const resolve = confirmDialogResolver;
    confirmDialogResolver = null;
    resolve?.(result);
  }

  async function confirmAction(message: string): Promise<boolean> {
    if (confirmDialogResolver) {
      confirmDialogResolver(false);
    }

    confirmDialogTitle = '削除の確認';
    confirmDialogMessage = message;
    confirmDialogConfirmLabel = '削除する';
    confirmDialogCancelLabel = 'キャンセル';
    confirmDialogOpen = true;

    return await new Promise<boolean>((resolve) => {
      confirmDialogResolver = resolve;
    });
  }

  const {
    driverUiPollingController,
    runtimeController,
    selectionController,
    tagUiController,
    deletionController,
    driverUiSettingsController,
  } = createThreePaneControllers({
    common: {
      notify,
      extractErrorMessage,
      confirmAction,
    },
    data: {
      getDrivers: () => $driversStore.items,
      getScanGroups: () => $scanGroupsStore.items,
      reloadTags,
      reloadDrivers,
      reloadScanGroups,
      reloadTagManagementData,
    },
    selectionState: {
      getSelectionState: () => ({
        selectedTag,
        selectedDriver,
        selectedScanGroup,
        tagMode,
        tagActionMessage,
        editorDriverId,
      }),
      setSelectionState: (state: SelectionState) => {
        selectedTag = state.selectedTag;
        selectedDriver = state.selectedDriver;
        selectedScanGroup = state.selectedScanGroup;
        tagMode = state.tagMode;
        tagActionMessage = state.tagActionMessage;
        editorDriverId = state.editorDriverId;
      },
      setSelectedDriver: (driver) => {
        selectedDriver = driver;
      },
      setSelectedTag: (tag) => {
        selectedTag = tag;
      },
      clearSelectedTag: () => {
        selectedTag = null;
      },
      clearSelectedDriver: () => {
        selectedDriver = null;
      },
      clearSelectedScanGroup: () => {
        selectedScanGroup = null;
      },
      getSelectedTag: () => selectedTag,
      getSelectedDriver: () => selectedDriver,
      getSelectedScanGroup: () => selectedScanGroup,
    },
    runtime: {
      isRuntimeBusy: () => runtimeBusy,
      setRuntimeBusy: (busy) => {
        runtimeBusy = busy;
      },
      setRuntimeStatus: (status) => {
        runtimeStatus = status;
      },
      setDashboardMessage: (message) => {
        dashboardMessage = message;
      },
    },
    driverUi: {
      ensureDriverUiNotBusy,
      setTagActionMessage,
      setDriverUiPolling: (polling) => {
        driverUiPolling = polling;
      },
      getDriverUiPolling: () => driverUiPolling,
      getDriverUiBaseDirSaved: () => driverUiBaseDirSaved,
      getDriverUiAvailableByType: () => driverUiAvailableByType,
      checkDriverUiResultApi: checkDriverUiResult,
      importDriverUiResultApi: importDriverUiResult,
      launchDriverUiForDriverApi: launchDriverUi,
      launchDriverUiForTypeApi: launchDriverUi,
    },
    tagFlow: {
      getDriversCount: () => $driversStore.items.length,
      isDriversLoading: () => $driversStore.loading,
      setDriverPickerOpen: (open) => {
        driverPickerOpen = open;
      },
      setDriverTypePickerOpen: (open) => {
        driverTypePickerOpen = open;
      },
    },
    deletion: {
      getDeletingTag: () => deletingTag,
      setDeletingTag: (value) => {
        deletingTag = value;
      },
      deleteTagApi: deleteTag,
      deleteDriverApi: deleteDriver,
    },
    settings: {
      getDriverUiBaseDirInput: () => driverUiBaseDirInput,
      setDriverUiBaseDirInput: (value) => {
        driverUiBaseDirInput = value;
      },
      setDriverUiBaseDirSaved: (value) => {
        driverUiBaseDirSaved = value;
      },
      setSettingsMessage: (message) => {
        settingsMessage = message;
      },
      normalizeDriverUiBaseDir,
      saveDriverUiBaseDirToStorage,
      pickFolder: async (defaultPath) => {
        const selected = await open({
          directory: true,
          multiple: false,
          defaultPath: defaultPath ?? undefined,
        });
        return typeof selected === 'string' ? selected : null;
      },
    },
  });

  function selectPage(pageId: string) {
    if (!isPageId(pageId)) {
      return;
    }
    if (confirmDialogOpen) {
      closeConfirmDialog(false);
    }
    currentPage = pageId;
    driverUiPollingController.cancel();
    selectionController.update((state) => {
      resetSelectionForPage(state);
    });
  }

  $effect(() => {
    const baseDir = driverUiBaseDirSaved;
    void buildDriverUiAvailabilityByType(knownDriverTypes, checkDriverUiAvailable, baseDir).then(
      (map) => {
        driverUiAvailableByType = map;
      },
    );
  });

  const driverTypeOptions = $derived.by<DriverTypeOption[]>(() =>
    buildDriverTypeOptions($driversStore.items, knownDriverTypes, driverUiAvailableByType),
  );

  const scanCycleHealthSummary = $derived.by<ScanCycleHealthSummary>(() => {
    const observed = $scanGroupsStore.items.filter((group) => group.cycle_delta_ratio != null);
    if (observed.length === 0) {
      return {
        observedGroupCount: 0,
        delayedGroupCount: 0,
        avgDeltaRatio: null,
        worstGroupLabel: null,
        worstDeltaRatio: null,
      };
    }

    const delayed = observed.filter((group) => (group.cycle_delta_ratio ?? 0) > 0.30);
    let worst = observed[0];
    for (const item of observed) {
      if ((item.cycle_delta_ratio ?? 0) > (worst.cycle_delta_ratio ?? 0)) {
        worst = item;
      }
    }
    const avgDeltaRatio =
      observed.reduce((sum, group) => sum + (group.cycle_delta_ratio ?? 0), 0) / observed.length;

    return {
      observedGroupCount: observed.length,
      delayedGroupCount: delayed.length,
      avgDeltaRatio,
      worstGroupLabel: `${worst.driver_id} / ${worst.id}`,
      worstDeltaRatio: worst.cycle_delta_ratio ?? null,
    };
  });

  $effect(() => {
    if (currentPage === 'dashboard' || currentPage === 'tags') {
      void reloadTagManagementData();
    }
  });

  $effect(() => {
    let disposed = false;
    const refresh = async () => {
      if (disposed) {
        return;
      }
      await runtimeController.refreshStatus(getRuntimeStatus);
      try {
        const metrics = await getAppMetrics();
        const dMetrics = await getDriverMetrics();
        const perf = (globalThis.performance as unknown as { memory?: {
          usedJSHeapSize?: number;
          totalJSHeapSize?: number;
          jsHeapSizeLimit?: number;
        } }).memory;

        appMetrics = {
          ...metrics,
          webview_memory_used_bytes: perf?.usedJSHeapSize ?? null,
          webview_memory_total_bytes: perf?.totalJSHeapSize ?? null,
          webview_memory_limit_bytes: perf?.jsHeapSizeLimit ?? null,
        };
        driverMetrics = dMetrics;
      } catch {
        // ダッシュボード表示に影響しないよう、メトリクス取得失敗は握りつぶす
      }
    };

    void refresh();
    const timerId = window.setInterval(() => {
      void refresh();
    }, 2000);

    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  });
</script>

<div class="three-pane">
  <NavigationPane
    pages={pages}
    currentPage={currentPage}
    grpcRunning={runtimeStatus.grpc_running}
    onSelect={selectPage}
  />

  <CenterPaneContent
    currentPage={currentPage}
    tagCount={$tagsStore.items.length}
    driverCount={$driversStore.items.length}
    enabledDriverCount={$driversStore.items.filter((d: DriverDto) => d.enabled).length}
    {runtimeStatus}
    {appMetrics}
    {driverMetrics}
    {runtimeBusy}
    {dashboardMessage}
    {scanCycleHealthSummary}
    {driverUiPolling}
    {tagActionMessage}
    selectedTagId={selectedTag?.id ?? null}
    selectedDriverId={selectedDriver?.id ?? null}
    selectedScanGroupId={selectedScanGroup?.id ?? null}
    {driverUiBaseDirInput}
    {driverUiBaseDirSaved}
    {settingsMessage}
    onNavigateTags={() => selectPage('tags')}
    onOpenMqttMonitor={openMqttMonitorWindow}
    onStartServers={() => runtimeController.runAction(() => startRuntimeServices({ driver_ui_base_dir: driverUiBaseDirSaved }), 'ドライバ / MQTT を開始しました。', 'ドライバ / MQTT の開始に失敗しました')}
    onStopServers={() => runtimeController.runAction(stopRuntimeServices, 'ドライバ / MQTT を停止しました。', 'ドライバ / MQTT の停止に失敗しました')}
    onNewDriver={tagUiController.newDriver}
    onSelectTag={selectionController.onTagSelect}
    onSelectDriver={selectionController.onDriverSelect}
    onSelectScanGroup={selectionController.onScanGroupSelect}
    onRequestNewTag={tagUiController.requestNewTagForDriver}
    onRequestEditDriver={tagUiController.requestEditDriver}
    onRequestDeleteDriver={deletionController.requestDeleteDriver}
    onRequestEditTag={tagUiController.requestEditTag}
    onRequestDeleteTag={deletionController.requestDeleteTag}
    onDriverUiBaseDirInput={driverUiSettingsController.setInputValue}
    onPickDriverUiBaseDir={driverUiSettingsController.pick}
    onSaveDriverUiBaseDir={driverUiSettingsController.save}
    onClearDriverUiBaseDir={driverUiSettingsController.clear}
  />

  <ThreePaneOverlays
    {currentPage}
    {tagMode}
    {selectedTag}
    {selectedDriver}
    {selectedScanGroup}
    {editorDriverId}
    onTagEditorDone={selectionController.onTagEditorDone}
    onTagEditorCancel={selectionController.onTagEditorCancel}
    onTagDetailEdit={tagUiController.requestEditTag}
    onTagDetailDelete={deletionController.requestDeleteTag}
    onTagDetailClose={selectionController.onTagDetailClose}
    onDriverDelete={deletionController.requestDeleteDriver}
    onDriverDone={handleDriverSaved}
    {driverPickerOpen}
    drivers={$driversStore.items}
    driversLoading={$driversStore.loading}
    driversError={$driversStore.error}
    {driverTypePickerOpen}
    {driverTypeOptions}
    onReloadDrivers={reloadDrivers}
    onCloseDriverPicker={tagUiController.closeDriverPicker}
    onSelectDriver={tagUiController.onDriverPicked}
    onCloseDriverTypePicker={tagUiController.closeDriverTypePicker}
    onSelectDriverType={tagUiController.onDriverTypePicked}
    {confirmDialogOpen}
    confirmDialogTitle={confirmDialogTitle}
    confirmDialogMessage={confirmDialogMessage}
    confirmDialogConfirmLabel={confirmDialogConfirmLabel}
    confirmDialogCancelLabel={confirmDialogCancelLabel}
    onConfirmDialogConfirm={() => closeConfirmDialog(true)}
    onConfirmDialogCancel={() => closeConfirmDialog(false)}
  />
</div>

<style>
  .three-pane {
    display: flex;
    height: 100%;
    width: 100%;
    background-color: #ffffff;
  }
</style>
