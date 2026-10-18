// The links page: what a page's links look like once collected, how they
// are checked, grouped and filtered. The page's DOM is untrusted, so
// everything that comes out of it is validated here before it is stored
// or shown.
import { t } from "../i18n";
import type { CollectedPage } from "./collect";
import { MAX_URL_LENGTH, baseName, magnetName, magnetUrl, webUrl } from "./url";

export const MAX_PAGE_LINKS = 2000;
export const MAX_LINK_TEXT = 200;
const MAX_TITLE = 300;
/** Collections waiting for their links page are dropped after this long. */
export const COLLECTION_TTL_MS = 10 * 60_000;
const KEY_PREFIX = "links:";
const ID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export interface PageLink {
  /** http(s), without the fragment. */
  url: string;
  /** The link's visible text, or "" when it has none. */
  text: string;
}

export interface LinkCollection {
  /** The page the links are on, without credentials; "" when it could not be read. */
  pageUrl: string;
  title: string;
  links: PageLink[];
  /** More links were on the page than were kept. */
  truncated: boolean;
  /** The browser did not let the extension read the page. */
  unreadable: boolean;
  /** Milliseconds since the epoch, when it was collected. */
  at: number;
}

/** `storage.session` key for one collection. */
export function collectionKey(id: string): string | null {
  return ID_PATTERN.test(id) ? `${KEY_PREFIX}${id}` : null;
}

/** Keys of collections nobody opened in time. */
export function staleCollectionKeys(stored: Record<string, unknown>, now: number): string[] {
  return Object.entries(stored)
    .filter(([key, value]) => {
      if (!key.startsWith(KEY_PREFIX)) return false;
      const at = typeof value === "object" && value !== null ? (value as Record<string, unknown>).at : undefined;
      return typeof at !== "number" || now - at > COLLECTION_TTL_MS || at > now + 60_000;
    })
    .map(([key]) => key);
}

function cleanText(value: unknown, max: number): string {
  if (typeof value !== "string") return "";
  let out = "";
  for (const ch of value.slice(0, max * 2)) {
    const code = ch.codePointAt(0) ?? 0;
    // Control characters would only garble the list.
    out += code < 0x20 || (code >= 0x7f && code <= 0x9f) ? " " : ch;
  }
  return out.replace(/\s+/g, " ").trim().slice(0, max);
}

/** The page address as it may be shown and sent: http(s) only, no credentials or fragment. */
function pageAddress(raw: unknown): string {
  const url = typeof raw === "string" ? webUrl(raw) : null;
  if (!url) return "";
  url.username = "";
  url.password = "";
  url.hash = "";
  return url.href;
}

/**
 * Checks and tidies a list of links: http(s) only, at most 8 KiB, one entry
 * per address (fragments dropped, the first non-empty text kept), no link to
 * the page itself, capped.
 */
export function cleanLinks(raw: unknown, pageUrl: string, max = MAX_PAGE_LINKS): { links: PageLink[]; dropped: boolean } {
  const links: PageLink[] = [];
  const index = new Map<string, number>();
  let dropped = false;
  if (!Array.isArray(raw)) return { links, dropped };
  for (const entry of raw) {
    if (typeof entry !== "object" || entry === null) continue;
    const v = entry as Record<string, unknown>;
    const magnet = typeof v.url === "string" ? magnetUrl(v.url) : null;
    const url = magnet === null && typeof v.url === "string" ? webUrl(v.url) : null;
    if (magnet) {
      if (index.has(magnet)) continue;
      if (links.length >= max) {
        dropped = true;
        break;
      }
      index.set(magnet, links.length);
      links.push({ url: magnet, text: cleanText(v.text, MAX_LINK_TEXT) });
      continue;
    }
    if (!url) continue;
    url.hash = "";
    if (url.href.length > MAX_URL_LENGTH || url.href === pageUrl) continue;
    const text = cleanText(v.text, MAX_LINK_TEXT);
    const seen = index.get(url.href);
    if (seen !== undefined) {
      const first = links[seen];
      if (first && !first.text && text) first.text = text;
      continue;
    }
    if (links.length >= max) {
      dropped = true;
      break;
    }
    index.set(url.href, links.length);
    links.push({ url: url.href, text });
  }
  return { links, dropped };
}

/** What the injected collector returned, checked; null when it is not that shape. */
export function collectionFromPage(raw: unknown, now: number): LinkCollection | null {
  if (typeof raw !== "object" || raw === null) return null;
  const v = raw as Partial<Record<keyof CollectedPage, unknown>>;
  if (!Array.isArray(v.links)) return null;
  const pageUrl = pageAddress(v.pageUrl);
  const { links, dropped } = cleanLinks(v.links, pageUrl);
  return {
    pageUrl,
    title: cleanText(v.title, MAX_TITLE),
    links,
    truncated: v.truncated === true || dropped,
    unreadable: false,
    at: now,
  };
}

/** Stand-in for a page the browser would not let the extension read. */
export function unreadableCollection(pageUrl: string | undefined, now: number): LinkCollection {
  return { pageUrl: pageAddress(pageUrl), title: "", links: [], truncated: false, unreadable: true, at: now };
}

