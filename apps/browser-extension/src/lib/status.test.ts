import { describe, expect, it } from "vitest";
import type { HostResult } from "./host";
import {
  bridgeDown,
  bridgeStatus,
  downloadOutcome,
  errorText,
  keptMessage,
  pageOutcome,
  streamOutcome,
} from "./status";
import { statusView } from "./view";

const ok = (response: Extract<HostResult, { ok: true }>["response"]): HostResult => ({ ok: true, response });
const fail = (failure: Extract<HostResult, { ok: false }>["failure"]): HostResult => ({
  ok: false,
  failure,
  detail: "browser said no",
});

describe("bridgeStatus", () => {
  it("tells connected, app closed and missing host apart", () => {
    expect(
      bridgeStatus(ok({ type: "pong", hostVersion: "0.2.0", appRunning: true, appVersion: "0.2.1", features: [] })),
    ).toEqual({
      state: "connected",
      appVersion: "0.2.1",
      hostVersion: "0.2.0",
      media: false,
      pages: false,
    });
    expect(bridgeStatus(ok({ type: "pong", hostVersion: "0.2.0", appRunning: false, features: [] }))).toEqual({
      state: "app_not_running",
      hostVersion: "0.2.0",
    });
    expect(bridgeStatus(fail("host_missing"))).toEqual({ state: "host_missing" });
    expect(bridgeStatus(fail("host_forbidden"))).toEqual({ state: "host_forbidden" });
    expect(bridgeStatus(fail("timeout"))).toEqual({ state: "error", message: "Downpipe did not answer in time." });
  });

  it("knows whether streams can be sent", () => {
    const pong = ok({ type: "pong", hostVersion: "0.4.0", appRunning: true, appVersion: "0.4.0", features: ["media"] });
    expect(bridgeStatus(pong)).toMatchObject({ state: "connected", media: true, pages: false });
  });

  it("knows whether video pages can be sent", () => {
    const pong = ok({ type: "pong", hostVersion: "0.4.0", appRunning: true, appVersion: "0.4.0", features: ["media", "page"] });
    expect(bridgeStatus(pong)).toMatchObject({ state: "connected", media: true, pages: true });
  });

  it("explains app errors in its own words", () => {
    expect(bridgeStatus(ok({ type: "error", code: "version_mismatch", message: "raw" }))).toEqual({
      state: "error",
      message: errorText("version_mismatch", "raw"),
    });
  });
});

describe("downloadOutcome", () => {
  it("added, duplicate, failed", () => {
    expect(downloadOutcome(ok({ type: "accepted", taskId: "t", fileName: "a.zip" }))).toEqual({
      outcome: "added",
      message: "Added to Downpipe",
      fileName: "a.zip",
      taskId: "t",
    });
    expect(downloadOutcome(ok({ type: "error", code: "duplicate", message: "x", taskId: "t" }))).toMatchObject({
      outcome: "duplicate",
      taskId: "t",
    });
    expect(downloadOutcome(ok({ type: "error", code: "app_not_running", message: "x" }))).toEqual({
      outcome: "failed",
      message: "Downpipe is not running. Open the Downpipe app and try again.",
    });
    expect(downloadOutcome(fail("host_error")).outcome).toBe("failed");
  });

  it("says when the app holds the download for the user's answer", () => {
    expect(downloadOutcome(ok({ type: "accepted", taskId: "t", fileName: "a.zip", awaitingConfirmation: true }))).toEqual({
      outcome: "added",
      message: "Waiting for your answer in Downpipe",
      fileName: "a.zip",
      taskId: "t",
      awaitingConfirmation: true,
    });
  });

  it("falls back to the host's message for codes it does not know", () => {
    expect(errorText("forbidden", "The server refused access")).toBe("The server refused access");
    expect(errorText("weird", "")).toBe("Downpipe refused the download (weird).");
  });
});

