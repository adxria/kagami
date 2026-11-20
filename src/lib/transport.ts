import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Change, DataPreview, ImportMode, Query, Receipt, Workspace } from './types';
export const available = isTauri();
export const currentTime = () => Date.now();
export const read = (query: Query) => invoke<Workspace>('read_workspace', { query });
export const apply = (change: Change) => invoke<Receipt>('apply_change', { change });
export const exportData = () => invoke<boolean>('export_data');
export const chooseImport = (recovery: boolean) =>
  invoke<DataPreview | null>('choose_import', { recovery });
export const cancelImport = (token: string) => invoke<void>('cancel_import', { token });
export const applyImport = (token: string, mode: ImportMode) =>
  invoke<boolean>('apply_import', { token, mode });
export const hasRecoveryFiles = () => invoke<boolean>('has_recovery_files');
export async function subscribe(changed: () => void, failed: (error: unknown) => void) {
  const cleanups: (() => void)[] = [];
  try {
    cleanups.push(await listen('workspace-changed', changed));
    cleanups.push(await listen('desktop-error', (event) => failed(event.payload)));
    return () => cleanups.forEach((cleanup) => cleanup());
  } catch (error) {
    cleanups.forEach((cleanup) => cleanup());
    throw error;
  }
}
