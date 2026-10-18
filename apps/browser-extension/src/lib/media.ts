// Media detection: which responses a page received are streams or media
// files worth offering, and the per-tab list of them. Pure functions; the
// service worker feeds them `webRequest` events while detection is on.
//
// The approach is the one IDM's download panel and open-source sniffers
// (cat-catch, Stream Detector) take: classify each response by its
// Content-Type and URL, keep what the page's own request sent as Referer
// and Origin, and list the result per tab. No code is taken from them.
import { t } from "../i18n";
import { MAX_EXCLUDED_SITES, normalizeSitePattern, normalizeType, parseTypeList, type ListParse } from "./filters";
import { mimeEssence } from "./protocol";
import { isSiteMedia } from "./sites";
import { MAX_URL_LENGTH, baseName, webUrl, withoutSecrets } from "./url";

export type MediaKind = "hls" | "dash" | "direct";

/** One stream or media file a tab played. */
export interface MediaCandidate {
  /** Random; what the popup and the overlay button refer to. */
  id: string;
  kind: MediaKind;
  url: string;
  /** `type/subtype` the server sent, for direct files. */
  mime?: string;
  /** Bytes, when the server said. */
  size?: number;
  /** When it was first seen, ms since the epoch. */
  at: number;
  /**
   * The document that requested it, as far as the browser says: Firefox
   * reports its URL, Chrome only its origin. The fallback for the referrer,
   * because players often sit in a frame of another site.
   */
  frameUrl?: string;
  /** The `Referer` the page's own request sent, credentials and fragment removed. */
  referrer?: string;
  /** The `Origin` it sent (players fetching with CORS send one). */
  origin?: string;
  /**
   * Set on an HLS master playlist the extension read: how many video
   * qualities it lists (0 when it only lists audio).
   */
  qualities?: number;
}

/** The parts of a `webRequest` response event classification looks at. */
export interface ResponseLike {
  url: string;
  /** `webRequest.ResourceType`: `media`, `xmlhttprequest`, `other`, ... */
  type: string;
  statusCode: number;
  tabId: number;
  responseHeaders?: { name: string; value?: string }[];
}

/** Matches the patterns the optional host permission and the listeners use. */
export const WEB_ORIGINS = ["http://*/*", "https://*/*"];
/** Older entries fall off; a long single-page session plays many videos. */
export const MAX_TAB_MEDIA = 20;

// ---------- Settings ----------

/** Which media files are listed, like IDM's list of types its panel offers. */
export interface MediaFilters {
  /** The Download button over playing videos. */
  mediaOverlay: boolean;
  /**
   * The button shows only while the pointer is over the video, not for a
   * few seconds after a video starts.
   */
  overlayHoverOnly: boolean;
  /**
   * Sites where the button never shows ("Hide on this site"). Unlike the
   * excluded sites, detection and capture still work there.
   */
  overlaySites: string[];
  /** Extensions of media files that are listed. Streams are always listed. */
  mediaTypes: string[];
  /** Media files smaller than this are left out; 0 lists them all. */
  mediaMinBytes: number;
}

export const DEFAULT_MEDIA_TYPES: readonly string[] = [
  // Video
  "mp4", "m4v", "webm", "mov", "mkv", "ogv", "flv",
  // Audio
  "mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav", "weba",
];
/** Notification sounds and short previews are not worth offering. */
export const DEFAULT_MEDIA_MIN_BYTES = 256 * 1024;
export const MAX_MEDIA_TYPES = 100;
/** 1 TiB, like the capture filter's limit. */
export const MAX_MEDIA_MIN_BYTES = 1024 ** 4;

export function defaultMediaFilters(): MediaFilters {
  return {
    mediaOverlay: true,
    overlayHoverOnly: false,
    overlaySites: [],
    mediaTypes: [...DEFAULT_MEDIA_TYPES],
    mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES,
  };
}

