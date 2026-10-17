import { describe, expect, it } from "vitest";
import type { Activity } from "./messages";
import type { TaskStatus } from "./protocol";
import { isRunning, itemView } from "./view";

const sent: Activity = {
  at: 0,
  label: "16777216",
  source: "link",
  outcome: "added",
  message: "Added to Downpipe",
  taskId: "t1",
};

function task(extra: Partial<TaskStatus>): TaskStatus {
  return { taskId: "t1", state: "downloading", downloaded: 0, speed: 0, ...extra };
}

describe("itemView", () => {
  it("shows a running download as Downloading with progress and speed", () => {
    const view = itemView(
      sent,
      task({ fileName: "browser-bridge.bin", downloaded: 4 * 1048576, total: 16 * 1048576, speed: 2 * 1048576 }),
    );
    expect(view).toEqual({
      label: "browser-bridge.bin",
      badge: "Downloading",
      tone: "primary",
      detail: "25% · 4.0 MiB / 16.0 MiB · 2.0 MiB/s",
      progress: 25,
    });
  });

  it("leaves out what it does not know yet", () => {
    expect(itemView(sent, task({ state: "queued" }))).toEqual({
      label: "16777216",
      badge: "Queued",
      tone: "muted",
      detail: "0 B",
      progress: null,
    });
    expect(itemView(sent, task({ downloaded: 2048, speed: 0 })).detail).toBe("2.0 KiB");
  });

  it("finished, failed, paused and canceled states", () => {
    expect(itemView(sent, task({ state: "completed", downloaded: 1048576, total: 1048576 }))).toMatchObject({
      badge: "Done",
      tone: "positive",
      detail: "1.0 MiB",
      progress: null,
    });
    expect(itemView(sent, task({ state: "failed", error: "the server refused access" }))).toMatchObject({
      badge: "Failed",
      tone: "danger",
      detail: "The server refused access",
    });
    expect(itemView(sent, task({ state: "failed" })).detail).toBe("The download failed");
    expect(itemView(sent, task({ state: "paused", downloaded: 512, total: 1024 }))).toMatchObject({
      badge: "Paused",
      detail: "50% · 512 B / 1.0 KiB",
      progress: null,
    });
    expect(itemView(sent, task({ state: "canceled" })).badge).toBe("Canceled");
  });

  it("falls back to what happened when it was sent", () => {
    expect(itemView(sent, undefined)).toEqual({
      label: "16777216",
      badge: "Sent",
      tone: "muted",
      detail: "Added to Downpipe",
      progress: null,
    });
    const kept: Activity = { ...sent, outcome: "kept", message: "Downpipe was not running.", taskId: undefined };
    expect(itemView(kept, undefined)).toMatchObject({ badge: "In browser", tone: "warning" });
    const failed: Activity = { ...kept, outcome: "failed" };
    expect(itemView(failed, undefined)).toMatchObject({ badge: "Not sent", tone: "danger" });
  });

  it("knows which states are still running", () => {
    expect(["queued", "starting", "downloading", "waiting", "verifying"].every((s) => isRunning(s as never))).toBe(true);
    expect(["paused", "completed", "failed", "canceled"].some((s) => isRunning(s as never))).toBe(false);
  });
});
