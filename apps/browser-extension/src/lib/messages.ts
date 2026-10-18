// What the extension's own pages (popup, options, links page) and the
// service worker say to each other.
import { t } from "../i18n/core";
import { isBypassKey, parseBypassKey, type BypassKey } from "./bypass";
import { parseFilters, normalizeSitePattern, validFilters, type CaptureFilters } from "./filters";
import {
  isMediaId,
  parseMediaFilters,
  parseTabMedia,
  validMediaFilters,
  type MediaCandidate,
  type MediaFilters,
} from "./media";
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
  /** The options page's media settings. */
  | { kind: "save_media"; media: MediaFilters }
  /** Adds or removes one exact entry of the excluded sites. */
  | { kind: "set_site_paused"; site: string; paused: boolean }
  /** Collect the links of this tab and open the links page; needs `activeTab` for it. */
  | { kind: "collect_links"; tabId: number }
  /** One link from the links page, sent as if it had been clicked on `pageUrl`. */
  | { kind: "send_link"; url: string; pageUrl?: string }
  /** The media detected in this tab. */
  | { kind: "media_list"; tabId: number }
  /** Send one of them to Downpipe. */
  | { kind: "send_media"; tabId: number; mediaId: string }
  /** Send the tab's video page (YouTube) to Downpipe's site provider. */
  | { kind: "send_page"; tabId: number }
  /** The options page's key for letting the browser download a link itself. */
  | { kind: "set_bypass_key"; key: BypassKey };

/**
 * What the overlay button in a page may ask. The tab is never part of the
 * message: the service worker takes it from the sender, so a page can only
 * ever reach its own tab's list.
 */
export type OverlayRequest =
  /**
   * The tab's media; `src` is the playing element's own http(s) source and
   * `poster` its poster image, if it has them (the poster picks the video's
   * own streams on a page with several, see `mediaForVideo`).
   */
  | { kind: "overlay_list"; src?: string; poster?: string }
  | { kind: "overlay_send"; mediaId: string }
  /** Send this frame's or tab's video page; its item has `PAGE_ITEM_ID`. */
  | { kind: "overlay_page" }
  /** How the button behaves on this tab's site: `OverlayPrefs`. */
  | { kind: "overlay_prefs" }
  /** "Hide on this site": the tab's site joins the overlay's own exclude list. */
  | { kind: "overlay_hide_site" }
  /** Where the user dragged the button, as fractions of the video's width and height. */
  | { kind: "overlay_place"; x: number; y: number };

/** The answer to `overlay_prefs`. */
export interface OverlayPrefs {
  hoverOnly: boolean;
  /** The tab's site is on the list (the frame itself may be another site). */
  hidden: boolean;
  /** The button's top right corner in the video, as fractions; none until dragged there. */
  place?: { x: number; y: number };
}

export function isOverlayPrefs(value: unknown): value is OverlayPrefs {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.hoverOnly === "boolean" &&
    typeof v.hidden === "boolean" &&
    (v.place === undefined || isPlace(v.place))
  );
}

const isFraction = (n: unknown): n is number => typeof n === "number" && Number.isFinite(n) && n >= 0 && n <= 1;

export function isPlace(value: unknown): value is { x: number; y: number } {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return Object.keys(v).length === 2 && isFraction(v.x) && isFraction(v.y);
}

/** The overlay menu's entry for the page itself, next to detected media ids. */
export const PAGE_ITEM_ID = "page";

const SIMPLE_KINDS = ["status", "tasks", "ping", "get_settings"] as const;

function hasKeys(v: Record<string, unknown>, ...keys: string[]): boolean {
  const own = Object.keys(v);
  return own.length === keys.length && keys.every((k) => own.includes(k));
}

function isWebUrl(value: unknown): value is string {
  return typeof value === "string" && webUrl(value) !== null;
}

function isTabId(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
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
    case "set_bypass_key":
      return hasKeys(v, "kind", "key") && isBypassKey(v.key) ? { kind: "set_bypass_key", key: v.key } : null;
    case "save_media": {
      const media = hasKeys(v, "kind", "media") ? validMediaFilters(v.media) : null;
      return media ? { kind: "save_media", media } : null;
    }
    case "set_site_paused": {
      if (!hasKeys(v, "kind", "site", "paused") || typeof v.site !== "string" || typeof v.paused !== "boolean") {
        return null;
      }
      return normalizeSitePattern(v.site) === v.site ? { kind: "set_site_paused", site: v.site, paused: v.paused } : null;
    }
    case "collect_links":
    case "media_list":
    case "send_page":
      return hasKeys(v, "kind", "tabId") && isTabId(v.tabId) ? { kind: v.kind, tabId: v.tabId } : null;
    case "send_media":
      return hasKeys(v, "kind", "tabId", "mediaId") && isTabId(v.tabId) && isMediaId(v.mediaId)
        ? { kind: "send_media", tabId: v.tabId, mediaId: v.mediaId }
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

/** Exact shapes only, like `parsePopupRequest`. */
export function parseOverlayRequest(value: unknown): OverlayRequest | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  switch (v.kind) {
    case "overlay_list": {
      const extra = Object.keys(v).filter((k) => k !== "kind");
      if (!extra.every((k) => (k === "src" || k === "poster") && isWebUrl(v[k]))) return null;
      return {
        kind: "overlay_list",
        ...(isWebUrl(v.src) ? { src: v.src } : {}),
        ...(isWebUrl(v.poster) ? { poster: v.poster } : {}),
      };
    }
    case "overlay_send":
      return hasKeys(v, "kind", "mediaId") && isMediaId(v.mediaId) ? { kind: "overlay_send", mediaId: v.mediaId } : null;
    case "overlay_page":
    case "overlay_prefs":
    case "overlay_hide_site":
      return hasKeys(v, "kind") ? { kind: v.kind } : null;
    case "overlay_place":
      return hasKeys(v, "kind", "x", "y") && isFraction(v.x) && isFraction(v.y) ? { kind: "overlay_place", x: v.x, y: v.y } : null;
    default:
      return null;
  }
}

