import { describe, expect, it } from "vitest";
import type { HostResult } from "./host";
import { bridgeStatus, downloadOutcome, errorText, keptMessage } from "./status";
import { statusView } from "./view";

const ok = (response: Extract<HostResult, { ok: true }>["response"]): HostResult => ({ ok: true, response });
const fail = (failure: Extract<HostResult, { ok: false }>["failure"]): HostResult => ({
  ok: false,
  failure,
  detail: "browser said no",
});

describe("bridgeStatus", () => {
  it("tells connected, app closed and missing host apart", () => {
    expect(bridgeStatus(ok({ type: "pong", hostVersion: "0.2.0", appRunning: true, appVersion: "0.2.1" }))).toEqual({
      state: "connected",
      appVersion: "0.2.1",
      hostVersion: "0.2.0",
    });
    expect(bridgeStatus(ok({ type: "pong", hostVersion: "0.2.0", appRunning: false }))).toEqual({
      state: "app_not_running",
      hostVersion: "0.2.0",
    });
    expect(bridgeStatus(fail("host_missing"))).toEqual({ state: "host_missing" });
    expect(bridgeStatus(fail("host_forbidden"))).toEqual({ state: "host_forbidden" });
    expect(bridgeStatus(fail("timeout"))).toEqual({ state: "error", message: "Downpipe did not answer in time." });
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

  it("falls back to the host's message for codes it does not know", () => {
    expect(errorText("forbidden", "The server refused access")).toBe("The server refused access");
    expect(errorText("weird", "")).toBe("Downpipe refused the download (weird).");
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
    expect(statusView({ state: "connected", appVersion: "1", hostVersion: "1" }, chromeExt).tone).toBe("positive");
    expect(statusView({ state: "app_not_running", hostVersion: "1" }, chromeExt).tone).toBe("warning");
  });
});