function siteList(value: unknown): string[] | null {
  if (!Array.isArray(value)) return null;
  const out: string[] = [];
  for (const entry of value) {
    const site = typeof entry === "string" ? normalizeSitePattern(entry) : null;
    if (site && !out.includes(site)) out.push(site);
    if (out.length >= MAX_EXCLUDED_SITES) break;
  }
  return out;
}

/** From `storage.local`: missing or malformed fields get their defaults. */
export function parseMediaFilters(v: Record<string, unknown>): MediaFilters {
  const d = defaultMediaFilters();
  const types = Array.isArray(v.mediaTypes)
    ? [...new Set(v.mediaTypes.flatMap((t) => (typeof t === "string" ? (normalizeType(t) ?? []) : [])))].slice(0, MAX_MEDIA_TYPES)
    : d.mediaTypes;
  const size = v.mediaMinBytes;
  return {
    mediaOverlay: typeof v.mediaOverlay === "boolean" ? v.mediaOverlay : d.mediaOverlay,
    overlayHoverOnly: typeof v.overlayHoverOnly === "boolean" ? v.overlayHoverOnly : d.overlayHoverOnly,
    overlaySites: siteList(v.overlaySites) ?? d.overlaySites,
    mediaTypes: types,
    mediaMinBytes:
      typeof size === "number" && Number.isInteger(size) && size >= 0 && size <= MAX_MEDIA_MIN_BYTES ? size : d.mediaMinBytes,
  };
}

/** Strict check for what the options page saves: already normalized, exact keys. */
export function validMediaFilters(value: unknown): MediaFilters | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  if (Object.keys(v).length !== 5) return null;
  const { mediaOverlay, overlayHoverOnly, overlaySites, mediaTypes, mediaMinBytes } = v;
  if (typeof mediaOverlay !== "boolean" || typeof overlayHoverOnly !== "boolean") return null;
  if (
    !Array.isArray(overlaySites) ||
    overlaySites.length > MAX_EXCLUDED_SITES ||
    new Set(overlaySites).size !== overlaySites.length ||
    !overlaySites.every((s) => typeof s === "string" && normalizeSitePattern(s) === s)
  ) {
    return null;
  }
  if (
    !Array.isArray(mediaTypes) ||
    mediaTypes.length > MAX_MEDIA_TYPES ||
    new Set(mediaTypes).size !== mediaTypes.length ||
    !mediaTypes.every((t) => typeof t === "string" && normalizeType(t) === t)
  ) {
    return null;
  }
  if (typeof mediaMinBytes !== "number" || !Number.isInteger(mediaMinBytes) || mediaMinBytes < 0 || mediaMinBytes > MAX_MEDIA_MIN_BYTES) {
    return null;
  }
  return { mediaOverlay, overlayHoverOnly, overlaySites: [...overlaySites], mediaTypes: [...mediaTypes], mediaMinBytes };
}

/** The options page's media type field. Unlike capture, an empty list is allowed: only streams are listed then. */
export function parseMediaTypeList(text: string): ListParse {
  // The capture list's own limit does not apply here; this one does.
  return parseTypeList(text, MAX_MEDIA_TYPES);
}

// ---------- Classification ----------

