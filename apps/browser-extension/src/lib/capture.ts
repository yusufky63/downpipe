import type { DownloadFields } from "./protocol";
import { referrerFor, webUrl } from "./url";

/** The parts of `downloads.DownloadItem` capture looks at. */
export interface DownloadItemLike {
  id: number;
  url: string;
  referrer?: string;
  mime?: string;
  filename?: string;
  incognito?: boolean;
  byExtensionId?: string;
  state?: string;
}

export type SkipReason = "own_download" | "private_window" | "not_in_progress" | "unsupported_url";

export type CaptureDecision = { capture: true; fields: DownloadFields } | { capture: false; reason: SkipReason };

/**
 * Whether a browser download should go to Downpipe instead.
 *
 * Skipped: downloads this extension started (the fallback path), private
 * windows (the app keeps a history of what it downloads), finished or
 * interrupted items, and `blob:` / `data:` / `file:` URLs, which only exist
 * inside the browser.
 */
export function decideCapture(item: DownloadItemLike, extensionId: string): CaptureDecision {
  if (item.byExtensionId && item.byExtensionId === extensionId) return { capture: false, reason: "own_download" };
  if (item.incognito) return { capture: false, reason: "private_window" };
  if (item.state !== undefined && item.state !== "in_progress") return { capture: false, reason: "not_in_progress" };
  // `url` rather than `finalUrl`: redirect targets are often short-lived signed
  // links, and Downpipe resumes through the original URL.
  if (!webUrl(item.url)) return { capture: false, reason: "unsupported_url" };
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
