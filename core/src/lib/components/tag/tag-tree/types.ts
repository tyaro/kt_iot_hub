import type { TagDto } from '$lib/ipc';

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  kind?: 'driver' | 'tag';
  driverId?: string;
  tag?: TagDto;
}
