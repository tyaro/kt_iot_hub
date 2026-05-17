<script lang="ts">
  import DashboardContent from './DashboardContent.svelte';
  import PublishersContent from './PublishersContent.svelte';
  import TagsContent from './TagsContent.svelte';
  import SettingsContent from './SettingsContent.svelte';
  import PlaceholderContent from './PlaceholderContent.svelte';
  import type { DriverDto, RuntimeStatusDto, ScanGroupDto, TagDto } from '$lib/ipc';
  import type { PageId } from './constants';

  type Props = {
    currentPage: PageId;
    tagCount: number;
    driverCount: number;
    enabledDriverCount: number;
    runtimeStatus: RuntimeStatusDto;
    runtimeBusy: boolean;
    dashboardMessage: string;
    driverUiPolling: boolean;
    tagActionMessage: string;
    selectedTagId: string | null;
    selectedDriverId: string | null;
    selectedScanGroupId: string | null;
    driverUiBaseDirInput: string;
    driverUiBaseDirSaved: string | null;
    settingsMessage: string;
    onNavigateTags: () => void;
    onOpenMqttMonitor: () => void;
    onStartServers: () => Promise<void>;
    onStopServers: () => Promise<void>;
    onNewDriver: () => Promise<void>;
    onSelectTag: (tag: TagDto | null) => void;
    onSelectDriver: (driver: DriverDto | null) => void;
    onSelectScanGroup: (scanGroup: ScanGroupDto | null) => void;
    onRequestNewTag: (driverId: string) => void;
    onRequestEditDriver: (driverId: string) => void;
    onRequestDeleteDriver: (driverId: string) => Promise<void>;
    onRequestEditTag: (tag: TagDto) => void;
    onRequestDeleteTag: (tag: TagDto) => Promise<void>;
    onDriverUiBaseDirInput: (value: string) => void;
    onPickDriverUiBaseDir: () => Promise<void>;
    onSaveDriverUiBaseDir: () => void;
    onClearDriverUiBaseDir: () => void;
  };

  let {
    currentPage,
    tagCount,
    driverCount,
    enabledDriverCount,
    runtimeStatus,
    runtimeBusy,
    dashboardMessage,
    driverUiPolling,
    tagActionMessage,
    selectedTagId,
    selectedDriverId,
    selectedScanGroupId,
    driverUiBaseDirInput,
    driverUiBaseDirSaved,
    settingsMessage,
    onNavigateTags,
    onOpenMqttMonitor,
    onStartServers,
    onStopServers,
    onNewDriver,
    onSelectTag,
    onSelectDriver,
    onSelectScanGroup,
    onRequestNewTag,
    onRequestEditDriver,
    onRequestDeleteDriver,
    onRequestEditTag,
    onRequestDeleteTag,
    onDriverUiBaseDirInput,
    onPickDriverUiBaseDir,
    onSaveDriverUiBaseDir,
    onClearDriverUiBaseDir,
  }: Props = $props();
</script>

<div class="center-pane">
  {#if currentPage === 'dashboard'}
    <DashboardContent
      {tagCount}
      {driverCount}
      {enabledDriverCount}
      {runtimeStatus}
      {runtimeBusy}
      {dashboardMessage}
      {onNavigateTags}
      {onOpenMqttMonitor}
      onStartServers={onStartServers}
      onStopServers={onStopServers}
    />

  {:else if currentPage === 'tags'}
    <TagsContent
      {driverUiPolling}
      {tagActionMessage}
      {selectedTagId}
      {selectedDriverId}
      {selectedScanGroupId}
      onNewDriver={onNewDriver}
      onSelectTag={onSelectTag}
      onSelectDriver={onSelectDriver}
      onSelectScanGroup={onSelectScanGroup}
      onRequestNewTag={onRequestNewTag}
      onRequestEditDriver={onRequestEditDriver}
      onRequestDeleteDriver={onRequestDeleteDriver}
      onRequestEditTag={onRequestEditTag}
      onRequestDeleteTag={onRequestDeleteTag}
    />

  {:else if currentPage === 'publishers'}
    <PublishersContent />

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
      onDriverUiBaseDirInput={onDriverUiBaseDirInput}
      onPickDriverUiBaseDir={onPickDriverUiBaseDir}
      onSaveDriverUiBaseDir={onSaveDriverUiBaseDir}
      onClearDriverUiBaseDir={onClearDriverUiBaseDir}
    />
  {/if}
</div>

<style>
  .center-pane {
    flex: 1;
    overflow-y: auto;
    background-color: #f4f6f9;
  }
</style>