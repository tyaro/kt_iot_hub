<script lang="ts">
  import TagRightPane from './TagRightPane.svelte';
  import DriverUiDialogs from './DriverUiDialogs.svelte';
  import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';
  import type { DriverTypeOption } from './driverUiFlow';
  import type { PageId } from './constants';

  type Props = {
    currentPage: PageId;
    tagMode: 'detail' | 'new' | 'edit';
    selectedTag: TagDto | null;
    selectedDriver: DriverDto | null;
    selectedScanGroup: ScanGroupDto | null;
    editorDriverId: string | null;
    onTagEditorDone: () => Promise<void>;
    onTagEditorCancel: () => void;
    onTagDetailEdit: (tag: TagDto) => void;
    onTagDetailDelete: (tag: TagDto) => Promise<void>;
    onTagDetailClose: () => void;
    onDriverDelete: (driverId: string) => Promise<void>;
    onDriverDone: (driverId?: string) => Promise<DriverDto | null>;
    driverPickerOpen: boolean;
    drivers: DriverDto[];
    driversLoading: boolean;
    driversError: string | null;
    driverTypePickerOpen: boolean;
    driverTypeOptions: DriverTypeOption[];
    onReloadDrivers: () => Promise<void>;
    onCloseDriverPicker: () => void;
    onSelectDriver: (driverId: string) => void;
    onCloseDriverTypePicker: () => void;
    onSelectDriverType: (driverType: string) => Promise<void>;
    confirmDialogOpen: boolean;
    confirmDialogTitle: string;
    confirmDialogMessage: string;
    confirmDialogConfirmLabel: string;
    confirmDialogCancelLabel: string;
    onConfirmDialogConfirm: () => void;
    onConfirmDialogCancel: () => void;
  };

  let {
    currentPage,
    tagMode,
    selectedTag,
    selectedDriver,
    selectedScanGroup,
    editorDriverId,
    onTagEditorDone,
    onTagEditorCancel,
    onTagDetailEdit,
    onTagDetailDelete,
    onTagDetailClose,
    onDriverDelete,
    onDriverDone,
    driverPickerOpen,
    drivers,
    driversLoading,
    driversError,
    driverTypePickerOpen,
    driverTypeOptions,
    onReloadDrivers,
    onCloseDriverPicker,
    onSelectDriver,
    onCloseDriverTypePicker,
    onSelectDriverType,
    confirmDialogOpen,
    confirmDialogTitle,
    confirmDialogMessage,
    confirmDialogConfirmLabel,
    confirmDialogCancelLabel,
    onConfirmDialogConfirm,
    onConfirmDialogCancel,
  }: Props = $props();
</script>

<TagRightPane
  {currentPage}
  {tagMode}
  {selectedTag}
  {selectedDriver}
  {selectedScanGroup}
  {editorDriverId}
  onTagEditorDone={onTagEditorDone}
  onTagEditorCancel={onTagEditorCancel}
  onTagDetailEdit={onTagDetailEdit}
  onTagDetailDelete={onTagDetailDelete}
  onTagDetailClose={onTagDetailClose}
  onDriverDelete={onDriverDelete}
  onDriverDone={onDriverDone}
/>

<DriverUiDialogs
  {driverPickerOpen}
  {drivers}
  {driversLoading}
  {driversError}
  {driverTypePickerOpen}
  {driverTypeOptions}
  onReloadDrivers={onReloadDrivers}
  onCloseDriverPicker={onCloseDriverPicker}
  onSelectDriver={onSelectDriver}
  onCloseDriverTypePicker={onCloseDriverTypePicker}
  onSelectDriverType={onSelectDriverType}
  confirmDialogOpen={confirmDialogOpen}
  confirmDialogTitle={confirmDialogTitle}
  confirmDialogMessage={confirmDialogMessage}
  confirmDialogConfirmLabel={confirmDialogConfirmLabel}
  confirmDialogCancelLabel={confirmDialogCancelLabel}
  onConfirmDialogConfirm={onConfirmDialogConfirm}
  onConfirmDialogCancel={onConfirmDialogCancel}
/>