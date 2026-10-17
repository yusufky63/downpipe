import type { BridgeFailure, HostResult } from "./host";

export type BridgeStatus =
  | { state: "connected"; appVersion: string; hostVersion: string }
  | { state: "app_not_running"; hostVersion: string }
  | { state: "host_missing" }
  | { state: "host_forbidden" }
  | { state: "error"; message: string };

const FAILURE_TEXT: Record<BridgeFailure, string> = {
  host_missing: "The Downpipe browser bridge is not installed.",
  host_forbidden: "The browser bridge is installed but does not allow this extension.",
  host_error: "The browser bridge stopped unexpectedly.",
  timeout: "Downpipe did not answer in time.",
  bad_response: "The browser bridge sent an answer this extension does not understand.",
};

// Error codes from crates/native-host/src/protocol.rs (`code`) and from the
// app's download queue.
const ERROR_TEXT: Record<string, string> = {
  app_not_running: "Downpipe is not running. Open the Downpipe app and try again.",
  app_unavailable: "Downpipe did not answer. Restart the app and try again.",
  duplicate: "Already in Downpipe.",
  invalid_url: "This link cannot be downloaded. Only http and https links work.",
  version_mismatch:
    "Downpipe and its browser bridge are different versions. Restart Downpipe so it registers its own bridge.",
  unsupported_version: "This extension is newer than the installed Downpipe. Update Downpipe.",
  message_too_large: "The link is too long to hand over.",
};

export function failureText(failure: BridgeFailure): string {
  return FAILURE_TEXT[failure];
}

/** Known codes get our wording; anything else falls back to the host's message. */
export function errorText(code: string, message: string): string {
  return ERROR_TEXT[code] ?? (message || `Downpipe refused the download (${code}).`);
}

export function bridgeStatus(result: HostResult): BridgeStatus {
  if (!result.ok) {
    if (result.failure === "host_missing") return { state: "host_missing" };
    if (result.failure === "host_forbidden") return { state: "host_forbidden" };
    return { state: "error", message: failureText(result.failure) };
  }
  const r = result.response;
  if (r.type === "pong") {
    return r.appRunning
      ? { state: "connected", appVersion: r.appVersion ?? "", hostVersion: r.hostVersion }
      : { state: "app_not_running", hostVersion: r.hostVersion };
  }
  if (r.type === "error") return { state: "error", message: errorText(r.code, r.message) };
  return { state: "error", message: FAILURE_TEXT.bad_response };
}

export type Outcome = "added" | "duplicate" | "failed";

/** What happened to one download, in words for the popup and the badge. */
export function downloadOutcome(result: HostResult): {
  outcome: Outcome;
  message: string;
  fileName?: string;
  taskId?: string;
} {
  if (!result.ok) return { outcome: "failed", message: failureText(result.failure) };
  const r = result.response;
  if (r.type === "accepted") {
    return { outcome: "added", message: "Added to Downpipe", fileName: r.fileName, taskId: r.taskId };
  }
  if (r.type === "error" && r.code === "duplicate") {
    return { outcome: "duplicate", message: ERROR_TEXT.duplicate ?? "", taskId: r.taskId };
  }
  if (r.type === "error") return { outcome: "failed", message: errorText(r.code, r.message) };
  return { outcome: "failed", message: FAILURE_TEXT.bad_response };
}

/** For a captured download the browser kept because Downpipe could not take it. */
export function keptMessage(result: HostResult): string {
  const closed = result.ok && result.response.type === "error" && result.response.code === "app_not_running";
  if (closed) return "Downpipe was not running, so the browser downloaded it.";
  return `${downloadOutcome(result).message} The browser downloaded it instead.`;
}
