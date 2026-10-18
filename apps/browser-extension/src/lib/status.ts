import { t } from "../i18n";
import type { BridgeFailure, HostResult } from "./host";

export type BridgeStatus =
  /**
   * `media`: both the bridge and the app can take streams (HLS, DASH).
   * `pages`: they can take video pages for the site provider (YouTube).
   * `lists`: they can open playlists and channels to choose videos from.
   */
  | { state: "connected"; appVersion: string; hostVersion: string; media: boolean; pages: boolean; lists: boolean }
  | { state: "app_not_running"; hostVersion: string }
  | { state: "host_missing" }
  | { state: "host_forbidden" }
  | { state: "error"; message: string };

// Error codes from crates/native-host/src/protocol.rs (`code`) and from the
// app's download queue that the extension words itself.
const ERROR_CODES = [
  "app_not_running",
  "app_unavailable",
  "duplicate",
  "invalid_url",
  "invalid_message",
  "cookies_not_accepted",
  "version_mismatch",
  "unsupported_version",
  "message_too_large",
  "unsupported_page",
  "internal",
] as const;

/**
 * The engine's failure kinds (`DownloadError::kind()` in
 * crates/http-engine/src/error.rs), as the app reports them in `tasks`
 * answers and as error codes when it refuses a download.
 */
export const FAILURE_KINDS = [
  "invalid_header",
  "auth_required",
  "forbidden",
  "not_found",
  "throttled",
  "http_status",
  "resource_changed",
  "different_file",
  "expired_link",
  "login_required",
  "range_not_honoured",
  "network",
  "checksum",
  "io",
  "corrupt_meta",
  "drm_protected",
  "live_stream",
  "invalid_manifest",
  "unsupported_media",
  "tool_missing",
  "unavailable",
  "extractor",
] as const;

type FailureKind = (typeof FAILURE_KINDS)[number];

const isFailureKind = (kind: string): kind is FailureKind => (FAILURE_KINDS as readonly string[]).includes(kind);

/**
 * A failure kind in the browser's language; undefined for one this version
 * does not know. `invalid_url` is a bridge code and words itself there.
 */
export function kindText(kind: string | undefined, raw: string | undefined): string | undefined {
  if (kind === "invalid_url") return t("errors.code.invalid_url");
  if (kind === undefined || !isFailureKind(kind)) return undefined;
  if (kind === "http_status") {
    const status = raw?.match(/\b([1-5]\d\d)\b/)?.[1];
    // A string, so the status is not written with a thousands separator.
    return status ? t("errors.kind.http_status", { status }) : t("errors.serverError");
  }
  return t(`errors.kind.${kind}`);
}

type KnownCode = (typeof ERROR_CODES)[number];

const isKnownCode = (code: string): code is KnownCode => (ERROR_CODES as readonly string[]).includes(code);

export function failureText(failure: BridgeFailure): string {
  return t(`errors.bridge.${failure}`);
}

/** Known codes get our wording; anything else falls back to the host's message. */
export function errorText(code: string, message: string): string {
  if (isKnownCode(code)) return t(`errors.code.${code}`);
  return kindText(code, message) ?? (message || t("errors.refused", { code }));
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
      ? {
          state: "connected",
          appVersion: r.appVersion ?? "",
          hostVersion: r.hostVersion,
          media: r.features.includes("media"),
          pages: r.features.includes("page"),
          lists: r.features.includes("list"),
        }
      : { state: "app_not_running", hostVersion: r.hostVersion };
  }
  if (r.type === "error") return { state: "error", message: errorText(r.code, r.message) };
  return { state: "error", message: failureText("bad_response") };
}

export type Outcome = "added" | "duplicate" | "failed";

/** What happened to one download, in words for the popup and the badge. */
export interface HandOver {
  outcome: Outcome;
  message: string;
  fileName?: string;
  taskId?: string;
  /** Taken, but held in the app's confirmation dialog rather than started. */
  awaitingConfirmation?: boolean;
}

export function downloadOutcome(result: HostResult): HandOver {
  if (!result.ok) return { outcome: "failed", message: failureText(result.failure) };
  const r = result.response;
  if (r.type === "accepted" && r.awaitingConfirmation) {
    return { outcome: "added", message: t("results.waiting"), fileName: r.fileName, taskId: r.taskId, awaitingConfirmation: true };
  }
  if (r.type === "accepted") {
    return { outcome: "added", message: t("results.added"), fileName: r.fileName, taskId: r.taskId };
  }
  // A playlist: nothing is added until its videos are chosen in the app.
  if (r.type === "list_opened") return { outcome: "added", message: t("results.listOpened") };
  if (r.type === "error" && r.code === "duplicate") {
    return { outcome: "duplicate", message: t("errors.code.duplicate"), taskId: r.taskId };
  }
  if (r.type === "error") return { outcome: "failed", message: errorText(r.code, r.message) };
  return { outcome: "failed", message: failureText("bad_response") };
}

/**
 * Like `downloadOutcome`, for a `media` message. An app or bridge from
 * before streams were supported refuses the message type.
 */
export function streamOutcome(result: HostResult): HandOver {
  if (result.ok && result.response.type === "error" && result.response.code === "invalid_message") {
    return { outcome: "failed", message: t("errors.streamsUnsupported") };
  }
  return downloadOutcome(result);
}

/**
 * Like `downloadOutcome`, for a `page` message. An app or bridge from
 * before the site provider refuses the message type.
 */
export function pageOutcome(result: HostResult): HandOver {
  if (result.ok && result.response.type === "error" && result.response.code === "invalid_message") {
    return { outcome: "failed", message: t("errors.pagesUnsupported") };
  }
  return downloadOutcome(result);
}

// Codes after which the next link would fail the same way.
const BRIDGE_DOWN_CODES = new Set(["app_not_running", "app_unavailable", "version_mismatch", "unsupported_version"]);

/**
 * True when the bridge or the app itself is the problem, not this one link.
 * Sending a page's links stops there instead of failing each of them,
 * possibly after a timeout each.
 */
export function bridgeDown(result: HostResult): boolean {
  if (!result.ok) return true;
  return result.response.type === "error" && BRIDGE_DOWN_CODES.has(result.response.code);
}

/** For a captured download the browser kept because Downpipe could not take it. */
export function keptMessage(result: HostResult): string {
  const closed = result.ok && result.response.type === "error" && result.response.code === "app_not_running";
  if (closed) return t("errors.keptClosed");
  return t("errors.keptInstead", { reason: downloadOutcome(result).message });
}
