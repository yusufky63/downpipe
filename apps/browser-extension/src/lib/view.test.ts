import { describe, expect, it } from "vitest";
import type { Activity } from "./messages";
import type { TaskStatus } from "./protocol";
import { isRunning, itemView, mayChange, mediaEmptyText, mediaRowView, pageRowView } from "./view";

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

  it("shows a download held for confirmation as waiting, not paused", () => {
    expect(itemView(sent, task({ state: "paused", fileName: "asked.bin", awaitingConfirmation: true }))).toEqual({
      label: "asked.bin",
      badge: "Waiting in Downpipe",
      tone: "warning",
      detail: "Needs your answer",
      progress: null,
    });
    // Once answered with "Download later" it is an ordinary paused download.
    expect(itemView(sent, task({ state: "paused" })).badge).toBe("Paused");
    // The flag only means something next to `paused`.
    expect(itemView(sent, task({ state: "canceled", awaitingConfirmation: true })).badge).toBe("Canceled");
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

  it("keeps asking about running and held downloads only", () => {
    expect(mayChange(task({ state: "downloading" }))).toBe(true);
    expect(mayChange(task({ state: "paused", awaitingConfirmation: true }))).toBe(true);
    expect(mayChange(task({ state: "paused" }))).toBe(false);
    expect(mayChange(task({ state: "completed" }))).toBe(false);
  });
});

describe("mediaRowView", () => {
  const stream = { id: "a", kind: "hls" as const, url: "https://video.example.net/v/master.m3u8", at: 1 };
  const file = { id: "b", kind: "direct" as const, url: "https://cdn.example.net/clip.mp4", mime: "video/mp4", size: 2048, at: 1 };

  it("names the media and where it comes from", () => {
    expect(mediaRowView(stream, true)).toEqual({
      label: "master.m3u8",
      detail: "HLS stream · video.example.net",
      sendable: true,
    });
    expect(mediaRowView(file, true).detail).toBe("MP4 video · 2.0 KiB · cdn.example.net");
  });

  it("takes the page's name and says how many qualities a master has", () => {
    expect(mediaRowView({ ...stream, qualities: 3 }, true, "Home / X (2)")).toEqual({
      label: "Home / X (2)",
      detail: "HLS · 3 qualities · video.example.net",
      sendable: true,
    });
  });

  it("asks for an update when Downpipe cannot take streams", () => {
    expect(mediaRowView(stream, false)).toMatchObject({ sendable: false, detail: "Update Downpipe to download streams" });
    // Files go through the ordinary download message, which every version knows.
    expect(mediaRowView(file, false).sendable).toBe(true);
  });

  it("offers a video page by its title", () => {
    expect(pageRowView({ site: "YouTube", title: "Me at the zoo" }, true)).toEqual({
      label: "Me at the zoo",
      detail: "YouTube video",
      sendable: true,
    });
    expect(pageRowView({ site: "YouTube" }, true).label).toBe("This YouTube video");
    expect(pageRowView({ site: "YouTube" }, false)).toMatchObject({
      sendable: false,
      detail: "Update Downpipe to download videos from pages",
    });
  });

  it("explains an empty list", () => {
    expect(mediaEmptyText(false)).toContain("access to all sites");
    expect(mediaEmptyText(true)).toContain("reload the page");
    expect(mediaEmptyText(true, "*.example.org")).toBe("Not listed here: *.example.org is an excluded site in Filters.");
  });
});
