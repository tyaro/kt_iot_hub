import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';
import {
  closeTagEditor as closeTagEditorState,
  handleDriverSelect,
  handleScanGroupSelect,
  handleTagSelect,
  onTagEditorDone as onTagEditorDoneFlow,
  openManualTagEditor as openManualTagEditorState,
  type SelectionState,
} from './handlers';

export type CreateSelectionControllerDeps = {
  getState: () => SelectionState;
  setState: (state: SelectionState) => void;
  getDrivers: () => DriverDto[];
  getScanGroups: () => ScanGroupDto[];
  reloadTags: () => Promise<void>;
};

export type SelectionController = {
  update: (mutator: (state: SelectionState) => void) => void;
  onTagSelect: (tag: TagDto | null) => void;
  onDriverSelect: (driver: DriverDto | null) => void;
  onScanGroupSelect: (scanGroup: ScanGroupDto | null) => void;
  openManualTagEditor: (driverId: string, mode: 'new' | 'edit', tag?: TagDto | null) => void;
  onTagEditorDone: () => Promise<void>;
  closeTagEditor: () => void;
  onTagEditorCancel: () => void;
  onTagDetailClose: () => void;
};

export function createSelectionController(
  deps: CreateSelectionControllerDeps,
): SelectionController {
  function update(mutator: (state: SelectionState) => void) {
    const state = deps.getState();
    mutator(state);
    deps.setState(state);
  }

  function updateAsync(mutator: (state: SelectionState) => Promise<void>) {
    const state = deps.getState();
    return mutator(state).then(() => {
      deps.setState(state);
    });
  }

  function onTagSelect(tag: TagDto | null) {
    update((state) => {
      handleTagSelect(state, tag, deps.getDrivers(), deps.getScanGroups());
    });
  }

  function onDriverSelect(driver: DriverDto | null) {
    update((state) => {
      handleDriverSelect(state, driver);
    });
  }

  function onScanGroupSelect(scanGroup: ScanGroupDto | null) {
    update((state) => {
      handleScanGroupSelect(state, scanGroup, deps.getDrivers());
    });
  }

  function openManualTagEditor(driverId: string, mode: 'new' | 'edit', tag?: TagDto | null) {
    update((state) => {
      openManualTagEditorState(state, driverId, mode, tag);
    });
  }

  async function onTagEditorDone() {
    await updateAsync(async (state) => {
      await onTagEditorDoneFlow(state, deps.reloadTags);
    });
  }

  function closeTagEditor() {
    update((state) => {
      closeTagEditorState(state);
    });
  }

  function onTagEditorCancel() {
    update((state) => {
      closeTagEditorState(state);
      state.tagActionMessage = '';
    });
  }

  function onTagDetailClose() {
    update((state) => {
      state.selectedTag = null;
      state.editorDriverId = null;
      state.tagActionMessage = '';
    });
  }

  return {
    update,
    onTagSelect,
    onDriverSelect,
    onScanGroupSelect,
    openManualTagEditor,
    onTagEditorDone,
    closeTagEditor,
    onTagEditorCancel,
    onTagDetailClose,
  };
}