/** The tab's video page, which the site provider can download. */
export interface PageItem {
  site: string;
  /** A playlist or channel rather than one video. */
  list?: "playlist" | "channel";
  /** The tab's title without the site's name, when there is one. */
  title?: string;
}

/** The answer to `media_list`. */
export interface MediaReply {
  /** Detection is on: the `webRequest` permission and access to sites are granted. */
  enabled: boolean;
  /** Newest first. */
  items: MediaCandidate[];
  /** The tab's site is on the excluded sites list, by this entry. */
  excludedBy?: string;
  /** Set on a video page (YouTube), whether detection is on or not. */
  page?: PageItem;
  /** The tab's title, which names the rows (and the downloads). */
  title?: string;
}

function isPageItem(value: unknown): value is PageItem {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.site === "string" &&
    (v.title === undefined || typeof v.title === "string") &&
    (v.list === undefined || v.list === "playlist" || v.list === "channel")
  );
}

export function isMediaReply(value: unknown): value is MediaReply {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.enabled === "boolean" &&
    Array.isArray(v.items) &&
    parseTabMedia(v.items).length === v.items.length &&
    (v.excludedBy === undefined || typeof v.excludedBy === "string") &&
    (v.page === undefined || isPageItem(v.page)) &&
    (v.title === undefined || typeof v.title === "string")
  );
}

/** One row of the overlay button's menu: no URLs, only what it shows. */
export interface OverlayItem {
  mediaId: string;
  label: string;
  detail: string;
}

export function isOverlayItems(value: unknown): value is { items: OverlayItem[] } {
  if (typeof value !== "object" || value === null) return false;
  const items = (value as Record<string, unknown>).items;
  return (
    Array.isArray(items) &&
    items.every((i) => {
      if (typeof i !== "object" || i === null) return false;
      const v = i as Record<string, unknown>;
      return (
        (isMediaId(v.mediaId) || v.mediaId === PAGE_ITEM_ID) && typeof v.label === "string" && typeof v.detail === "string"
      );
    })
  );
}

/** What the overlay button says after a send: the service worker's answer in short. */
export function overlayResultText(reply: unknown): { text: string; failed: boolean } {
  if (!isLinkResult(reply)) {
    return { text: t("overlay.noAnswer"), failed: true };
  }
  switch (reply.outcome) {
    case "added":
      // The service worker words its replies in the same language as this script.
      return { text: reply.message === t("results.added") ? t("results.sent") : reply.message, failed: false };
    case "duplicate":
      return { text: t("overlay.duplicate"), failed: false };
    case "failed":
      return { text: reply.message, failed: true };
  }
}

/** Stored in `storage.local` under one key. */
export interface Settings extends CaptureFilters, MediaFilters {
  /** Show a notification when a download is handed over, or could not be. */
  notify: boolean;
  /** Held while clicking a link, the browser downloads it itself (lib/bypass.ts). */
  bypassKey: BypassKey;
}

/**
 * Anything stored, including settings from before the capture filters
 * existed (`{ notify }` only): missing or malformed fields get defaults.
 */
export function parseSettings(value: unknown): Settings {
  const v = typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  return { notify: v.notify !== false, ...parseFilters(v), ...parseMediaFilters(v), bypassKey: parseBypassKey(v.bypassKey) };
}

export function isSettings(value: unknown): value is Settings {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.notify === "boolean" &&
    isBypassKey(v.bypassKey) &&
    validFilters({
      captureAll: v.captureAll,
      captureTypes: v.captureTypes,
      excludedSites: v.excludedSites,
      minSizeBytes: v.minSizeBytes,
    }) !== null &&
    validMediaFilters({
      mediaOverlay: v.mediaOverlay,
      overlayHoverOnly: v.overlayHoverOnly,
      overlaySites: v.overlaySites,
      mediaTypes: v.mediaTypes,
      mediaMinBytes: v.mediaMinBytes,
    }) !== null
  );
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
  /** `media`: a stream or media file from the popup's media list or the overlay button. */
  source: "link" | "capture" | "media";
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
        (v.source === "link" || v.source === "capture" || v.source === "media") &&
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
  if (seconds < 45) return t("common.justNow");
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return t("common.minutesAgo", { count: minutes });
  const hours = Math.round(minutes / 60);
  if (hours < 24) return t("common.hoursAgo", { count: hours });
  return new Date(at).toISOString().slice(0, 10);
}
