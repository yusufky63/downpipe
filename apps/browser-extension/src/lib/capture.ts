import {
  CAPTURE_EVERYTHING,
  downloadName,
  hostOf,
  knownSize,
  matchingSite,
  typeListed,
  type CaptureFilters,
} from "./filters";
import type { DownloadFields } from "./protocol";
import { referrerFor, webUrl } from "./url";

/** The parts of `downloads.DownloadItem` capture looks at. */
export interface DownloadItemLike {
  id: number;
  url: string;
  finalUrl?: string;
  referrer?: string;
  mime?: string;
  filename?: string;
  incognito?: boolean;
  byExtensionId?: string;
  state?: string;
  /** -1 (or 0 in Chrome) while unknown. */
  totalBytes?: number;
  fileSize?: number;
}

export type SkipReason =
  | "own_download"
  | "private_window"
  | "not_in_progress"
  | "unsupported_url"
  | "excluded_site"
  | "unlisted_type"
  | "too_small";

export type CaptureDecision = { capture: true; fields: DownloadFields } | { capture: false; reason: SkipReason };

/**
 * Whether a browser download should go to Downpipe instead.
 *
 * Always skipped: downloads this extension started (the fallback path),
 * private windows (the app keeps a history of what it downloads), finished
 * or interrupted items, and `blob:` / `data:` / `file:` URLs, which only
 * exist inside the browser. Then the user's filters: sites they excluded
 * (the download's own host, where it was redirected to, or the page it
 * came from), file types they did not list, and files under the minimum
 * size. A size the browser does not know yet never counts as too small.
 */
export function decideCapture(
  item: DownloadItemLike,
  extensionId: string,
  filters: CaptureFilters = CAPTURE_EVERYTHING,
): CaptureDecision {
  if (item.byExtensionId && item.byExtensionId === extensionId) return { capture: false, reason: "own_download" };
  if (item.incognito) return { capture: false, reason: "private_window" };
  if (item.state !== undefined && item.state !== "in_progress") return { capture: false, reason: "not_in_progress" };
  // `url` rather than `finalUrl`: redirect targets are often short-lived signed
  // links, and Downpipe resumes through the original URL.
  if (!webUrl(item.url)) return { capture: false, reason: "unsupported_url" };
  const hosts = [hostOf(item.url), hostOf(item.finalUrl), hostOf(item.referrer)];
  if (matchingSite(hosts, filters.excludedSites)) return { capture: false, reason: "excluded_site" };
  if (!filters.captureAll && filters.captureTypes.length > 0) {
    if (!typeListed(downloadName(item.filename, item.url), filters.captureTypes)) {
      return { capture: false, reason: "unlisted_type" };
    }
  }
  const size = knownSize(item.totalBytes, item.fileSize);
  if (filters.minSizeBytes > 0 && size !== undefined && size < filters.minSizeBytes) {
    return { capture: false, reason: "too_small" };
  }
  return {
    capture: true,
    fields: {
      resourceUrl: item.url,
      // The browser already applied the page's referrer policy to this one.
      referrer: item.referrer || undefined,
      mimeType: item.mime || undefined,
      fileName: item.filename || undefined,
    },
  };
}

/** The parts of `contextMenus.OnClickData` a link click needs. */
export interface LinkClickLike {
  linkUrl?: string;
  pageUrl?: string;
  frameUrl?: string;
}

/** Null when the link is not an http(s) URL. */
export function linkFields(info: LinkClickLike): DownloadFields | null {
  if (!info.linkUrl || !webUrl(info.linkUrl)) return null;
  // A link inside an iframe belongs to the frame's document.
  const page = info.frameUrl || info.pageUrl;
  return {
    resourceUrl: info.linkUrl,
    pageUrl: page,
    referrer: referrerFor(page, info.linkUrl),
  };
}
