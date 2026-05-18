// タグ・ドライバ用グローバル store
// writable store を使い、テンプレートで $tagsStore.items のようにアクセスする

import { writable } from 'svelte/store';
import {
  listDrivers,
  listScanGroups,
  listTags,
  type DriverDto,
  type ScanGroupDto,
  type TagDto,
} from '$lib/ipc/index';

// ─── タグ ────────────────────────────────────────────
interface TagsState { items: TagDto[]; loading: boolean; error: string; }

export const tagsStore = writable<TagsState>({ items: [], loading: false, error: '' });

export async function reloadTags(): Promise<void> {
  tagsStore.update(s => ({ ...s, loading: true, error: '' }));
  try {
    const items = await listTags();
    tagsStore.update(s => ({ ...s, items, loading: false }));
  } catch (e) {
    tagsStore.update(s => ({
      ...s,
      loading: false,
      error: e instanceof Error ? e.message : 'タグ取得失敗',
    }));
  }
}

// ─── スキャングループ ─────────────────────────────────
interface ScanGroupsState { items: ScanGroupDto[]; loading: boolean; error: string; }

export const scanGroupsStore = writable<ScanGroupsState>({ items: [], loading: false, error: '' });

export async function reloadScanGroups(driverId?: string): Promise<void> {
  scanGroupsStore.update((s) => ({ ...s, loading: true, error: '' }));
  try {
    const items = await listScanGroups(driverId);
    scanGroupsStore.update((s) => ({ ...s, items, loading: false }));
  } catch (e) {
    scanGroupsStore.update((s) => ({
      ...s,
      loading: false,
      error: e instanceof Error ? e.message : 'スキャングループ取得失敗',
    }));
  }
}

// ─── ドライバ ─────────────────────────────────────────
interface DriversState { items: DriverDto[]; loading: boolean; error: string; }

export const driversStore = writable<DriversState>({ items: [], loading: false, error: '' });

export async function reloadDrivers(): Promise<void> {
  driversStore.update(s => ({ ...s, loading: true, error: '' }));
  try {
    const items = await listDrivers();
    driversStore.update(s => ({ ...s, items, loading: false }));
  } catch (e) {
    driversStore.update(s => ({
      ...s,
      loading: false,
      error: e instanceof Error ? e.message : 'ドライバ取得失敗',
    }));
  }
}

export async function reloadAllRegistry(): Promise<void> {
  await reloadDrivers();
  await reloadScanGroups();
  await reloadTags();
}


