// Pausing capture for a while: every browser download stays in the browser
// until the time is up, without giving up the `downloads` permission (the
// capture switch removes it, and turning it back on asks the browser again).
// Kept in session storage, so a browser restart also ends the pause.

/** What the popup and the toolbar button's menu offer, in minutes. */
export const PAUSE_MINUTES = [10, 60] as const;

export type PauseMinutes = (typeof PAUSE_MINUTES)[number];

export function isPauseMinutes(value: unknown): value is PauseMinutes {
  return PAUSE_MINUTES.includes(value as PauseMinutes);
}

const LONGEST_MS = Math.max(...PAUSE_MINUTES) * 60_000;

/** When a pause started `now` for `minutes` ends. */
export function pauseEnd(now: number, minutes: PauseMinutes): number {
  return now + minutes * 60_000;
}

/**
 * The end of the pause in force at `now`, or undefined when capture is not
 * paused. Anything stored that could not have come from `pauseEnd` (further
 * ahead than the longest pause, say after the clock was turned back) counts
 * as no pause rather than one that never ends.
 */
export function pausedUntil(stored: unknown, now: number): number | undefined {
  if (typeof stored !== "number" || !Number.isFinite(stored)) return undefined;
  return stored > now && stored <= now + LONGEST_MS ? stored : undefined;
}