const HLS_TYPES = new Set([
  "application/vnd.apple.mpegurl",
  "application/x-mpegurl",
  "application/mpegurl",
  "audio/mpegurl",
  "audio/x-mpegurl",
]);
const DASH_TYPES = new Set(["application/dash+xml"]);
// Fragments of a stream rather than whole files: listing them would offer
// hundreds of seconds-long pieces of one video.
const FRAGMENT_TYPES = new Set(["video/mp2t", "video/iso.segment", "audio/iso.segment"]);
const FRAGMENT_EXTENSIONS = new Set(["ts", "m4s", "m4f", "cmfv", "cmfa"]);
/** What a media file is called when its URL has no extension. */
const MIME_EXTENSIONS: Record<string, string> = {
  "video/mp4": "mp4",
  "video/x-m4v": "m4v",
  "video/webm": "webm",
  "video/quicktime": "mov",
  "video/x-matroska": "mkv",
  "video/ogg": "ogv",
  "video/x-flv": "flv",
  "audio/mpeg": "mp3",
  "audio/mp3": "mp3",
  "audio/mp4": "m4a",
  "audio/x-m4a": "m4a",
  "audio/aac": "aac",
  "audio/ogg": "ogg",
  "application/ogg": "ogg",
  "audio/opus": "opus",
  "audio/flac": "flac",
  "audio/x-flac": "flac",
  "audio/wav": "wav",
  "audio/x-wav": "wav",
  "audio/wave": "wav",
  "audio/webm": "weba",
};
const KNOWN_MEDIA_EXTENSIONS = new Set([...DEFAULT_MEDIA_TYPES, ...Object.values(MIME_EXTENSIONS)]);
/**
 * Query parameters that make a request ask for a piece of a file. Some CDNs
 * (Facebook's and Instagram's among them) serve progressive video this way;
 * without them the same URL answers with the whole file.
 */
const RANGE_PARAMS = ["bytestart", "byteend", "range"];

function header(r: ResponseLike, name: string): string | undefined {
  return r.responseHeaders?.find((h) => h.name.toLowerCase() === name)?.value;
}

