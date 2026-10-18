import { t } from "../i18n/core";
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
      return { label: t("settings.bridge.states.registered"), tone: "positive" };
    case "not_installed":
      return { label: t("settings.bridge.states.notInstalled"), tone: "neutral" };
    case "missing":
      return { label: t("settings.bridge.states.missing"), tone: "warning" };
    case "invalid":
      return { label: t("settings.bridge.states.invalid"), tone: "warning" };
    case "other_host":
      return { label: t("settings.bridge.states.otherHost"), tone: "warning" };
    case "extension_not_allowed":
      return { label: t("settings.bridge.states.extensionNotAllowed"), tone: "warning" };
    case "failed":
      return { label: t("settings.bridge.states.failed"), tone: "danger" };
  }
}

/** Whether registering now would change anything. */
export function canRegister(status: BridgeRegistration): boolean {
  return status.host_found && status.browsers.some((b) => b.state !== "registered" && b.state !== "not_installed");
}
