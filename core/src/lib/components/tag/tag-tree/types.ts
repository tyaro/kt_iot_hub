import type { TagDto } from '$lib/ipc';
import type { ScanGroupDto } from '$lib/ipc';

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  kind?: 'driver' | 'scan-group' | 'tag';
  driverId?: string;
  scanGroup?: ScanGroupDto;
  tag?: TagDto;
}
