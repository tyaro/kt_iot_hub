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
    confirmAction,
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
    importDriverUiResult,
    launchDriverUi,
    startRuntimeServices,
    stopRuntimeServices,
    type DriverDto,
    type RuntimeStatusDto,
    type ScanGroupDto,
    type TagDto,
  } from '$lib/ipc/index';

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
  let runtimeBusy = $state(false);
  let dashboardMessage = $state('');
  let settingsMessage = $state('');

  const loadedDriverUiBaseDir = loadDriverUiBaseDirFromStorage();
  let driverUiBaseDirInput = $state(loadedDriverUiBaseDir ?? '');
  let driverUiBaseDirSaved = $state<string | null>(loadedDriverUiBaseDir);

  let selectedDriver = $state<DriverDto | null>(null);
  let driverUiAvailableByType = $state<Record<string, boolean>>({});

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
    {runtimeBusy}
    {dashboardMessage}
    {driverUiPolling}
    {tagActionMessage}
    selectedTagId={selectedTag?.id ?? null}
    selectedDriverId={selectedDriver?.id ?? null}
    selectedScanGroupId={selectedScanGroup?.id ?? null}
    {driverUiBaseDirInput}
    {driverUiBaseDirSaved}
    {settingsMessage}
    onNavigateTags={() => selectPage('tags')}
    onStartServers={() => runtimeController.runAction(startRuntimeServices, 'バックグラウンドサービスを起動しました。', 'サービス起動に失敗しました')}
    onStopServers={() => runtimeController.runAction(stopRuntimeServices, 'バックグラウンドサービスを停止しました。', 'サービス停止に失敗しました')}
    onNewDriver={tagUiController.newDriver}
    onNewTag={tagUiController.newTag}
    onSelectTag={selectionController.onTagSelect}
    onSelectDriver={selectionController.onDriverSelect}
    onSelectScanGroup={selectionController.onScanGroupSelect}
    onRequestNewTag={tagUiController.requestNewTagForDriver}
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
    onDriverDone={deletionController.onDriverDetailDone}
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
