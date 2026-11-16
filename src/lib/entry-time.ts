import { localInput } from './time';

export type DateSegment = 'day' | 'month' | 'year';
export type TimeSegment = 'hour' | 'minute' | 'second';
export type Segment = DateSegment | TimeSegment;
export type DateTimeParts = Record<Segment, string>;
export type DatePart = { segment: DateSegment } | { literal: string };
export interface DateTimeProblem {
  segment: Segment;
  message: string;
}

export const timeSegments: TimeSegment[] = ['hour', 'minute', 'second'];

export function dateLayout(locale?: string): DatePart[] {
  return new Intl.DateTimeFormat(locale, {
    calendar: 'gregory',
    numberingSystem: 'latn',
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  })
    .formatToParts(new Date(2026, 10, 15))
    .flatMap((part): DatePart[] => {
      if (part.type === 'day' || part.type === 'month' || part.type === 'year')
        return [{ segment: part.type }];
      return part.type === 'literal' ? [{ literal: part.value }] : [];
    });
}

export function dateOrder(layout: DatePart[]): DateSegment[] {
  return layout.flatMap((part) => ('segment' in part ? [part.segment] : []));
}

export function entryTimeParts(instant: number): DateTimeParts {
  const [year, month, day, hour, minute, second] = localInput(instant).split(/[-T:]/);
  return { year, month, day, hour, minute, second };
}

export function segmentWidth(segment: Segment): number {
  return segment === 'year' ? 4 : 2;
}

export function segmentBounds(segment: Segment, parts: DateTimeParts): [number, number] {
  switch (segment) {
    case 'year':
      return [1, 9999];
    case 'month':
      return [1, 12];
    case 'day': {
      const year = Number(parts.year);
      const month = Number(parts.month);
      const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
      const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
      return [1, parts.year && year >= 1 && month >= 1 && month <= 12 ? days[month - 1] : 31];
    }
    case 'hour':
      return [0, 23];
    default:
      return [0, 59];
  }
}

export function partsProblem(parts: DateTimeParts, order: Segment[]): DateTimeProblem | null {
  for (const segment of order) {
    const value = parts[segment];
    if (!value) return { segment, message: `Enter a ${segment}.` };
    const [min, max] = segmentBounds(segment, parts);
    if (
      !/^\d+$/.test(value) ||
      value.length > segmentWidth(segment) ||
      Number(value) < min ||
      Number(value) > max
    )
      return { segment, message: `Enter a valid ${segment} (${min}–${max}).` };
  }
  return null;
}

/** Only complete calendar values may reach local-time/DST resolution. */
export function partsInput(parts: DateTimeParts): string {
  if (partsProblem(parts, ['year', 'month', 'day', ...timeSegments])) return '';
  const padded = (segment: Segment) => parts[segment].padStart(segmentWidth(segment), '0');
  return `${padded('year')}-${padded('month')}-${padded('day')}T${padded('hour')}:${padded('minute')}:${padded('second')}`;
}