function extension(url: URL): string {
  const name = baseName(url.pathname);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

/** The file's extension: from its URL when that is a media extension, else from its type. */
export function mediaExtension(rawUrl: string, mime: string | undefined): string | undefined {
  const url = webUrl(rawUrl);
  const ext = url ? extension(url) : "";
  if (KNOWN_MEDIA_EXTENSIONS.has(ext)) return ext;
  return mime ? MIME_EXTENSIONS[mime] : undefined;
}

/** The whole file's size: `Content-Range` total of a range answer, else `Content-Length`. */
export function responseSize(r: ResponseLike): number | undefined {
  const range = header(r, "content-range");
  const total = range ? /\/(\d+)\s*$/.exec(range)?.[1] : undefined;
  const raw = total ?? (r.statusCode === 200 ? header(r, "content-length") : undefined);
  const n = raw === undefined ? NaN : Number(raw);
  return Number.isSafeInteger(n) && n >= 0 ? n : undefined;
}

/** The URL without range query parameters, or null when it had none. */
export function withoutRangeParams(raw: string): string | null {
  const url = webUrl(raw);
  if (!url) return null;
  let removed = false;
  for (const name of RANGE_PARAMS) {
    if (url.searchParams.has(name)) {
      url.searchParams.delete(name);
      removed = true;
    }
  }
  return removed ? url.href : null;
}

/** What a response turned out to be. `url` is the address to list, which may differ from the request's. */
export type Classified = Pick<MediaCandidate, "kind" | "mime" | "size"> & { url: string };

/**
 * Whether a response is a stream manifest or a media file a page played,
 * and which. Manifests count from any request type (players fetch them
 * with XHR). A file counts when a `<video>` or `<audio>` element loaded it,
 * or when a script fetched it whole: a range answer that names the file's
 * total size, or a request whose range is in its query string. Players
 * that fetch media themselves otherwise request fragments, not files.
 * Files must have a listed type and reach the minimum size, when known.
 */
export function classifyResponse(r: ResponseLike, filters: MediaFilters = defaultMediaFilters()): Classified | null {
  if (r.tabId < 0 || r.statusCode < 200 || r.statusCode > 299) return null;
  if (r.url.length > MAX_URL_LENGTH) return null;
  const url = webUrl(r.url);
  if (!url) return null;
  // YouTube's video servers only serve pieces its own player can use; the
  // page itself is offered instead (lib/sites.ts).
  if (isSiteMedia(r.url)) return null;
  const mime = mimeEssence(header(r, "content-type"));
  const ext = extension(url);
  if (ext === "m3u8" || (mime !== undefined && HLS_TYPES.has(mime))) return { kind: "hls", url: r.url };
  if (ext === "mpd" || (mime !== undefined && DASH_TYPES.has(mime))) return { kind: "dash", url: r.url };
  if (FRAGMENT_EXTENSIONS.has(ext) || (mime !== undefined && FRAGMENT_TYPES.has(mime))) return null;

  const whole = withoutRangeParams(r.url);
  const element = r.type === "media";
  const rangeAnswer = r.statusCode === 206 && header(r, "content-range") !== undefined;
  if (!element && !whole && !rangeAnswer) return null;

  const mediaMime =
    mime !== undefined && (mime.startsWith("video/") || mime.startsWith("audio/") || mime === "application/ogg");
  const vague = mime === undefined || mime === "application/octet-stream";
  if (!mediaMime && !(vague && KNOWN_MEDIA_EXTENSIONS.has(ext))) return null;
  const type = mediaExtension(r.url, mediaMime ? mime : undefined);
  if (filters.mediaTypes.length > 0 && (!type || !filters.mediaTypes.includes(type))) return null;
  // A piece asked for by query string says nothing about the whole file.
  const size = whole ? undefined : responseSize(r);
  if (filters.mediaMinBytes > 0 && size !== undefined && size < filters.mediaMinBytes) return null;
  return { kind: "direct", mime: mediaMime ? mime : undefined, size, url: whole ?? r.url };
}

/**
 * A media element's own source, offered by the overlay button: a playlist
 * by its extension, anything else as a file. `blob:` sources are not
 * http(s) and never get here; their real source comes from `webRequest`.
 */
export function classifySource(src: string): Pick<MediaCandidate, "kind"> | null {
  const url = webUrl(src);
  if (!url || src.length > MAX_URL_LENGTH) return null;
  const ext = extension(url);
  if (ext === "m3u8") return { kind: "hls" };
  if (ext === "mpd") return { kind: "dash" };
  return { kind: "direct" };
}

/** Where a request came from: Firefox's `documentUrl`, else Chrome's `initiator` origin. */
export function frameOf(details: { documentUrl?: string; initiator?: string }): string | undefined {
  const page = webUrl(details.documentUrl) ?? webUrl(details.initiator ? `${details.initiator}/` : undefined);
  if (!page) return undefined;
  return withoutSecrets(page).href;
}

/**
 * `Referer` and `Origin` of a request as the page sent them, so Downpipe
 * asks the server the way the page did. Anything that is not an http(s)
 * address (`null` origins, `about:` referrers) is left out.
 */
export function pageHeaders(headers: readonly { name: string; value?: string }[] | undefined): Pick<MediaCandidate, "referrer" | "origin"> {
  const get = (name: string) => headers?.find((h) => h.name.toLowerCase() === name)?.value;
  const referer = webUrl(get("referer"));
  const origin = webUrl(get("origin"));
  return {
    referrer: referer ? withoutSecrets(referer).href : undefined,
    origin: origin ? origin.origin : undefined,
  };
}

/**
 * Content script match patterns that leave out the excluded sites, so the
 * overlay button never runs there. `*.example.com` in a match pattern also
 * covers `example.com`, as in the excluded sites list. IPv6 hosts have no
 * match pattern form and are left to the service worker's own check.
 */
export function excludeMatchesFor(sites: readonly string[]): string[] {
  return sites.filter((s) => !s.includes("[")).map((s) => `*://${s}/*`);
}

// ---------- HLS playlists ----------
// A player loads the master playlist and then the quality and audio
// playlists it lists. Only the master is worth offering: Downpipe picks the
// quality from it. The service worker reads each new playlist once to tell
// which is which; these functions do the rest.

/** What reading a playlist showed. */
export type PlaylistInfo =
  | {
      master: true;
      /** Its quality, audio and I-frame playlists, as `playlistKey`s. */
      parts: string[];
      /** Distinct video qualities (by resolution, else by playlist). */
      qualities: number;
    }
  | { master: false };

/** A master is a few KiB; a media playlist is known by its first segment long before this. */
export const MAX_PLAYLIST_BYTES = 256 * 1024;
export const PLAYLIST_TIMEOUT_MS = 5_000;
/** Parts a tab remembers, so they stay hidden after their master falls off the list. */
export const MAX_TAB_PARTS = 400;
const MAX_MASTER_PARTS = 200;
/**
 * Low-latency HLS asks for each reload of a media playlist with these
 * query parameters; without them it is the same playlist.
 */
const HLS_DIRECTIVE_PREFIX = "_HLS_";

/** A playlist's address for comparing: no fragment, no low-latency reload parameters. */
export function playlistKey(raw: string): string {
  const url = webUrl(raw);
  if (!url) return raw;
  url.hash = "";
  for (const name of [...url.searchParams.keys()]) {
    if (name.startsWith(HLS_DIRECTIVE_PREFIX)) url.searchParams.delete(name);
  }
  return url.href;
}

/** An attribute of a tag line, quoted (`URI="..."`) or not (`RESOLUTION=1280x720`). */
function tagAttribute(line: string, name: string): string | undefined {
  const match = new RegExp(`[:,]${name}=(?:"([^"]*)"|([^,]*))`).exec(line);
  return match ? (match[1] ?? match[2]) : undefined;
}

function resolvedPart(uri: string, base: string): string | undefined {
  try {
    const url = webUrl(new URL(uri, base).href);
    return url && url.href.length <= MAX_URL_LENGTH ? playlistKey(url.href) : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Whether a playlist is a master and, if so, the playlists it lists,
 * resolved against `base` (its address after redirects). Only the URI
 * lines are read: `#EXT-X-STREAM-INF` and the line after it, and the `URI`
 * of `#EXT-X-MEDIA` and `#EXT-X-I-FRAME-STREAM-INF`. Null for anything
 * that is not a playlist.
 */
export function parsePlaylist(text: string, base: string): PlaylistInfo | null {
  const lines = text.replace(/^\uFEFF/, "").split(/\r?\n|\r/);
  if (!lines[0]?.trimEnd().startsWith("#EXTM3U")) return null;
  const parts = new Set<string>();
  const qualities = new Set<string>();
  let master = false;
  // Set after `#EXT-X-STREAM-INF`: the quality its URI line belongs to.
  let quality: string | null = null;
  const add = (uri: string | undefined) => {
    const part = uri ? resolvedPart(uri.trim(), base) : undefined;
    if (part && parts.size < MAX_MASTER_PARTS) parts.add(part);
    return part;
  };
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;
    if (line.startsWith("#EXT-X-STREAM-INF:")) {
      master = true;
      quality = tagAttribute(line, "RESOLUTION") ?? "";
    } else if (line.startsWith("#EXT-X-MEDIA:") || line.startsWith("#EXT-X-I-FRAME-STREAM-INF:")) {
      master = true;
      add(tagAttribute(line, "URI"));
    } else if (!line.startsWith("#") && quality !== null) {
      const part = add(line);
      if (part) qualities.add(quality || part);
      quality = null;
    }
  }
  return master ? { master: true, parts: [...parts], qualities: qualities.size } : { master: false };
}

/**
 * A playlist's text, read until `max` bytes and cut to whole lines. It
 * stops at the first segment (`#EXTINF`): from there on it is a media
 * playlist, which can run to megabytes and has nothing more to say.
 */
export async function readPlaylistText(body: ReadableStream<Uint8Array>, max = MAX_PLAYLIST_BYTES): Promise<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let text = "";
  let bytes = 0;
  const wholeLines = () => text.slice(0, text.lastIndexOf("\n") + 1);
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) return text + decoder.decode();
      const room = max - bytes;
      bytes += value.byteLength;
      if (value.byteLength >= room) {
        text += decoder.decode(value.subarray(0, room));
        return wholeLines();
      }
      text += decoder.decode(value, { stream: true });
      if (text.includes("#EXTINF")) return wholeLines();
    }
  } finally {
    // Stops the download when it ended early.
    void reader.cancel().catch(() => undefined);
  }
}

