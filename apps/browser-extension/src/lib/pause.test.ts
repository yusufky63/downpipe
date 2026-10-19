import { describe, expect, it } from "vitest";
import { PAUSE_MINUTES, isPauseMinutes, pauseEnd, pausedUntil } from "./pause";

const NOW = 1_800_000_000_000;

describe("pausing capture", () => {
  it("offers only its own lengths", () => {
    expect(PAUSE_MINUTES.every(isPauseMinutes)).toBe(true);
    expect(isPauseMinutes(5)).toBe(false);
    expect(isPauseMinutes("10")).toBe(false);
  });

  it("is in force until its end", () => {
    const end = pauseEnd(NOW, 10);
    expect(end).toBe(NOW + 600_000);
    expect(pausedUntil(end, NOW)).toBe(end);
    expect(pausedUntil(end, end - 1)).toBe(end);
    expect(pausedUntil(end, end)).toBeUndefined();
  });

  it("treats anything else stored as no pause", () => {
    expect(pausedUntil(undefined, NOW)).toBeUndefined();
    expect(pausedUntil("soon", NOW)).toBeUndefined();
    expect(pausedUntil(Number.POSITIVE_INFINITY, NOW)).toBeUndefined();
    expect(pausedUntil(NOW - 1, NOW)).toBeUndefined();
    // Further ahead than any pause, say after the clock was turned back.
    expect(pausedUntil(pauseEnd(NOW, 60) + 1, NOW)).toBeUndefined();
  });
});
