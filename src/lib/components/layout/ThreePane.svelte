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
    computeScanCycleHealthSummary,
    createInitialDashboardMetrics,
    refreshDashboardMetrics,
    type DashboardMetrics,
    type ScanCycleHealthSummary,
  } from './three-pane/orchestrators/dashboard';
  import {
    ensureDriverUiNotBusy as ensureDriverUiNotBusyOrchestrated,
    syncSelectionAfterDriverSaved,
  } from './three-pane/orchestrators/tagManagement';
  import { buildThreePaneControllerDeps } from './three-pane/orchestrators/controllerDeps';
  import {
    scanGroupsStore,
    tagsStore,
    driversStore,
    reloadScanGroups,
    reloadTags,
    reloadDrivers,
    reloadAllRegistry,
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
    type DriverMetricsDto,
    type ScanGroupDto,
    type TagDto,
  } from '$lib/ipc/index';

  let currentPage = $state<PageId>('dashboard');

  function ensureDriverUiNotBusy(): boolean {
    return ensureDriverUiNotBusyOrchestrated(driverUiPolling, DRIVER_UI_IMPORT_BUSY_MESSAGE, (msg) => {
      tagActionMessage = msg;
    });
  }

  function setTagActionMessage(message: string) {
    tagActionMessage = message;
  }

  async function reloadTagManagementData() {
    await reloadAllRegistry();
  }

  async function handleDriverSaved(driverId?: string): Promise<DriverDto | null> {
    await reloadTagManagementData();
    const next = syncSelectionAfterDriverSaved(
      $driversStore.items,
      driverId,
      selectedTag,
      selectedScanGroup,
    );
    selectedDriver = next.selectedDriver;
    selectedTag = next.selectedTag;
    selectedScanGroup = next.selectedScanGroup;
    return next.selectedDriver;
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
  let appMetrics = $state<DashboardMetrics>(createInitialDashboardMetrics());
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
  } = createThreePaneControllers(
    buildThreePaneControllerDeps({
      notify,
      extractErrorMessage,
      confirmAction,
      getDrivers: () => $driversStore.items,
      getScanGroups: () => $scanGroupsStore.items,
      reloadTags,
      reloadDrivers,
      reloadScanGroups,
      reloadAllRegistry,
      reloadTagManagementData,
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
      getDriversCount: () => $driversStore.items.length,
      isDriversLoading: () => $driversStore.loading,
      setDriverPickerOpen: (open) => {
        driverPickerOpen = open;
      },
      setDriverTypePickerOpen: (open) => {
        driverTypePickerOpen = open;
      },
      getDeletingTag: () => deletingTag,
      setDeletingTag: (value) => {
        deletingTag = value;
      },
      deleteTagApi: deleteTag,
      deleteDriverApi: deleteDriver,
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
    }),
  );

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

  const scanCycleHealthSummary = $derived.by<ScanCycleHealthSummary>(() =>
    computeScanCycleHealthSummary($scanGroupsStore.items),
  );

  function handleNavigateTags() {
    selectPage('tags');
  }

  function handleStartServers() {
    return runtimeController.runAction(
      () => startRuntimeServices({ driver_ui_base_dir: driverUiBaseDirSaved }),
      'ドライバ / MQTT を開始しました。',
      'ドライバ / MQTT の開始に失敗しました',
    );
  }

  function handleStopServers() {
    return runtimeController.runAction(
      stopRuntimeServices,
      'ドライバ / MQTT を停止しました。',
      'ドライバ / MQTT の停止に失敗しました',
    );
  }

  function handleConfirmDialogConfirm() {
    closeConfirmDialog(true);
  }

  function handleConfirmDialogCancel() {
    closeConfirmDialog(false);
  }

  const navigationPaneProps = $derived.by(() => ({
    pages,
    currentPage,
    grpcRunning: runtimeStatus.grpc_running,
    onSelect: selectPage,
  }));

  const centerPaneProps = $derived.by(() => ({
    currentPage,
    tagCount: $tagsStore.items.length,
    driverCount: $driversStore.items.length,
    enabledDriverCount: $driversStore.items.filter((d: DriverDto) => d.enabled).length,
    runtimeStatus,
    appMetrics,
    driverMetrics,
    runtimeBusy,
    dashboardMessage,
    scanCycleHealthSummary,
    driverUiPolling,
    tagActionMessage,
    selectedTagId: selectedTag?.id ?? null,
    selectedDriverId: selectedDriver?.id ?? null,
    selectedScanGroupId: selectedScanGroup?.id ?? null,
    driverUiBaseDirInput,
    driverUiBaseDirSaved,
    settingsMessage,
    onNavigateTags: handleNavigateTags,
    onOpenMqttMonitor: openMqttMonitorWindow,
    onStartServers: handleStartServers,
    onStopServers: handleStopServers,
    onNewDriver: tagUiController.newDriver,
    onSelectTag: selectionController.onTagSelect,
    onSelectDriver: selectionController.onDriverSelect,
    onSelectScanGroup: selectionController.onScanGroupSelect,
    onRequestNewTag: tagUiController.requestNewTagForDriver,
    onRequestEditDriver: tagUiController.requestEditDriver,
    onRequestDeleteDriver: deletionController.requestDeleteDriver,
    onRequestEditTag: tagUiController.requestEditTag,
    onRequestDeleteTag: deletionController.requestDeleteTag,
    onDriverUiBaseDirInput: driverUiSettingsController.setInputValue,
    onPickDriverUiBaseDir: driverUiSettingsController.pick,
    onSaveDriverUiBaseDir: driverUiSettingsController.save,
    onClearDriverUiBaseDir: driverUiSettingsController.clear,
  }));

  const threePaneOverlayProps = $derived.by(() => ({
    currentPage,
    tagMode,
    selectedTag,
    selectedDriver,
    selectedScanGroup,
    editorDriverId,
    onTagEditorDone: selectionController.onTagEditorDone,
    onTagEditorCancel: selectionController.onTagEditorCancel,
    onTagDetailEdit: tagUiController.requestEditTag,
    onTagDetailDelete: deletionController.requestDeleteTag,
    onTagDetailClose: selectionController.onTagDetailClose,
    onDriverDelete: deletionController.requestDeleteDriver,
    onDriverDone: handleDriverSaved,
    driverPickerOpen,
    drivers: $driversStore.items,
    driversLoading: $driversStore.loading,
    driversError: $driversStore.error,
    driverTypePickerOpen,
    driverTypeOptions,
    onReloadDrivers: reloadDrivers,
    onCloseDriverPicker: tagUiController.closeDriverPicker,
    onSelectDriver: tagUiController.onDriverPicked,
    onCloseDriverTypePicker: tagUiController.closeDriverTypePicker,
    onSelectDriverType: tagUiController.onDriverTypePicked,
    confirmDialogOpen,
    confirmDialogTitle,
    confirmDialogMessage,
    confirmDialogConfirmLabel,
    confirmDialogCancelLabel,
    onConfirmDialogConfirm: handleConfirmDialogConfirm,
    onConfirmDialogCancel: handleConfirmDialogCancel,
  }));

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
      await refreshDashboardMetrics({
        currentPage,
        reloadScanGroups,
        refreshRuntimeStatus: runtimeController.refreshStatus,
        getRuntimeStatus,
        getAppMetrics,
        getDriverMetrics,
        setAppMetrics: (metrics) => {
          appMetrics = metrics;
        },
        setDriverMetrics: (metrics) => {
          driverMetrics = metrics;
        },
      });
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
  <NavigationPane {...navigationPaneProps} />

  <CenterPaneContent {...centerPaneProps} />

  <ThreePaneOverlays {...threePaneOverlayProps} />
</div>

<style>
  .three-pane {
    display: flex;
    height: 100%;
    width: 100%;
    background-color: #ffffff;
  }
</style>
