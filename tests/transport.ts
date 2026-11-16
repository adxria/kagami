import type { Change, Query, Receipt, Workspace } from '../src/lib/types';
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
export async function subscribe(_changed: () => void, _failed: (error: unknown) => void) {
  return () => {};
}