describe("bridgeDown", () => {
  it("stops a batch when the bridge or the app is the problem", () => {
    for (const failure of ["host_missing", "host_forbidden", "host_error", "timeout", "bad_response"] as const) {
      expect(bridgeDown(fail(failure)), failure).toBe(true);
    }
    for (const code of ["app_not_running", "app_unavailable", "version_mismatch", "unsupported_version"]) {
      expect(bridgeDown(ok({ type: "error", code, message: "x" })), code).toBe(true);
    }
  });

  it("keeps going when only one link was refused", () => {
    expect(bridgeDown(ok({ type: "accepted", taskId: "t" }))).toBe(false);
    expect(bridgeDown(ok({ type: "error", code: "duplicate", message: "x", taskId: "t" }))).toBe(false);
    expect(bridgeDown(ok({ type: "error", code: "invalid_url", message: "x" }))).toBe(false);
    expect(bridgeDown(ok({ type: "error", code: "invalid_header", message: "x" }))).toBe(false);
  });
});

describe("keptMessage", () => {
  it("says plainly that the browser kept the file", () => {
    expect(keptMessage(ok({ type: "error", code: "app_not_running", message: "x" }))).toBe(
      "Downpipe was not running, so the browser downloaded it.",
    );
    expect(keptMessage(fail("timeout"))).toBe("Downpipe did not answer in time. The browser downloaded it instead.");
  });
});

describe("statusView", () => {
  const chromeExt = { id: "efgicfboaldflckpncfdojkeoaamflpf", firefox: false };

  it("shows the install command when the host is missing", () => {
    const view = statusView({ state: "host_missing" }, chromeExt);
    expect(view.tone).toBe("danger");
    expect(view.command).toBe("downpipe-native-host install");
    expect(view.guide).toBe(true);
  });

  it("names this extension's id when the host does not allow it", () => {
    expect(statusView({ state: "host_forbidden" }, chromeExt).command).toBe(
      "downpipe-native-host install --extension-id efgicfboaldflckpncfdojkeoaamflpf",
    );
    expect(statusView({ state: "host_forbidden" }, { id: "downpipe@downpipe.dev", firefox: true }).command).toBe(
      "downpipe-native-host install --firefox-extension-id downpipe@downpipe.dev",
    );
  });

  it("uses calm tones for working states", () => {
    expect(statusView(null, chromeExt).tone).toBe("muted");
    expect(statusView({ state: "connected", appVersion: "1", hostVersion: "1", media: true, pages: true }, chromeExt).tone).toBe(
      "positive",
    );
    expect(statusView({ state: "app_not_running", hostVersion: "1" }, chromeExt).tone).toBe("warning");
  });

  it("says a closed Downpipe starts with the next download when the bridge may start it", () => {
    const closed = { state: "app_not_running", hostVersion: "1" } as const;
    expect(statusView(closed, chromeExt).detail).toContain("not queued");
    const starts = statusView(closed, chromeExt, true);
    expect(starts.tone).toBe("muted");
    expect(starts.detail).toBe("A download you send starts it in the tray.");
  });
});

describe("streamOutcome", () => {
  it("tells an outdated Downpipe from a broken stream", () => {
    const old = ok({ type: "error", code: "invalid_message", message: "unknown message type" });
    expect(streamOutcome(old)).toEqual({ outcome: "failed", message: "This version of Downpipe cannot download streams. Update Downpipe." });
    const taken = ok({ type: "accepted", taskId: "t", fileName: "Clip.mp4", awaitingConfirmation: true });
    expect(streamOutcome(taken)).toMatchObject({ outcome: "added", taskId: "t", awaitingConfirmation: true });
  });
});

describe("pageOutcome", () => {
  it("tells an outdated Downpipe from a refused page", () => {
    const old = ok({ type: "error", code: "invalid_message", message: "unknown variant `page`" });
    expect(pageOutcome(old)).toEqual({ outcome: "failed", message: "This version of Downpipe cannot download videos from pages. Update Downpipe." });
    const other = ok({ type: "error", code: "unsupported_page", message: "raw" });
    expect(pageOutcome(other)).toEqual({ outcome: "failed", message: "Downpipe does not download videos from this site." });
    const held = ok({ type: "accepted", taskId: "y", fileName: "Me at the zoo.mp4", awaitingConfirmation: true });
    expect(pageOutcome(held)).toMatchObject({ outcome: "added", taskId: "y", fileName: "Me at the zoo.mp4" });
  });
});
