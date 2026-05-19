<script lang="ts">
  import DriverPickerDialog from '../../driver/DriverPickerDialog.svelte';
  import DriverTypePickerDialog from '../../driver/DriverTypePickerDialog.svelte';
  import type { DriverDto } from '$lib/ipc';

  type DriverTypeOption = {
    driverType: string;
    label: string;
    available: boolean;
    description: string;
    statusMessage?: string;
  };

  let {
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
  }: {
    driverPickerOpen: boolean;
    drivers: DriverDto[];
    driversLoading: boolean;
    driversError: string | null;
    driverTypePickerOpen: boolean;
    driverTypeOptions: DriverTypeOption[];
    onReloadDrivers: () => void | Promise<void>;
    onCloseDriverPicker: () => void;
    onSelectDriver: (driverId: string) => void;
    onCloseDriverTypePicker: () => void;
    onSelectDriverType: (driverType: string) => void | Promise<void>;
    confirmDialogOpen: boolean;
    confirmDialogTitle: string;
    confirmDialogMessage: string;
    confirmDialogConfirmLabel: string;
    confirmDialogCancelLabel: string;
    onConfirmDialogConfirm: () => void;
    onConfirmDialogCancel: () => void;
  } = $props();
</script>

<DriverPickerDialog
  open={driverPickerOpen}
  {drivers}
  loading={driversLoading}
  error={driversError ?? undefined}
  onReload={onReloadDrivers}
  onClose={onCloseDriverPicker}
  onSelect={onSelectDriver}
/>

<DriverTypePickerDialog
  open={driverTypePickerOpen}
  options={driverTypeOptions}
  onClose={onCloseDriverTypePicker}
  onSelect={onSelectDriverType}
/>

{#if confirmDialogOpen}
  <div class="overlay" role="presentation" onclick={onConfirmDialogCancel}>
    <div
      class="dialog confirm-dialog"
      role="dialog"
      aria-modal="true"
      aria-label={confirmDialogTitle}
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          onConfirmDialogCancel();
        }
      }}
    >
      <div class="header">
        <h3>{confirmDialogTitle}</h3>
        <button type="button" class="btn-close" onclick={onConfirmDialogCancel} aria-label="閉じる">✕</button>
      </div>

      <p class="description confirm-message">{confirmDialogMessage}</p>

      <div class="actions confirm-actions">
        <button type="button" class="btn-cancel" onclick={onConfirmDialogCancel}>{confirmDialogCancelLabel}</button>
        <button type="button" class="btn-danger" onclick={onConfirmDialogConfirm}>{confirmDialogConfirmLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1500;
    padding: 16px;
  }

  .dialog {
    width: min(480px, 100%);
    max-height: 80vh;
    overflow: auto;
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.22);
    padding: 16px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .header h3 {
    margin: 0;
    font-size: 1rem;
    color: #1e293b;
  }

  .btn-close {
    border: none;
    background: transparent;
    font-size: 1rem;
    color: #64748b;
    cursor: pointer;
  }

  .description {
    margin: 0 0 12px;
    font-size: 0.85rem;
    color: #475569;
    line-height: 1.5;
  }

  .confirm-message {
    white-space: pre-line;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }

  .btn-cancel,
  .btn-danger {
    border: none;
    border-radius: 6px;
    padding: 7px 12px;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .btn-cancel {
    background: #e2e8f0;
    color: #334155;
  }

  .btn-danger {
    background: #dc2626;
    color: #fff;
  }

  .btn-danger:hover {
    background: #b91c1c;
  }
</style>
