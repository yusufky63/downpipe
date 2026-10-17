import type { BridgeBrowser, BridgeRegistration, BridgeState } from "./api";
import type { Tone } from "./tasks";

export const BROWSER_NAMES: Record<BridgeBrowser, string> = {
  chrome: "Chrome",
  edge: "Edge",
  chromium: "Chromium",
  firefox: "Firefox",
};

export function bridgeStateInfo(state: BridgeState): { label: string; tone: Tone } {
  switch (state) {
    case "registered":
      return { label: "Registered", tone: "positive" };
    case "not_installed":
      return { label: "Not installed", tone: "neutral" };
    case "missing":
      return { label: "Not registered", tone: "warning" };
    case "invalid":
      return { label: "Unreadable", tone: "warning" };
    case "other_host":
      return { label: "Other copy", tone: "warning" };
    case "extension_not_allowed":
      return { label: "Other extension", tone: "warning" };
    case "failed":
      return { label: "Error", tone: "danger" };
  }
}

/** Whether registering now would change anything. */
export function canRegister(status: BridgeRegistration): boolean {
  return status.host_found && status.browsers.some((b) => b.state !== "registered" && b.state !== "not_installed");
}
