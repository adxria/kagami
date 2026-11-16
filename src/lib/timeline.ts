import type { Entry, EntryDraft } from './types';
import { addDays, MINUTE } from './time';

export interface Segment {
  entry: Entry;
  start: number;
  end: number;
  top: number;
  height: number;
  lane: number;
  lanes: number;
  tiny: boolean;
}

export interface MarkerGroup {
  top: number;
  height: number;
  segments: Segment[];
}

export function layoutDay(
  entries: Entry[],
  day: number,
  nextDay: number,
  now: number,
  zoom: number,
): { segments: Segment[]; groups: MarkerGroup[] } {
  const segments: Segment[] = entries
    .map((entry) => ({
      entry,
      start: Math.max(day, entry.startedAt),
      end: Math.min(nextDay, entry.endedAt ?? now),
      top: 0,
      height: 0,
      lane: 0,
      lanes: 1,
      tiny: false,
    }))
    .filter(
      (segment) =>
        segment.start < segment.end ||
        (segment.entry.startedAt === segment.entry.endedAt &&
          segment.entry.startedAt >= day &&
          segment.entry.startedAt < nextDay),
    )
    .sort((a, b) => a.start - b.start || b.end - a.end || a.entry.id.localeCompare(b.entry.id));

  // Assign lanes by real half-open intervals, never enlarged marker geometry.
  let cluster: Segment[] = [];
  let clusterEnd = -Infinity;
  const laneEnds: number[] = [];
  function finishCluster() {
    for (const segment of cluster) segment.lanes = laneEnds.length;
    cluster = [];
    laneEnds.length = 0;
  }
  for (const segment of segments) {
    if (segment.start === segment.end) {
      segment.top = ((segment.start - day) / MINUTE) * zoom;
      segment.tiny = true;
      continue;
    }
    if (segment.start >= clusterEnd) finishCluster();
    let lane = laneEnds.findIndex((end) => end <= segment.start);
    if (lane === -1) lane = laneEnds.length;
    laneEnds[lane] = segment.end;
    segment.lane = lane;
    cluster.push(segment);
    clusterEnd = Math.max(clusterEnd, segment.end);
    segment.top = ((segment.start - day) / MINUTE) * zoom;
    segment.height = ((segment.end - segment.start) / MINUTE) * zoom;
    segment.tiny = segment.height < 16;
  }
  finishCluster();

  const dayHeight = ((nextDay - day) / MINUTE) * zoom;
  const groups: MarkerGroup[] = [];
  for (const segment of segments.filter((segment) => segment.tiny)) {
    const top = Math.max(0, Math.min(segment.top, dayHeight - 16));
    const last = groups.at(-1);
    if (last && top < last.top + last.height + 2) {
      last.segments.push(segment);
      // Use the full collision envelope for transitive grouping, not a fake duration.
      last.height = Math.max(last.height, top + 16 - last.top);
    } else groups.push({ top, height: 16, segments: [segment] });
  }
  return { segments, groups };
}

export function zoomAt(
  oldZoom: number,
  delta: number,
  scroll: number,
  pointerY: number,
): { zoom: number; scroll: number } {
  const zoom = Math.max(0.5, Math.min(60, oldZoom * Math.exp(-delta * 0.002)));
  return { zoom, scroll: Math.max(0, ((scroll + pointerY) / oldZoom) * zoom - pointerY) };
}

/** Grid coordinates are elapsed minutes, including on daylight-saving transition days. */
export function draftAt(day: number, y: number, zoom: number, now: number): EntryDraft | null {
  const dayEnd = addDays(day, 1);
  const clicked = day + (y / zoom) * MINUTE;
  if (!Number.isFinite(clicked) || y < 0 || clicked >= dayEnd || clicked >= now) return null;
  const startedAt = Math.floor(clicked / MINUTE) * MINUTE;
  const endedAt = Math.min(startedAt + 30 * MINUTE, dayEnd, now);
  return endedAt > startedAt ? { contextId: null, startedAt, endedAt } : null;
}
