<script lang="ts">
  import DriverPickerDialog from '../../driver/DriverPickerDialog.svelte';
  import DriverTypePickerDialog from '../../driver/DriverTypePickerDialog.svelte';
  import type { DriverDto } from '$lib/ipc';

  type DriverTypeOption = {
    driverType: string;
    label: string;
    available: boolean;
    description: string;
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
