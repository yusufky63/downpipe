import { describe, expect, it } from "vitest";
import {
  MAX_ACTIVITY,
  addActivity,
  parseActivity,
  parsePopupRequest,
  parseSettings,
  relativeTime,
  taskIdsOf,
  type Activity,
} from "./messages";

const entry = (at: number): Activity => ({ at, label: `f${at}.zip`, source: "link", outcome: "added", message: "Added" });

describe("parsePopupRequest", () => {
  it("accepts only its own requests, exactly", () => {
    expect(parsePopupRequest({ kind: "status" })).toEqual({ kind: "status" });
    expect(parsePopupRequest({ kind: "tasks" })).toEqual({ kind: "tasks" });
    expect(parsePopupRequest({ kind: "set_notify", enabled: false })).toEqual({ kind: "set_notify", enabled: false });
    expect(parsePopupRequest({ kind: "set_notify", enabled: "no" })).toBeNull();
    expect(parsePopupRequest({ kind: "set_notify", enabled: true, extra: 1 })).toBeNull();
    expect(parsePopupRequest({ kind: "tasks", task_ids: ["x"] })).toBeNull();
    expect(parsePopupRequest({ kind: "status", url: "https://x" })).toBeNull();
    expect(parsePopupRequest({ kind: "download", url: "https://x" })).toBeNull();
    expect(parsePopupRequest("status")).toBeNull();
    expect(parsePopupRequest(null)).toBeNull();
  });
});

describe("parseSettings", () => {
  it("notifies unless switched off", () => {
    expect(parseSettings(undefined)).toEqual({ notify: true });
    expect(parseSettings({ notify: false })).toEqual({ notify: false });
    expect(parseSettings({ notify: "off" })).toEqual({ notify: true });
  });
});

describe("activity", () => {
  it("keeps the newest few", () => {
    let list: Activity[] = [];
    for (let i = 0; i < MAX_ACTIVITY + 3; i++) list = addActivity(list, entry(i));
    expect(list).toHaveLength(MAX_ACTIVITY);
    expect(list[0]?.at).toBe(MAX_ACTIVITY + 2);
  });

  it("drops stored entries that do not have the right shape", () => {
    expect(parseActivity([entry(1), { at: "x" }, null, { ...entry(2), outcome: "hacked" }])).toEqual([entry(1)]);
    expect(parseActivity([{ ...entry(3), taskId: "t" }, { ...entry(4), taskId: 7 }])).toEqual([{ ...entry(3), taskId: "t" }]);
    expect(parseActivity(undefined)).toEqual([]);
  });

  it("collects the app's ids once each", () => {
    expect(taskIdsOf([{ ...entry(1), taskId: "a" }, entry(2), { ...entry(3), taskId: "a" }, { ...entry(4), taskId: "b" }])).toEqual([
      "a",
      "b",
    ]);
  });

  it("describes times briefly", () => {
    const now = Date.UTC(2026, 9, 17, 12);
    expect(relativeTime(now - 5_000, now)).toBe("just now");
    expect(relativeTime(now - 4 * 60_000, now)).toBe("4 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 3 * 86_400_000, now)).toBe("2026-10-14");
    expect(relativeTime(now + 10_000, now)).toBe("just now");
  });
});
