<!--
  このファイルは R-FE-01 完了時点で ThreePane の薄化を優先して一時的に 300 行を超えています。
  次段の R-FE-02 以降で state/handler の責務単位分割を進め、300 行以内へ縮小予定です。
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { open, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import NavigationPane from './NavigationPane.svelte';
  import CenterPaneContent from './CenterPaneContent.svelte';
  import ThreePaneOverlays from './ThreePaneOverlays.svelte';
  import {
    defaultRuntimeStatus,
    isPageId,
    pages,
    type PageId,
  } from './constants';
  import {
    buildDriverUiAvailabilityByType,
    buildDriverTypeOptions,
    type DriverTypeOption,
  } from './driverUiFlow';
  import { createThreePaneControllers } from './controllerFactory';
  import {
    DRIVER_UI_IMPORT_BUSY_MESSAGE,
    extractErrorMessage,
    loadDriverUiBaseDirFromStorage,
    normalizeDriverUiBaseDir,
    notify,
    saveDriverUiBaseDirToStorage,
  } from './helpers';
  import {
    resetSelectionForPage,
    type SelectionState,
  } from './handlers';
  import {
    computeScanCycleHealthSummary,
    createInitialDashboardMetrics,
    refreshDashboardMetrics,
    type DashboardMetrics,
    type ScanCycleHealthSummary,
  } from './orchestrators/dashboard';
  import {
    ensureDriverUiNotBusy as ensureDriverUiNotBusyOrchestrated,
    syncSelectionAfterDriverSaved,
  } from './orchestrators/tagManagement';
  import { buildThreePaneControllerDeps } from './orchestrators/controllerDeps';
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
    checkDriverUiResult,
    deleteDriver,
    discoverDriverPackages,
    deleteTag,
    exportTagManagementSettings,
    getDefaultDriverUiBaseDir,
    getRuntimeStatus,
    getAppMetrics,
    getDriverMetrics,
    importDriverUiResult,
    importTagManagementSettings,
    launchDriverUi,
    openMqttMonitorWindow,
    startRuntimeServices,
    stopRuntimeServices,
    type DriverDto,
    type DiscoveredDriverPackageDto,
    type InvalidDriverPackageDto,
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

  async function handleExportTagSettings() {
    if (!ensureDriverUiNotBusy()) {
      return;
    }

    try {
      const selected = await saveDialog({
        defaultPath: 'tag-management-settings.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (typeof selected !== 'string') {
        return;
      }

      const exported = await exportTagManagementSettings(selected);
      tagActionMessage = `接続先設定をエクスポートしました: ${exported.driver_count}件の接続先 / ${exported.scan_group_count}件のスキャングループ / ${exported.tag_count}件のタグ`;
    } catch (error) {
      const message = extractErrorMessage(error, '設定エクスポートに失敗しました');
      tagActionMessage = message;
      notify(message);
    }
  }

  async function handleImportTagSettings() {
    if (!ensureDriverUiNotBusy()) {
      return;
    }

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (typeof selected !== 'string') {
        return;
      }

      const imported = await importTagManagementSettings(selected);
      await reloadTagManagementData();
      selectedDriver = null;
      selectedScanGroup = null;
      selectedTag = null;
      tagMode = 'detail';
      editorDriverId = null;
      tagActionMessage = `接続先設定をインポートしました: ${imported.driver_count}件の接続先 / ${imported.scan_group_count}件のスキャングループ / ${imported.tag_count}件のタグ`;
    } catch (error) {
      const message = extractErrorMessage(error, '設定インポートに失敗しました');
      tagActionMessage = message;
      notify(message);
    }
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

  onMount(() => {
    if (loadedDriverUiBaseDir) {
      return;
    }

    let disposed = false;
    void getDefaultDriverUiBaseDir()
      .then((defaultBaseDir) => {
        if (disposed || !defaultBaseDir) {
          return;
        }
        driverUiBaseDirInput = defaultBaseDir;
        driverUiBaseDirSaved = defaultBaseDir;
      })
      .catch(() => {
        // 既定値取得に失敗しても手入力できるため黙って継続する
      });

    return () => {
      disposed = true;
    };
  });

  let selectedDriver = $state<DriverDto | null>(null);
  let driverUiAvailableByType = $state<Record<string, boolean>>({});
  let discoveredDriverPackages = $state<DiscoveredDriverPackageDto[]>([]);
  let invalidDriverPackages = $state<InvalidDriverPackageDto[]>([]);
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
    if (!baseDir) {
      discoveredDriverPackages = [];
      invalidDriverPackages = [];
      driverUiAvailableByType = {};
      return;
    }

    let disposed = false;

    void discoverDriverPackages({ driver_ui_base_dir: baseDir })
      .then((result) => {
        if (disposed) {
          return;
        }

        discoveredDriverPackages = result.available;
        invalidDriverPackages = result.invalid;
        driverUiAvailableByType = buildDriverUiAvailabilityByType(result.available);
      })
      .catch((error) => {
        if (disposed) {
          return;
        }

        discoveredDriverPackages = [];
        invalidDriverPackages = [];
        driverUiAvailableByType = {};
        settingsMessage = extractErrorMessage(error, 'ドライバマニフェストの検出に失敗しました');
      });

    return () => {
      disposed = true;
    };
  });

  const driverTypeOptions = $derived.by<DriverTypeOption[]>(() =>
    buildDriverTypeOptions(
      $driversStore.items,
      discoveredDriverPackages,
      invalidDriverPackages,
      driverUiAvailableByType,
    ),
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
    discoveryAvailableCount: discoveredDriverPackages.length,
    discoveryInvalidCount: invalidDriverPackages.length,
    discoveryInvalidItems: invalidDriverPackages.map((item) => ({
      manifestPath: item.manifest_path,
      statusCode: item.status_code,
      statusMessage: item.status_message,
      driverTypeHint: item.driver_type_hint,
    })),
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
    onImportSettings: handleImportTagSettings,
    onExportSettings: handleExportTagSettings,
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
