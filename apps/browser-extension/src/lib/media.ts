// Media detection: which responses a page received are streams or media
// files worth offering, and the per-tab list of them. Pure functions; the
// service worker feeds them `webRequest` events while detection is on.
//
// The approach is the one IDM's download panel and open-source sniffers
// (cat-catch, Stream Detector) take: classify each response by its
// Content-Type and URL, keep what the page's own request sent as Referer
// and Origin, and list the result per tab. No code is taken from them.
import { normalizeType, parseTypeList, type ListParse } from "./filters";
import { mimeEssence } from "./protocol";
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
  return { mediaOverlay: true, mediaTypes: [...DEFAULT_MEDIA_TYPES], mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES };
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
    mediaTypes: types,
    mediaMinBytes:
      typeof size === "number" && Number.isInteger(size) && size >= 0 && size <= MAX_MEDIA_MIN_BYTES ? size : d.mediaMinBytes,
  };
}

/** Strict check for what the options page saves: already normalized, exact keys. */
export function validMediaFilters(value: unknown): MediaFilters | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  if (Object.keys(v).length !== 3) return null;
  const { mediaOverlay, mediaTypes, mediaMinBytes } = v;
  if (typeof mediaOverlay !== "boolean") return null;
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
  return { mediaOverlay, mediaTypes: [...mediaTypes], mediaMinBytes };
}

/** The options page's media type field. Unlike capture, an empty list is allowed: only streams are listed then. */
export function parseMediaTypeList(text: string): ListParse {
  const { values, errors } = parseTypeList(text);
  // The capture list's own limit does not apply here; this one does.
  const own = errors.filter((e) => !e.startsWith("List at most"));
  if (values.length > MAX_MEDIA_TYPES) own.push(`List at most ${MAX_MEDIA_TYPES} file types.`);
  return { values, errors: own };
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

function directory(url: URL): string {
  return url.pathname.slice(0, url.pathname.lastIndexOf("/") + 1);
}

/**
 * A playlist below another one of the same site: players load the master
 * playlist first and then the quality and audio playlists it lists, which
 * usually sit in folders under it. Those are parts of the stream already
 * listed, and Downpipe picks the quality from the master itself.
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

/**
 * The tab's list with `found` added, newest first, or null when nothing
 * changed: the same URL seen again, or a playlist that is part of one
 * already listed.
 */
export function addCandidate(
  list: readonly MediaCandidate[],
  found: Omit<MediaCandidate, "id" | "at">,
  id: string,
  now: number,
): MediaCandidate[] | null {
  const url = withoutFragment(found.url);
  if (list.some((c) => c.url === url)) return null;
  if (found.kind === "hls" && list.some((c) => c.kind === "hls" && isPartOf(url, c.url))) return null;
  // Stored without empty fields, so the session storage entry stays small.
  const entry = Object.fromEntries(
    Object.entries({ ...found, url, id, at: now }).filter(([, value]) => value !== undefined),
  ) as unknown as MediaCandidate;
  return [entry, ...list].slice(0, MAX_TAB_MEDIA);
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
        (v.size === undefined || (typeof v.size === "number" && v.size >= 0))
      );
    })
    .slice(0, MAX_TAB_MEDIA);
}

/** Storage key of one tab's list in `storage.session`. */
export function tabMediaKey(tabId: number): string {
  return `media:${tabId}`;
}

const KIND_WORDS: Record<Exclude<MediaKind, "direct">, string> = {
  hls: "HLS stream",
  dash: "DASH stream",
};

const AUDIO_EXTENSIONS = new Set(["mp3", "m4a", "aac", "ogg", "oga", "opus", "flac", "wav", "weba"]);

/** "HLS stream", "MP4 video", "MP3 audio", "Video". */
export function kindLabel(c: Pick<MediaCandidate, "kind" | "mime" | "url">): string {
  if (c.kind !== "direct") return KIND_WORDS[c.kind];
  const ext = mediaExtension(c.url, c.mime) ?? "";
  const audio = c.mime ? c.mime.startsWith("audio/") : AUDIO_EXTENSIONS.has(ext);
  const word = audio ? "audio" : "video";
  return ext ? `${ext.toUpperCase()} ${word}` : audio ? "Audio" : "Video";
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

const MAX_TITLE_LENGTH = 200;

/**
 * A page title the host accepts as a name for the download: whitespace
 * collapsed, control characters removed, capped. Undefined when nothing
 * is left.
 */
export function cleanTitle(raw: string | undefined): string | undefined {
  if (!raw) return undefined;
  let out = "";
  for (const ch of raw) {
    const code = ch.codePointAt(0) ?? 0;
    out += code < 0x20 || (code >= 0x7f && code <= 0x9f) ? " " : ch;
  }
  const title = out.replace(/\s+/g, " ").trim();
  return title ? [...title].slice(0, MAX_TITLE_LENGTH).join("") : undefined;
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