type FetchLike = (url: string, init: RequestInit) => Promise<Response>;

/**
 * Reads a playlist a page loaded: once, without cookies or a referrer,
 * at most `MAX_PLAYLIST_BYTES` and `PLAYLIST_TIMEOUT_MS`. Null when it
 * could not be read (a playlist that needs the page's cookies, for one).
 */
export async function readPlaylist(
  fetchFn: FetchLike,
  url: string,
  timeoutMs = PLAYLIST_TIMEOUT_MS,
): Promise<PlaylistInfo | null> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchFn(url, {
      credentials: "omit",
      referrerPolicy: "no-referrer",
      redirect: "follow",
      signal: controller.signal,
    });
    if (!response.ok || !response.body) return null;
    return parsePlaylist(await readPlaylistText(response.body), response.url || url);
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

function directory(url: URL): string {
  return url.pathname.slice(0, url.pathname.lastIndexOf("/") + 1);
}

/**
 * Ids in a poster image's address that the video's own media addresses
 * share: path segments of six or more characters that are mostly digits,
 * such as the post id in X's `amplify_video_thumb/2098814416359653377/img/…`
 * and `amplify_video/2098814416359653377/pl/….m3u8`. Sizes (`1280x720`) and
 * random file names are not ids.
 */
export function posterIds(poster: string | undefined): string[] {
  const url = webUrl(poster);
  if (!url) return [];
  return url.pathname
    .split("/")
    .map((segment) => {
      try {
        return decodeURIComponent(segment);
      } catch {
        return segment;
      }
    })
    .filter((s) => {
      if (s.length < 6 || /^\d+x\d+$/.test(s)) return false;
      const digits = s.replace(/\D/g, "").length;
      return digits >= 6 && digits * 2 >= s.length;
    });
}

