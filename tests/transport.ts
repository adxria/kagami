import type { Change, DataPreview, ImportMode, Query, Receipt, Workspace } from '../src/lib/types';
export const available = true;
export const currentTime = () => Date.now();
const session = (sessionStorage.kagamiTestSession ??= crypto.randomUUID());
export async function rpc(method: string, extra: Record<string, unknown> = {}) {
  const response = await fetch('/__test__/rpc', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-kagami-session': session },
    body: JSON.stringify({ method, now: currentTime(), ...extra }),
  });
  const result = await response.json();
  if ('error' in result) throw result.error;
  return result.ok;
}
export const read = (query: Query): Promise<Workspace> => rpc('read', { query });
export const apply = (change: Change): Promise<Receipt> => rpc('change', { change });
export const exportData = (): Promise<boolean> => rpc('export_data');
export const chooseImport = (recovery: boolean): Promise<DataPreview | null> =>
  rpc('choose_import', { recovery });
export const cancelImport = (token: string): Promise<void> => rpc('cancel_import', { token });
export const applyImport = (token: string, mode: ImportMode): Promise<boolean> =>
  rpc('apply_import', { token, mode });
export const hasRecoveryFiles = (): Promise<boolean> => rpc('has_recovery_files');
export async function subscribe(_changed: () => void, _failed: (error: unknown) => void) {
  return () => {};
}
