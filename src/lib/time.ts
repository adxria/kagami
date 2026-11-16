export const MINUTE = 60_000;
export const HOUR = 60 * MINUTE;

export function dayStart(value: number | Date): number {
  const date = new Date(value);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

export function addDays(value: number, days: number): number {
  const date = new Date(value);
  date.setDate(date.getDate() + days);
  return date.getTime();
}

export function weekStart(value: number): number {
  const day = dayStart(value);
  return addDays(day, -((new Date(day).getDay() + 6) % 7));
}

export function duration(value: number, seconds = false): string {
  const total = Math.max(0, Math.floor(value / 1_000));
  const h = Math.floor(total / 3_600);
  const m = Math.floor(total / 60) % 60;
  const s = total % 60;
  if (seconds)
    return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  if (h) return `${h}h ${String(m).padStart(2, '0')}m`;
  if (m) return `${m}m ${s}s`;
  return `${s}s`;
}

export function clock(value: number, seconds = false): string {
  return new Date(value).toLocaleTimeString('en-GB', {
    hour: '2-digit',
    minute: '2-digit',
    ...(seconds ? { second: '2-digit' } : {}),
  });
}

export function dateLabel(value: number): string {
  return new Date(value).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  });
}

export function localInput(value: number): string {
  const date = new Date(value);
  const part = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${part(date.getMonth() + 1)}-${part(date.getDate())}T${part(date.getHours())}:${part(date.getMinutes())}:${part(date.getSeconds())}`;
}

/** Enumerate repeated local times, and reject nonexistent spring-forward times. */
export function localCandidates(value: string): number[] {
  // datetime-local controls omit the seconds portion when it is zero.
  if (value.length === 16) value += ':00';
  const parsed = new Date(value).getTime();
  if (!Number.isFinite(parsed)) return [];
  const offsets = new Set<number>();
  for (const delta of [-36, -12, 0, 12, 36])
    offsets.add(new Date(parsed + delta * HOUR).getTimezoneOffset());
  const [year, month, day, hour, minute, second] = utcParts(value);
  const utc = new Date(0);
  utc.setUTCFullYear(year, month, day);
  utc.setUTCHours(hour, minute, second, 0);
  const localAsUtc = utc.getTime();
  return [...offsets]
    .map((offset) => localAsUtc + offset * MINUTE)
    .filter((instant) => localInput(instant) === value)
    .sort((a, b) => a - b);
}

function utcParts(value: string): [number, number, number, number, number, number] {
  const [year, month, day, hour, minute, second = 0] = value.split(/[-T:]/).map(Number);
  return [year, month - 1, day, hour, minute, second];
}

export function offsetLabel(value: number): string {
  const offset = -new Date(value).getTimezoneOffset();
  return `UTC${offset >= 0 ? '+' : '−'}${String(Math.floor(Math.abs(offset) / 60)).padStart(2, '0')}:${String(Math.abs(offset) % 60).padStart(2, '0')}`;
}

/** Preserve subsecond precision until the displayed wall time or offset changes. */
export function resolveLocal(value: string, preferred: number | null): number | undefined {
  if (preferred !== null && localInput(preferred) === value) return preferred;
  return localCandidates(value)[0];
}
