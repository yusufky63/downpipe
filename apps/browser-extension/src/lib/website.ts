// downpipe.xyz, the project's website, may ask this extension two things
// (Chrome and Edge, through `externally_connectable` in the manifest;
// Firefox has no such key, so there the site only offers the app): whether
// it is installed, and to send one YouTube video, playlist or channel page
// to the app. The website's server cannot read YouTube, which refuses data
// centres; this hands the link to the app on the user's own connection.
// Nothing else crosses: no other links, no downloads list, no settings.
import type { SenderLike } from "./sender";
import { siteList, sitePage } from "./sites";

export const WEBSITE_ORIGINS: readonly string[] = ["https://downpipe.xyz", "https://www.downpipe.xyz"];

/** At most one page per this many milliseconds, so a page cannot flood the app. */
export const WEBSITE_SEND_GAP_MS = 3000;

const MAX_URL = 2048;
const MAX_TITLE = 200;

export type WebsiteRequest = { kind: "hello" } | { kind: "send_page"; url: string; title?: string };

/** Why a page did not reach the app, in codes the website words itself. */
export type WebsiteFailure = "app_unreachable" | "too_fast" | "refused";

export type WebsiteReply =
  | { kind: "hello"; version: string }
  | { kind: "sent"; outcome: "added" | "duplicate" }
  | { kind: "sent"; outcome: "failed"; reason: WebsiteFailure };

/**
 * A web page of downpipe.xyz. The manifest lets no other site through; the
 * origin is checked again here so that a wider manifest would not widen
 * what is accepted. Extensions carry an id, web pages do not.
 */
export function isWebsiteSender(sender: SenderLike): boolean {
  if (sender.id !== undefined) return false;
  let origin = sender.origin;
  if (origin === undefined && typeof sender.url === "string") {
    try {
      origin = new URL(sender.url).origin;
    } catch {
      return false;
    }
  }
  return origin !== undefined && WEBSITE_ORIGINS.includes(origin);
}

/** The request, or null for anything else (then there is no answer at all). */
export function parseWebsiteRequest(message: unknown): WebsiteRequest | null {
  if (typeof message !== "object" || message === null) return null;
  const m = message as Record<string, unknown>;
  if (m.kind === "hello") return { kind: "hello" };
  if (m.kind !== "send_page" || typeof m.url !== "string" || m.url.length > MAX_URL) return null;
  // Only what the app's site provider takes; any other link is refused here.
  if (!sitePage(m.url) && !siteList(m.url)) return null;
  const title = typeof m.title === "string" ? m.title.trim().slice(0, MAX_TITLE) : "";
  return title ? { kind: "send_page", url: m.url, title } : { kind: "send_page", url: m.url };
}

/** True at most once per `gapMs`; the caller refuses the rest as `too_fast`. */
export function sendGate(gapMs: number, now: () => number = Date.now): () => boolean {
  let last = Number.NEGATIVE_INFINITY;
  return () => {
    const at = now();
    if (at - last < gapMs) return false;
    last = at;
    return true;
  };
}
