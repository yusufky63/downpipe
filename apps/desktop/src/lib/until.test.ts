import { describe, expect, it } from "vitest";

import { makeTask as task } from "./testing";
import { canPauseUntil, clockText, fromLocalInput, nextClockTime, presetTime, resumesText, toLocalInput, unixSeconds } from "./until";

describe("pause until", () => {
  const evening = new Date(2026, 9, 18, 21, 15, 30);

  it("finds the next time the clock shows a time", () => {
    expect(nextClockTime(evening, 2, 0)).toEqual(new Date(2026, 9, 19, 2, 0, 0));
    expect(nextClockTime(new Date(2026, 9, 19, 1, 0), 2, 0)).toEqual(new Date(2026, 9, 19, 2, 0, 0));
    // Exactly then is too late: the next day.
    expect(nextClockTime(new Date(2026, 9, 19, 2, 0), 2, 0)).toEqual(new Date(2026, 9, 20, 2, 0, 0));
  });

  it("turns the presets into times", () => {
    expect(presetTime("30m", evening).getTime() - evening.getTime()).toBe(30 * 60_000);
    expect(presetTime("1h", evening).getTime() - evening.getTime()).toBe(60 * 60_000);
    expect(presetTime("tonight", evening)).toEqual(new Date(2026, 9, 19, 2, 0, 0));
    expect(unixSeconds(new Date(1_500))).toBe(1);
  });

  it("reads and writes the date-time field in local time", () => {
    expect(toLocalInput(evening)).toBe("2026-10-18T21:15");
    expect(fromLocalInput("2026-10-18T21:15")).toEqual(new Date(2026, 9, 18, 21, 15, 0));
    expect(fromLocalInput("")).toBeNull();
    expect(fromLocalInput("2026-02-31T10:00")).toBeNull();
    expect(fromLocalInput("tomorrow")).toBeNull();
  });

  it("offers it for what can still wait", () => {
    expect(canPauseUntil(task("a", { state: "downloading" }))).toBe(true);
    expect(canPauseUntil(task("a", { state: "queued" }))).toBe(true);
    expect(canPauseUntil(task("a", { state: "paused" }))).toBe(true);
    expect(canPauseUntil(task("a", { state: "failed" }))).toBe(true);
    expect(canPauseUntil(task("a", { state: "completed" }))).toBe(false);
    expect(canPauseUntil(task("a", { state: "canceled" }))).toBe(false);
    expect(canPauseUntil(task("a", { state: "paused", awaiting_confirmation: true }))).toBe(false);
  });

  it("says when a paused download resumes", () => {
    const at = unixSeconds(new Date(2026, 9, 19, 2, 0));
    const text = resumesText(task("a", { state: "paused", resume_at: at }), evening);
    expect(text).toMatch(/^Resumes at /);
    expect(text).toContain(clockText(at, evening));
    expect(resumesText(task("a", { state: "paused" }), evening)).toBeNull();
    expect(resumesText(task("a", { state: "downloading", resume_at: at }), evening)).toBeNull();
  });

  it("says a time as short as it can", () => {
    const today = unixSeconds(new Date(2026, 9, 18, 23, 30));
    const tomorrow = unixSeconds(new Date(2026, 9, 19, 2, 0));
    const later = unixSeconds(new Date(2026, 10, 30, 2, 0));
    expect(clockText(today, evening)).toMatch(/^\d{2}:\d{2}/);
    expect(clockText(tomorrow, evening)).not.toBe(clockText(today, evening));
    expect(clockText(later, evening)).toMatch(/30|Nov/);
  });
});
