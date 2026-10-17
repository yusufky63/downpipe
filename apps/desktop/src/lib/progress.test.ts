import { describe, expect, it } from "vitest";

import { progressEyebrow, progressId, progressPhase, resumeText, sizeText, speedText, statusText, windowTitle } from "./progress";
import { makeTask as task } from "./testing";

const MIB = 1024 * 1024;

describe("progress window", () => {
  const running = task("a", {
    file_name: "report.pdf",
    state: "downloading",
    total: 50 * MIB,
    downloaded: 12.5 * MIB,
    speed: 2 * MIB,
    eta_secs: 19,
    connections: 4,
    resumable: true,
  });

  it("reads its download from the window's label", () => {
    expect(progressId("progress-68b6ffff-1a2b")).toBe("68b6ffff-1a2b");
    expect(progressId("progress-")).toBeNull();
    expect(progressId("confirm")).toBeNull();
  });

  it("follows the download's state", () => {
    expect(progressPhase(running)).toBe("active");
    expect(progressPhase(task("b", { state: "queued" }))).toBe("active");
    expect(progressPhase(task("b", { state: "paused" }))).toBe("paused");
    expect(progressPhase(task("b", { state: "completed" }))).toBe("done");
    expect(progressPhase(task("b", { state: "failed" }))).toBe("failed");
    expect(progressPhase(task("b", { state: "canceled" }))).toBe("canceled");
    expect(progressPhase(null)).toBe("removed");
  });

  it("says what happens, IDM's way", () => {
    expect(progressEyebrow(running)).toBe("Downloading");
    expect(progressEyebrow(task("b", { state: "queued", media: { kind: "hls" } }))).toBe("Queued · HLS stream");
    expect(progressEyebrow(task("b", { state: "completed", site: { site: "YouTube" } }))).toBe("Download complete · YouTube");
    expect(statusText(running)).toBe("Receiving data · 4 connections");
    expect(statusText(task("b", { state: "downloading", fragments: { done: 3, total: 40 } }))).toBe("Receiving data · 3 of 40 fragments");
    expect(statusText(task("b", { state: "failed", error: "the server answered 403" }))).toBe("The server answered 403");
    expect(resumeText(running)).toBe("Yes");
    expect(resumeText(task("b", { state: "downloading", resumable: false }))).toBe("No: pausing starts it over");
    expect(resumeText(task("b", { state: "queued" }))).toBeNull();
  });

  it("shows sizes, speed and time left", () => {
    expect(sizeText(running)).toBe("12.5 MiB of 50.0 MiB · 25.0%");
    expect(sizeText(task("b", { downloaded: 1536, total: null }))).toBe("1.5 KiB");
    expect(sizeText(task("b", { state: "downloading", downloaded: 10 * MIB, total: 40 * MIB, media: { kind: "dash" } }))).toBe(
      "10.0 MiB of ~40.0 MiB · 25.0%",
    );
    expect(sizeText(task("b", { state: "completed", downloaded: 50 * MIB, total: 50 * MIB }))).toBe("50.0 MiB");
    expect(speedText(running)).toBe("2.0 MiB/s · 19s left");
    expect(speedText(task("b", { state: "downloading", speed: 0, eta_secs: null }))).toBe("–");
    expect(speedText(task("b", { state: "paused" }))).toBeNull();
  });

  it("puts the progress in the window's title", () => {
    expect(windowTitle(running)).toBe("25% report.pdf - Downpipe");
    expect(windowTitle({ ...running, total: null })).toBe("report.pdf - Downpipe");
    expect(windowTitle({ ...running, state: "paused" })).toBe("Paused · report.pdf - Downpipe");
    expect(windowTitle({ ...running, state: "completed" })).toBe("Done · report.pdf - Downpipe");
    expect(windowTitle(null)).toBe("Removed - Downpipe");
  });
});
