// What the extension's own pages (popup, options, links page) and the
// service worker say to each other.
import { parseFilters, normalizeSitePattern, validFilters, type CaptureFilters } from "./filters";
import type { TaskStatus } from "./protocol";
import type { BridgeStatus, Outcome } from "./status";
import { webUrl } from "./url";

export type PopupRequest =
  | { kind: "status" }
  | { kind: "tasks" }
  | { kind: "ping" }
  | { kind: "get_settings" }
  | { kind: "set_notify"; enabled: boolean }
  | { kind: "save_filters"; filters: CaptureFilters }
  /** Adds or removes one exact entry of the excluded sites. */
  | { kind: "set_site_paused"; site: string; paused: boolean }
  /** Collect the links of this tab and open the links page; needs `activeTab` for it. */
  | { kind: "collect_links"; tabId: number }
  /** One link from the links page, sent as if it had been clicked on `pageUrl`. */
  | { kind: "send_link"; url: string; pageUrl?: string };

const SIMPLE_KINDS = ["status", "tasks", "ping", "get_settings"] as const;

function hasKeys(v: Record<string, unknown>, ...keys: string[]): boolean {
  const own = Object.keys(v);
  return own.length === keys.length && keys.every((k) => own.includes(k));
}

function isWebUrl(value: unknown): value is string {
  return typeof value === "string" && webUrl(value) !== null;
}

/**
 * Exact shapes only: an unknown kind, a missing or extra field, or a value
 * of the wrong type makes the whole request invalid.
 */
export function parsePopupRequest(value: unknown): PopupRequest | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  const simple = SIMPLE_KINDS.find((k) => k === v.kind);
  if (simple) return hasKeys(v, "kind") ? { kind: simple } : null;
  switch (v.kind) {
    case "set_notify":
      return hasKeys(v, "kind", "enabled") && typeof v.enabled === "boolean"
        ? { kind: "set_notify", enabled: v.enabled }
        : null;
    case "save_filters": {
      const filters = hasKeys(v, "kind", "filters") ? validFilters(v.filters) : null;
      return filters ? { kind: "save_filters", filters } : null;
    }
    case "set_site_paused": {
      if (!hasKeys(v, "kind", "site", "paused") || typeof v.site !== "string" || typeof v.paused !== "boolean") {
        return null;
      }
      return normalizeSitePattern(v.site) === v.site ? { kind: "set_site_paused", site: v.site, paused: v.paused } : null;
    }
    case "collect_links":
      return hasKeys(v, "kind", "tabId") && Number.isSafeInteger(v.tabId) && (v.tabId as number) >= 0
        ? { kind: "collect_links", tabId: v.tabId as number }
        : null;
    case "send_link": {
      if (!isWebUrl(v.url)) return null;
      if (hasKeys(v, "kind", "url")) return { kind: "send_link", url: v.url };
      return hasKeys(v, "kind", "url", "pageUrl") && isWebUrl(v.pageUrl)
        ? { kind: "send_link", url: v.url, pageUrl: v.pageUrl }
        : null;
    }
    default:
      return null;
  }
}

/** Stored in `storage.local` under one key. */
export interface Settings extends CaptureFilters {
  /** Show a notification when a download is handed over, or could not be. */
  notify: boolean;
}

/**
 * Anything stored, including settings from before the capture filters
 * existed (`{ notify }` only): missing or malformed fields get defaults.
 */
export function parseSettings(value: unknown): Settings {
  const v = typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  return { notify: v.notify !== false, ...parseFilters(v) };
}

export function isSettings(value: unknown): value is Settings {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.notify === "boolean" && validFilters({
    captureAll: v.captureAll,
    captureTypes: v.captureTypes,
    excludedSites: v.excludedSites,
    minSizeBytes: v.minSizeBytes,
  }) !== null;
}

/** The answer to `send_link`. */
export interface LinkResult {
  outcome: Outcome;
  message: string;
  fileName?: string;
  taskId?: string;
  /** The bridge or the app is down: the remaining links would fail too. */
  stop: boolean;
}

export function isLinkResult(value: unknown): value is LinkResult {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    (v.outcome === "added" || v.outcome === "duplicate" || v.outcome === "failed") &&
    typeof v.message === "string" &&
    typeof v.stop === "boolean" &&
    (v.fileName === undefined || typeof v.fileName === "string") &&
    (v.taskId === undefined || typeof v.taskId === "string")
  );
}

/** The answer to `ping`. */
export interface PingReply {
  bridge: BridgeStatus;
}

export interface Activity {
  /** Milliseconds since the epoch. */
  at: number;
  /** File name or host, never the full URL. */
  label: string;
  source: "link" | "capture";
  /** `kept` means capture failed and the browser downloaded it instead. */
  outcome: Outcome | "kept";
  message: string;
  /** The app's id for the download, when it took it. */
  taskId?: string;
  /** The app took it but waits for the user to confirm it before starting. */
  awaitingConfirmation?: boolean;
}

export interface StatusReply {
  bridge: BridgeStatus;
  /** The `downloads` permission is granted, so browser downloads are captured. */
  capture: boolean;
  notify: boolean;
  /** For the popup's "Pause capture on this site". */
  excludedSites: string[];
  activity: Activity[];
  /** Live state of the downloads in `activity`, when the app could be asked. */
  tasks: TaskStatus[];
}

export interface TasksReply {
  /** Null when the app could not be asked. */
  tasks: TaskStatus[] | null;
}

/** The app's ids for the listed downloads, without repeats. */
export function taskIdsOf(list: readonly Activity[]): string[] {
  return [...new Set(list.flatMap((a) => (a.taskId ? [a.taskId] : [])))];
}

export const MAX_ACTIVITY = 5;

/** Newest first, capped. */
export function addActivity(list: readonly Activity[], entry: Activity): Activity[] {
  return [entry, ...list].slice(0, MAX_ACTIVITY);
}

export function parseActivity(value: unknown): Activity[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((a): a is Activity => {
      if (typeof a !== "object" || a === null) return false;
      const v = a as Record<string, unknown>;
      return (
        typeof v.at === "number" &&
        typeof v.label === "string" &&
        typeof v.message === "string" &&
        (v.source === "link" || v.source === "capture") &&
        ["added", "duplicate", "failed", "kept"].includes(v.outcome as string) &&
        (v.taskId === undefined || typeof v.taskId === "string") &&
        (v.awaitingConfirmation === undefined || typeof v.awaitingConfirmation === "boolean")
      );
    })
    .slice(0, MAX_ACTIVITY);
}

/** "just now", "4 min ago", "2 h ago", or a date. */
export function relativeTime(at: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - at) / 1000));
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return new Date(at).toISOString().slice(0, 10);
}