/** A stored collection, read back by the links page. */
export function parseCollection(value: unknown): LinkCollection | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  if (!Array.isArray(v.links) || typeof v.at !== "number" || !Number.isFinite(v.at)) return null;
  if (typeof v.truncated !== "boolean" || typeof v.unreadable !== "boolean") return null;
  const pageUrl = pageAddress(v.pageUrl);
  const { links, dropped } = cleanLinks(v.links, pageUrl);
  return {
    pageUrl,
    title: cleanText(v.title, MAX_TITLE),
    links,
    truncated: v.truncated || dropped,
    unreadable: v.unreadable,
    at: v.at,
  };
}

// ---------- Grouping and filtering ----------

/** Pages, not files: hidden by default, like IDM. */
export const WEB_PAGE_EXTENSIONS: readonly string[] = [
  "html", "htm", "xhtml", "shtml", "php", "asp", "aspx", "jsp", "jspx", "cfm", "cgi",
];

/** The chip for links whose type has no chip of its own, or no extension at all. */
export const OTHER = "other";

export interface LinkView extends PageLink {
  /** Decoded last path segment, or the host for a bare address. */
  name: string;
  /** Lower-case extension, or null when the address has none. */
  ext: string | null;
  /** A web page rather than a file: a page extension or no extension. */
  page: boolean;
}

function lastSegment(url: URL): string {
  const last = baseName(url.pathname);
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

/**
 * The extension of the address's last path segment. The query string does
 * not count (`get.php?f=a.zip` is `php`). All-digit suffixes count only as
 * split archive parts (`.001`), not version numbers (`/v1.2`).
 */
export function linkExtension(raw: string): string | null {
  const url = webUrl(raw);
  if (!url) return null;
  const name = lastSegment(url).toLowerCase();
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) return null;
  const ext = name.slice(dot + 1);
  if (!/^[a-z0-9]{1,10}$/.test(ext)) return null;
  if (/^\d+$/.test(ext) && ext.length !== 3) return null;
  return ext;
}

export function linkView(link: PageLink): LinkView {
  // A torrent: named by its link's `dn`, grouped under "torrent".
  if (magnetUrl(link.url)) return { ...link, name: magnetName(link.url) ?? "magnet", ext: "torrent", page: false };
  const url = webUrl(link.url);
  const ext = linkExtension(link.url);
  const name = (url && lastSegment(url)) || url?.hostname || link.url;
  return { ...link, name, ext, page: ext === null || WEB_PAGE_EXTENSIONS.includes(ext) };
}

export interface Chip {
  key: string;
  label: string;
  count: number;
}

/**
 * One chip per extension among these links, most frequent first, plus
 * "Other" for links without an extension and for types beyond `max`.
 */
export function extensionChips(views: readonly LinkView[], max = 12): Chip[] {
  const counts = new Map<string, number>();
  let other = 0;
  for (const view of views) {
    if (view.ext) counts.set(view.ext, (counts.get(view.ext) ?? 0) + 1);
    else other += 1;
  }
  const sorted = [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const chips = sorted.slice(0, max).map(([key, count]) => ({ key, label: key, count }));
  for (const [, count] of sorted.slice(max)) other += count;
  if (other > 0) chips.push({ key: OTHER, label: t("links.otherTypes"), count: other });
  return chips;
}

/** The chip a link belongs to, given the chips on show. */
export function chipOf(view: LinkView, chips: readonly Chip[]): string {
  return view.ext && chips.some((c) => c.key === view.ext) ? view.ext : OTHER;
}

export interface LinkFilter {
  /** Words that must all appear in the text, name or address; case is ignored. */
  query: string;
  hidePages: boolean;
  /** Chip keys to show; empty shows every type. */
  types: ReadonlySet<string>;
}

/** Links left once "Hide web pages" applies; the chips are built from these. */
export function withoutPages(views: readonly LinkView[], hidePages: boolean): LinkView[] {
  return hidePages ? views.filter((v) => !v.page) : [...views];
}

export function filterLinks(views: readonly LinkView[], filter: LinkFilter): LinkView[] {
  const base = withoutPages(views, filter.hidePages);
  const chips = filter.types.size ? extensionChips(base) : [];
  const words = filter.query.toLowerCase().split(/\s+/).filter(Boolean);
  return base.filter((view) => {
    if (filter.types.size && !filter.types.has(chipOf(view, chips))) return false;
    if (!words.length) return true;
    const haystack = `${view.text}\n${view.name}\n${view.url}`.toLowerCase();
    return words.every((w) => haystack.includes(w));
  });
}

// ---------- Results ----------

export type LinkOutcome = "added" | "duplicate" | "failed";

/** "3 added, 1 already in Downpipe, 2 failed." */
export function summarize(outcomes: readonly LinkOutcome[]): string {
  const count = (o: LinkOutcome) => outcomes.filter((x) => x === o).length;
  const parts: string[] = [];
  const added = count("added");
  const duplicate = count("duplicate");
  const failed = count("failed");
  if (added) parts.push(t("links.added", { count: added }));
  if (duplicate) parts.push(t("links.duplicate", { count: duplicate }));
  if (failed) parts.push(t("links.failed", { count: failed }));
  return parts.length ? t("links.summary", { parts: parts.join(t("links.separator")) }) : t("links.nothingSent");
}

/** "Download 1 link", "Download 12 links". */
export function downloadLabel(n: number): string {
  return t("links.downloadCount", { count: n });
}
