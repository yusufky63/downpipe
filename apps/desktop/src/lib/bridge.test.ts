import { describe, expect, it } from "vitest";

import type { BridgeRegistration, BridgeState } from "./api";
import { bridgeStateInfo, canRegister } from "./bridge";

function status(states: BridgeState[], hostFound = true): BridgeRegistration {
  const browsers = (["chrome", "edge", "chromium", "firefox"] as const).map((browser, i) => ({
    browser,
    state: states[i] ?? "registered",
    detail: null,
  }));
  return { host: "C:/Downpipe/downpipe-native-host.exe", host_found: hostFound, browsers };
}

describe("bridge registration", () => {
  it("offers to register only when a present browser is off", () => {
    expect(canRegister(status([]))).toBe(false);
    expect(canRegister(status(["registered", "not_installed", "not_installed"]))).toBe(false);
    for (const state of ["missing", "invalid", "other_host", "extension_not_allowed", "failed"] as const) {
      expect(canRegister(status([state])), state).toBe(true);
    }
  });

  it("has nothing to register without the bundled host", () => {
    expect(canRegister(status(["missing", "missing"], false))).toBe(false);
  });

  it("labels every state", () => {
    expect(bridgeStateInfo("registered")).toEqual({ label: "Registered", tone: "positive" });
    expect(bridgeStateInfo("not_installed").tone).toBe("neutral");
    expect(bridgeStateInfo("other_host").tone).toBe("warning");
    expect(bridgeStateInfo("failed").tone).toBe("danger");
  });
});