/**
 * The tab's media that belong to one video: those whose address contains
 * an id from the video's poster (see `posterIds`). A page with several
 * videos (a timeline) lists all of their streams; the Download button over
 * one of them should offer that one. When nothing matches, or the video
 * has no poster, the whole list.
 */
export function mediaForVideo<T extends Pick<MediaCandidate, "url">>(list: readonly T[], poster: string | undefined): T[] {
  const ids = posterIds(poster);
  if (ids.length === 0) return [...list];
  const matched = list.filter((c) => {
    const url = webUrl(c.url);
    return url !== null && url.pathname.split("/").some((segment) => ids.includes(segment));
  });
  return matched.length > 0 ? matched : [...list];
}

/**
 * A playlist below another one of the same site. Some sites (X among
 * them) keep a master's quality and audio playlists in folders under it,
 * so this hides them without reading anything, and still does when the
 * master cannot be read.
 */
export function isPartOf(child: string, parent: string): boolean {
  const c = webUrl(child);
  const p = webUrl(parent);
  if (!c || !p || c.origin !== p.origin) return false;
  const dir = directory(p);
  const childDir = directory(c);
  return childDir.length > dir.length && childDir.startsWith(dir);
}

function withoutFragment(raw: string): string {
  const url = webUrl(raw);
  if (!url) return raw;
  url.hash = "";
  return url.href;
}

/** One tab's media list and the playlists known to belong to a master. */
export interface TabMedia {
  /** Newest first. */
  items: MediaCandidate[];
  /** `playlistKey`s of the quality and audio playlists of masters this tab played, newest first. */
  parts: string[];
}

function isKnownMaster(c: MediaCandidate): boolean {
  return c.kind === "hls" && c.qualities !== undefined;
}

