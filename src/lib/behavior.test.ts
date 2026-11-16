import { Controller } from './controller.svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { addDays, HOUR, localCandidates, localInput, resolveLocal } from './time';
import { entryTimeParts, partsInput } from './entry-time';
import { layoutDay, zoomAt, draftAt } from './timeline';
import type { Entry } from './types';
const zone = process.env.TZ;
afterEach(() => {
  if (zone) process.env.TZ = zone;
  else delete process.env.TZ;
});
const entry = (id: string, start: number, end: number | null): Entry => ({
  id,
  contextId: null,
  startedAt: start,
  endedAt: end,
});
it('resolves DST gaps and repeated offsets while calendar days retain their real lengths', () => {
  process.env.TZ = 'America/New_York';
  expect(localCandidates('2026-03-08T02:30:00')).toEqual([]);
  const repeated = localCandidates('2026-11-01T01:30:00');
  expect(repeated).toHaveLength(2);
  expect(repeated[1] - repeated[0]).toBe(HOUR);
  const spring = new Date(2026, 2, 8).getTime(),
    fall = new Date(2026, 10, 1).getTime();
  expect(addDays(spring, 1) - spring).toBe(23 * HOUR);
  expect(addDays(fall, 1) - fall).toBe(25 * HOUR);
  expect(draftAt(fall, 150, 1, addDays(fall, 1))?.startedAt).toBe(repeated[1]);
});
it('preserves unchanged milliseconds and chosen repeated offsets, rejecting invalid calendar edits', () => {
  process.env.TZ = 'America/New_York';
  const original = localCandidates('2026-11-01T01:30:00')[1] + 123;
  const text = partsInput(entryTimeParts(original));
  expect(resolveLocal(text, original)).toBe(original);
  expect(resolveLocal(text, localCandidates(text)[0])).toBe(localCandidates(text)[0]);
  expect(resolveLocal('2026-11-01T01:31:00', original)).not.toBe(original);
  expect(partsInput({ ...entryTimeParts(original), month: '02', day: '30' })).toBe('');
  expect(partsInput({ ...entryTimeParts(original), second: '' })).toBe('');
  expect(localInput(resolveLocal(text, original)!)).toBe(text);
});
it('clips across midnight and uses actual intervals rather than marker size for overlap lanes', () => {
  const entries = [
    entry('overnight', 23 * HOUR, 25 * HOUR),
    entry('adjacent', 25 * HOUR, 25 * HOUR + 1000),
    entry('next', 25 * HOUR + 2000, 25 * HOUR + 3000),
  ];
  expect(layoutDay(entries, 0, 24 * HOUR, 30 * HOUR, 1).segments[0].height).toBe(60);
  const day = layoutDay(entries, 24 * HOUR, 48 * HOUR, 30 * HOUR, 1);
  expect(day.segments[0].height).toBe(60);
  expect(day.segments.every((s) => s.lanes === 1)).toBe(true);
  expect(day.groups[0].segments).toHaveLength(2);
  const overlap = layoutDay(
    [entry('a', HOUR, 2 * HOUR), entry('b', HOUR + 1, 3 * HOUR), entry('c', 3 * HOUR, 4 * HOUR)],
    0,
    24 * HOUR,
    24 * HOUR,
    1,
  );
  expect(overlap.segments.map((s) => s.lanes)).toEqual([2, 2, 1]);
});
it('keeps transitive marker groups inside the day and separates them when zoomed', () => {
  const entries = [
    entry('a', 60_000, 61_000),
    entry('b', 80_000, 81_000),
    entry('c', 24 * HOUR - 1, 24 * HOUR),
  ];
  expect(layoutDay(entries, 0, 24 * HOUR, 24 * HOUR, 1).groups).toHaveLength(2);
  expect(layoutDay(entries, 0, 24 * HOUR, 24 * HOUR, 60).groups).toHaveLength(3);
  const last = layoutDay(entries, 0, 24 * HOUR, 24 * HOUR, 1).groups.at(-1)!;
  expect(last.top + last.height).toBeLessThanOrEqual(1440);
  expect(layoutDay([entry('zero', 1, 1)], 0, 24 * HOUR, 24 * HOUR, 1).groups).toHaveLength(1);
});
it('anchors zoom and refuses clicks outside a local day or after now', () => {
  const result = zoomAt(1, -100, 500, 200);
  expect((result.scroll + 200) / result.zoom).toBeCloseTo(700);
  expect(zoomAt(0.5, 1000, 0, 0).zoom).toBe(0.5);
  expect(zoomAt(60, -1000, 0, 0).zoom).toBe(60);
  process.env.TZ = 'UTC';
  const day = new Date(2026, 10, 11).getTime();
  expect(draftAt(day, 719, 1, day + 12 * HOUR)?.endedAt).toBe(day + 12 * HOUR);
  expect(draftAt(day, 721, 1, day + 12 * HOUR)).toBeNull();
  expect(draftAt(day, 1440, 1, day + 48 * HOUR)).toBeNull();
});

vi.mock('$transport', () => ({
  available: true,
  currentTime: () => Date.parse('2026-11-16T12:00:00Z'),
}));
it('retains the selected calendar Monday when the system timezone changes', () => {
  process.env.TZ = 'Asia/Yekaterinburg';
  const app = new Controller();
  const date = new Date(app.week).getDate();
  process.env.TZ = 'America/Los_Angeles';
  const query = app.query();
  expect(new Date(query.weekStart).getDate()).toBe(date);
  expect(new Date(query.weekStart).getDay()).toBe(1);
  expect(new Date(query.weekStart).getHours()).toBe(0);
  expect(new Date(query.todayStart).getHours()).toBe(0);
});
