import type { Context } from './types';
import paletteSource from '../../shared/context-palette.txt?raw';

export const contextPalette = paletteSource
  .trim()
  .split('\n')
  .map((line) => {
    const [name, color] = line.trim().split(/\s+/);
    return { name, color };
  });

export const unassignedColor = '#ACB2C3';

export function normalizeColor(value: string): string | null {
  const match = /^#?([\da-f]{3}|[\da-f]{6})$/i.exec(value.trim());
  if (!match) return null;
  const hex = match[1].length === 3 ? [...match[1]].map((c) => c + c).join('') : match[1];
  return `#${hex.toUpperCase()}`;
}

export function suggestContextColor(contexts: Context[], parentId: string | null = null): string {
  const parent = contexts.find((context) => context.id === parentId);
  if (parent) return parent.color;
  return [...contextPalette].sort(
    (a, b) =>
      contexts.filter((context) => !context.parentId && context.color === a.color).length -
      contexts.filter((context) => !context.parentId && context.color === b.color).length,
  )[0].color;
}

export function subtreeIds(contexts: Context[], id: string): Set<string> {
  const ids = new Set([id]);
  for (const parent of ids) {
    for (const context of contexts) if (context.parentId === parent) ids.add(context.id);
  }
  return ids;
}

export interface ContextRow extends Context {
  depth: number;
  path: string;
}

export function contextRows(contexts: Context[]): ContextRow[] {
  const result: ContextRow[] = [];
  const visited = new Set<string>();
  function visit(context: Context, depth: number, prefix: string) {
    if (visited.has(context.id)) return;
    visited.add(context.id);
    const path = prefix ? `${prefix} / ${context.name}` : context.name;
    result.push({ ...context, depth, path });
    contexts
      .filter((child) => child.parentId === context.id)
      .forEach((child) => visit(child, depth + 1, path));
  }
  contexts.filter((context) => !context.parentId).forEach((context) => visit(context, 0, ''));
  contexts
    .filter((context) => !visited.has(context.id))
    .forEach((context) => visit(context, 0, ''));
  return result;
}

export function contextPath(contexts: Context[], id: string | null): string {
  return contextRows(contexts).find((context) => context.id === id)?.path ?? 'Unassigned';
}

export function contextColor(id: string | null, contexts: Context[]): string {
  return contexts.find((context) => context.id === id)?.color ?? unassignedColor;
}

const styles = new Map<string, string>();
export function colorStyle(color: string): string {
  color = normalizeColor(color) ?? unassignedColor;
  const cached = styles.get(color);
  if (cached) return cached;
  const rgb = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));
  const luminance = (channels: number[]) =>
    channels.reduce((sum, channel, i) => {
      const c = channel / 255;
      return (
        sum +
        (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][i]
      );
    }, 0);
  const contrast = (a: number, b: number) => (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  const ink = '#121A24';
  const paper = '#EEF0F2';
  const foregroundFor = (channels: number[]) => {
    const background = luminance(channels);
    const inkContrast = contrast(background, luminance([18, 26, 36]));
    const paperContrast = contrast(background, luminance([238, 240, 242]));
    if (Math.max(inkContrast, paperContrast) < 4.5)
      return contrast(background, 0) >= contrast(background, 1) ? '#000000' : '#FFFFFF';
    return inkContrast >= paperContrast ? ink : paper;
  };
  const toHex = (channels: number[]) =>
    '#' + channels.map((c) => c.toString(16).padStart(2, '0')).join('');
  // A medium neutral base gives entries a solid, subdued material color rather
  // than a dark wash or a large patch of the full-strength identification color.
  const fill = rgb.map((c, i) => Math.round(c * 0.24 + [68, 71, 80][i] * 0.76));
  // Small indicators still need to be visible on the surrounding dark chrome.
  const chrome = luminance([44, 45, 49]);
  let indicator = rgb;
  for (let amount = 0; amount <= 100; amount++) {
    indicator = rgb.map((c) => Math.round(c + ((255 - c) * amount) / 100));
    if ((luminance(indicator) + 0.05) / (chrome + 0.05) >= 3) break;
  }
  const style = `--context-color:${color};--context-indicator:${toHex(indicator)};--context-fill:${toHex(fill)};--context-on-fill:${foregroundFor(fill)}`;
  if (styles.size > 256) styles.clear();
  styles.set(color, style);
  return style;
}
