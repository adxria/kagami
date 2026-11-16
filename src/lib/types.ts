// Keep this small boundary aligned with crates/kagami-core/src/types.rs.
export interface Context {
  id: string;
  name: string;
  parentId: string | null;
  color: string;
  trackingLocked: boolean;
  position: number;
}
export interface Entry {
  id: string;
  contextId: string | null;
  startedAt: number;
  endedAt: number | null;
}
export interface Query {
  weekStart: number;
  weekEnd: number;
  todayStart: number;
  todayEnd: number;
}
export interface Totals {
  todayMs: number;
  weekMs: number;
  lifetimeMs: number;
}
export interface Workspace {
  contexts: Context[];
  recordings: Entry[];
  running: Entry | null;
  overlapIds: string[];
  totals: Totals;
  unassigned: Totals;
  contextTotals: { contextId: string; direct: Totals; total: Totals }[];
  calculatedAt: number;
  query: Query;
}
export type Change =
  | { type: 'create_context'; name: string; parentId: string | null; color: string }
  | { type: 'rename_context'; id: string; name: string }
  | { type: 'color_context'; id: string; color: string }
  | { type: 'move_context'; id: string; parentId: string | null; position: number }
  | { type: 'lock_context'; id: string; locked: boolean }
  | { type: 'delete_context'; id: string }
  | { type: 'start_timer'; contextId: string | null }
  | { type: 'stop_timer'; id: string }
  | { type: 'assign_timer'; id: string; contextId: string | null }
  | { type: 'create_recording'; contextId: string | null; startedAt: number; endedAt: number }
  | {
      type: 'edit_recording';
      id: string;
      contextId: string | null;
      startedAt: number;
      endedAt: number | null;
      expectedRunning: boolean;
    }
  | { type: 'delete_recording'; id: string; expectedRunning: boolean };
export interface Receipt {
  id: string;
}
export type Page = { kind: 'week' } | { kind: 'unassigned' } | { kind: 'context'; id: string };
export interface EntryDraft {
  contextId: string | null;
  startedAt: number;
  endedAt: number;
}
export function errorMessage(error: unknown): string {
  return typeof error === 'object' && error && 'message' in error
    ? String(error.message)
    : String(error);
}