/**
 * Whether a newly seen HLS playlist is worth reading: not listed, not a
 * known part of a master, and not below a listed playlist.
 */
export function shouldReadPlaylist(tab: TabMedia, url: string): boolean {
  const key = playlistKey(url);
  if (tab.parts.includes(key) || tab.items.some((c) => c.url === key)) return false;
  return !tab.items.some((c) => c.kind === "hls" && isPartOf(key, c.url));
}

/**
 * The tab with `found` added, newest first, or null when nothing changed.
 * `playlist` is what reading an HLS playlist showed, when it was read.
 *
 * - The same address seen again changes nothing.
 * - A known part of a master is never listed. A master that was read
 *   adds its parts and drops any already listed: a player can fetch a
 *   quality playlist before its master, or the master only later.
 * - Without that knowledge, playlists in folders below a listed one are
 *   parts of it, both ways round; a master that was read is never hidden
 *   that way.
 */
export function addMedia(
  tab: TabMedia,
  found: Omit<MediaCandidate, "id" | "at">,
  id: string,
  now: number,
  playlist?: PlaylistInfo | null,
): TabMedia | null {
  const hls = found.kind === "hls";
  const url = hls ? playlistKey(found.url) : withoutFragment(found.url);
  if (tab.items.some((c) => c.url === url)) return null;
  if (hls && tab.parts.includes(url)) return null;
  const master = playlist?.master ? playlist : undefined;
  if (hls && !master && tab.items.some((c) => c.kind === "hls" && isPartOf(url, c.url))) return null;

  let { items, parts } = tab;
  if (master) {
    const own = new Set(master.parts);
    parts = [...new Set([...master.parts, ...parts])].slice(0, MAX_TAB_PARTS);
    items = items.filter((c) => !(c.kind === "hls" && own.has(c.url)));
  }
  // A media playlist that was read has no parts; anything else may.
  if (hls && playlist?.master !== false) {
    items = items.filter((c) => !(c.kind === "hls" && !isKnownMaster(c) && isPartOf(c.url, url)));
  }
  // Stored without empty fields, so the session storage entry stays small.
  const entry = Object.fromEntries(
    Object.entries({ ...found, url, id, at: now, qualities: master?.qualities }).filter(([, value]) => value !== undefined),
  ) as unknown as MediaCandidate;
  return { items: [entry, ...items].slice(0, MAX_TAB_MEDIA), parts };
}

const ID_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

export function isMediaId(value: unknown): value is string {
  return typeof value === "string" && ID_PATTERN.test(value);
}

const optionalUrl = (value: unknown) => value === undefined || (typeof value === "string" && webUrl(value) !== null);

/** Whatever was stored for a tab, reduced to well-formed entries. */
export function parseTabMedia(value: unknown): MediaCandidate[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((c): c is MediaCandidate => {
      if (typeof c !== "object" || c === null) return false;
      const v = c as Record<string, unknown>;
      return (
        isMediaId(v.id) &&
        (v.kind === "hls" || v.kind === "dash" || v.kind === "direct") &&
        typeof v.url === "string" &&
        webUrl(v.url) !== null &&
        typeof v.at === "number" &&
        (v.mime === undefined || typeof v.mime === "string") &&
        optionalUrl(v.frameUrl) &&
        optionalUrl(v.referrer) &&
        optionalUrl(v.origin) &&
        (v.size === undefined || (typeof v.size === "number" && v.size >= 0)) &&
        (v.qualities === undefined || (Number.isInteger(v.qualities) && (v.qualities as number) >= 0))
      );
    })
    .slice(0, MAX_TAB_MEDIA);
}

/** Whatever was stored as a tab's playlist parts, reduced to web addresses. */
export function parseTabParts(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((p): p is string => typeof p === "string" && p.length <= MAX_URL_LENGTH && webUrl(p) !== null)
    .slice(0, MAX_TAB_PARTS);
}

