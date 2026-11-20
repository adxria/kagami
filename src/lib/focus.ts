const breakPercentageKey = 'kagami.breakPercentage';
const defaultBreakPercentage = 20;

export function validBreakPercentage(value: number | undefined): value is number {
  return value !== undefined && Number.isInteger(value) && value >= 0 && value <= 100;
}

export function readBreakPercentage(): number {
  try {
    const stored = localStorage.getItem(breakPercentageKey);
    const value = stored?.trim() ? Number(stored) : undefined;
    if (validBreakPercentage(value)) return value;
  } catch {
    // Use the default if the webview cannot read preferences.
  }
  return defaultBreakPercentage;
}

export function writeBreakPercentage(value: number): void {
  localStorage.setItem(breakPercentageKey, String(value));
}

export function breakDuration(focusedMs: number, percentage: number): number {
  const breakSeconds = (Math.max(0, focusedMs) * percentage) / (100 * 1_000);
  return Math.floor(breakSeconds) * 1_000;
}