/** Storage key of one tab's list in `storage.session`. */
export function tabMediaKey(tabId: number): string {
  return `media:${tabId}`;
}

/** Storage key of one tab's playlist parts; under the same `media:` prefix, so it is forgotten with the list. */
export function tabPartsKey(tabId: number): string {
  return `media:${tabId}:parts`;
}

const KIND_WORDS = {
  hls: "media.hls",
  dash: "media.dash",
} as const satisfies Record<Exclude<MediaKind, "direct">, string>;

const AUDIO_EXTENSIONS = new Set(["mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav", "weba"]);

/** "HLS · 3 qualities" (a master that was read), "HLS stream", "MP4 video", "MP3 audio", "Video". */
export function kindLabel(c: Pick<MediaCandidate, "kind" | "mime" | "url" | "qualities">): string {
  if (c.kind === "hls" && c.qualities) return t("media.hlsQualities", { count: c.qualities });
  if (c.kind !== "direct") return t(KIND_WORDS[c.kind]);
  const ext = mediaExtension(c.url, c.mime) ?? "";
  const audio = c.mime ? c.mime.startsWith("audio/") : AUDIO_EXTENSIONS.has(ext);
  if (ext) return t(audio ? "media.fileAudio" : "media.fileVideo", { format: ext.toUpperCase() });
  return t(audio ? "media.audio" : "media.video");
}

/** The file name in the URL, decoded, or the host. */
export function mediaName(raw: string): string {
  const url = webUrl(raw);
  if (!url) return raw.slice(0, 80);
  const last = baseName(url.pathname);
  let name = last;
  try {
    name = decodeURIComponent(last);
  } catch {
    // Keep the encoded form.
  }
  return (name || url.hostname).slice(0, 80);
}

/**
 * What each row of a tab's list (newest first) is called: the page's
 * title, which is also what Downpipe names the download, numbered from the
 * oldest entry when several rows share it (a feed of videos). Playlist file
 * names are often random (`8k4_FZTUGeXpWu9v.m3u8`), so they are only used
 * when the page has no title.
 */
export function mediaLabels(items: readonly Pick<MediaCandidate, "url">[], title: string | undefined): string[] {
  const clean = cleanTitle(title);
  if (!clean) return items.map((c) => mediaName(c.url));
  return items.map((_, i) => {
    const n = items.length - i;
    return n === 1 ? clean : `${clean} (${n})`;
  });
}

const MAX_TITLE_LENGTH = 200;

/**
 * A page title the host accepts as a name for the download: whitespace
 * collapsed, control characters removed, capped. Undefined when nothing
 * is left, or only a site's short name: X titles its tab "X" until the
 * post's text arrives, and "X.mp4" would name every video the same.
 */
export function cleanTitle(raw: string | undefined): string | undefined {
  if (!raw) return undefined;
  let out = "";
  for (const ch of raw) {
    const code = ch.codePointAt(0) ?? 0;
    out += code < 0x20 || (code >= 0x7f && code <= 0x9f) ? " " : ch;
  }
  const title = out.replace(/\s+/g, " ").trim();
  return [...title].length >= 3 ? [...title].slice(0, MAX_TITLE_LENGTH).join("") : undefined;
}

const FILE_NAME_UNSAFE = /[<>:"/\\|?*]/g;

/**
 * File name for a direct media file named after the page: the title made
 * safe for file systems plus the file's own extension. Undefined without
 * a title or an extension.
 */
export function titleFileName(title: string | undefined, c: Pick<MediaCandidate, "url" | "mime">): string | undefined {
  const clean = cleanTitle(title)?.replace(FILE_NAME_UNSAFE, " ").replace(/\s+/g, " ").trim();
  const ext = mediaExtension(c.url, c.mime);
  if (!clean || !ext) return undefined;
  const stem = [...clean].slice(0, 120).join("").replace(/[. ]+$/, "");
  return stem ? `${stem}.${ext}` : undefined;
}
