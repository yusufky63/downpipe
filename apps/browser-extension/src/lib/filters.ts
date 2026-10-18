// Which browser downloads capture hands to Downpipe: file types, sites to
// leave alone and a minimum size, like IDM's "File types" settings. Pure
// functions, shared by the service worker (matching) and the options page
// (parsing what the user typed).
import { t } from "../i18n";
import { baseName, webUrl } from "./url";

export interface CaptureFilters {
  /** Every file type is captured; `captureTypes` is kept for when this is switched off. */
  captureAll: boolean;
  /** Lower-case extensions without the dot (`zip`, `tar.gz`). Empty also means every type. */
  captureTypes: string[];
  /** `example.com` or `*.example.com`, lower case, IDN in ASCII (punycode) form. */
  excludedSites: string[];
  /** 0 turns the limit off. */
  minSizeBytes: number;
}

export const MAX_CAPTURE_TYPES = 300;
export const MAX_EXCLUDED_SITES = 500;
export const MAX_TYPE_LENGTH = 16;
/** 1 TiB: anything above is a typo, not a filter. */
export const MAX_MIN_SIZE_BYTES = 1024 ** 4;

/**
 * Close to IDM's default list: archives, programs and packages, disk
 * images, documents, audio and video. Web pages, images and scripts stay in
 * the browser, which handles them better than a download manager.
 */
export const DEFAULT_CAPTURE_TYPES: readonly string[] = [
  // Archives
  "7z", "zip", "rar", "tar", "gz", "tgz", "bz2", "xz", "zst", "lz", "lzh", "arj", "cab", "z",
  // Programs and packages
  "exe", "msi", "msix", "msu", "appx", "apk", "dmg", "pkg", "deb", "rpm", "appimage", "jar",
  // Disk images
  "iso", "img", "bin", "vhd", "vhdx", "vmdk",
  // Documents
  "pdf", "epub", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pps", "odt", "ods", "odp",
  // Audio
  "mp3", "m4a", "aac", "flac", "ogg", "opus", "wav", "wma", "aif", "aiff",
  // Video
  "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpg", "mpeg", "3gp", "ogv",
];

/** No filtering at all: what capture did before filters existed. */
export const CAPTURE_EVERYTHING: CaptureFilters = {
  captureAll: true,
  captureTypes: [],
  excludedSites: [],
  minSizeBytes: 0,
};

export function defaultFilters(): CaptureFilters {
  return { captureAll: false, captureTypes: [...DEFAULT_CAPTURE_TYPES], excludedSites: [], minSizeBytes: 0 };
}

// ---------- File types ----------

const TYPE_PATTERN = /^[a-z0-9]+(?:\.[a-z0-9]+)*$/;

/**
 * `zip`, `.ZIP` and `*.zip` all become `zip`; null for anything that is not
 * an extension. Inner dots are allowed so `tar.gz` can be listed on its own.
 */
export function normalizeType(raw: string): string | null {
  let t = raw.trim().toLowerCase();
  if (t.startsWith("*.")) t = t.slice(2);
  else if (t.startsWith(".")) t = t.slice(1);
  if (!t || t.length > MAX_TYPE_LENGTH || !TYPE_PATTERN.test(t)) return null;
  return t;
}

export interface ListParse {
  values: string[];
  errors: string[];
}

/**
 * The options page's file type field: separated by spaces, commas or
 * semicolons. `max` is the list's limit (the media list has its own).
 */
export function parseTypeList(text: string, max = MAX_CAPTURE_TYPES): ListParse {
  const values: string[] = [];
  const errors: string[] = [];
  for (const token of text.split(/[\s,;]+/)) {
    if (!token) continue;
    const type = normalizeType(token);
    if (!type) {
      errors.push(t("options.errors.notExtension", { value: token.slice(0, 40) }));
      continue;
    }
    if (!values.includes(type)) values.push(type);
  }
  if (values.length > max) errors.push(t("options.errors.tooManyTypes", { count: max }));
  return { values, errors };
}

/**
 * The name capture judges a download by: the browser's file name (on
 * Chrome the server's `Content-Disposition` name is already applied), else
 * the last segment of the URL path. Query strings never count, so
 * `get.php?file=a.zip` is a `php` download.
 */
export function downloadName(filename: string | undefined, url: string): string {
  const fromBrowser = filename ? baseName(filename).trim() : "";
  if (fromBrowser) return fromBrowser;
  const parsed = webUrl(url);
  if (!parsed) return "";
  const last = baseName(parsed.pathname);
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

/** True when the name ends in `.<type>` for a listed type, ignoring case. */
export function typeListed(name: string, types: readonly string[]): boolean {
  const lower = name.toLowerCase();
  return types.some((type) => lower.length > type.length + 1 && lower.endsWith(`.${type}`));
}

// ---------- Sites ----------

const IPV4 = /^\d{1,3}(?:\.\d{1,3}){3}$/;

/** A URL's host the way site patterns store it: lower case, punycode, no trailing dot. */
export function hostOf(raw: string | undefined): string | null {
  const url = webUrl(raw);
  if (!url) return null;
  return url.hostname.replace(/\.$/, "") || null;
}

/**
 * `example.com` or `*.example.com` in the stored form, or null.
 *
 * Accepts what people paste: a full http(s) URL, a host with a port or a
 * path, upper case and Unicode host names. Ports and paths are dropped, as
 * filters are per site. `*` is only allowed as the whole first label.
 */
export function normalizeSitePattern(raw: string): string | null {
  let text = raw.trim().toLowerCase();
  if (!text || text.length > 300) return null;
  if (text.includes("://")) {
    if (text.includes("*")) return null;
    return hostOf(text);
  }
  const wildcard = text.startsWith("*.");
  if (wildcard) text = text.slice(2);
  // Userinfo would make the URL parser below read a different host.
  if (!text || text.includes("*") || text.includes("@") || /[\s\\]/.test(text)) return null;
  const host = hostOf(`http://${text}`);
  if (!host) return null;
  if (wildcard && (IPV4.test(host) || host.startsWith("["))) return null;
  return wildcard ? `*.${host}` : host;
}

/** `*.example.com` covers example.com itself and every subdomain. */
export function hostMatches(host: string, pattern: string): boolean {
  if (!pattern.startsWith("*.")) return host === pattern;
  const base = pattern.slice(2);
  return host === base || host.endsWith(`.${base}`);
}

/** The first pattern that covers any of the hosts, or undefined. */
export function matchingSite(hosts: readonly (string | null)[], patterns: readonly string[]): string | undefined {
  for (const host of hosts) {
    if (!host) continue;
    const hit = patterns.find((p) => hostMatches(host, p));
    if (hit) return hit;
  }
  return undefined;
}

/** The options page's site list: one per line, blank lines ignored. */
export function parseSiteList(text: string): ListParse {
  const values: string[] = [];
  const errors: string[] = [];
  text.split(/\r?\n/).forEach((line, index) => {
    if (!line.trim()) return;
    const site = normalizeSitePattern(line);
    if (!site) {
      // A string, so a line number is not written with a thousands separator.
      errors.push(t("options.errors.notSite", { line: String(index + 1), value: line.trim().slice(0, 60) }));
      return;
    }
    if (!values.includes(site)) values.push(site);
  });
  if (values.length > MAX_EXCLUDED_SITES) errors.push(t("options.errors.tooManySites", { count: MAX_EXCLUDED_SITES }));
  return { values, errors };
}

/** Adds or removes one exact entry, leaving the other patterns alone. */
export function withSite(patterns: readonly string[], site: string, excluded: boolean): string[] {
  const rest = patterns.filter((p) => p !== site);
  return excluded ? [...rest, site].slice(-MAX_EXCLUDED_SITES) : rest;
}

// ---------- Size ----------

export const SIZE_UNITS = { KiB: 1024, MiB: 1024 ** 2 } as const;
export type SizeUnit = keyof typeof SIZE_UNITS;

export type SizeParse = { ok: true; bytes: number } | { ok: false; error: string };

/** The options page's minimum size: empty or 0 is off; decimals are fine. */
export function parseMinSize(text: string, unit: SizeUnit): SizeParse {
  const value = text.trim();
  if (!value) return { ok: true, bytes: 0 };
  if (!/^\d+(?:\.\d+)?$/.test(value)) return { ok: false, error: t("options.errors.sizeNotNumber") };
  const bytes = Math.round(Number(value) * SIZE_UNITS[unit]);
  if (bytes > MAX_MIN_SIZE_BYTES) return { ok: false, error: t("options.errors.sizeTooBig") };
  return { ok: true, bytes };
}

/** The stored size as the options page shows it. */
export function sizeFields(bytes: number): { value: string; unit: SizeUnit } {
  if (bytes <= 0) return { value: "0", unit: "MiB" };
  const unit: SizeUnit = bytes >= SIZE_UNITS.MiB ? "MiB" : "KiB";
  return { value: String(Number((bytes / SIZE_UNITS[unit]).toFixed(2))), unit };
}

/**
 * The download's size if the browser knows it. Chrome and Firefox report -1
 * (and Chrome sometimes 0) while there is no `Content-Length`, which must
 * not count as "small".
 */
export function knownSize(totalBytes: number | undefined, fileSize: number | undefined): number | undefined {
  for (const n of [totalBytes, fileSize]) {
    if (typeof n === "number" && Number.isFinite(n) && n > 0) return n;
  }
  return undefined;
}

// ---------- Stored data ----------

function storedList(value: unknown, normalize: (s: string) => string | null, max: number): string[] | null {
  if (!Array.isArray(value)) return null;
  const out: string[] = [];
  for (const entry of value) {
    if (typeof entry !== "string") continue;
    const v = normalize(entry);
    if (v && !out.includes(v)) out.push(v);
    if (out.length >= max) break;
  }
  return out;
}

/**
 * Filters from `storage.local`. Fields that are missing (settings saved by
 * an older version) or malformed get their defaults; invalid list entries
 * are dropped one by one rather than throwing the whole list away.
 */
export function parseFilters(v: Record<string, unknown>): CaptureFilters {
  const defaults = defaultFilters();
  const size = v.minSizeBytes;
  return {
    captureAll: typeof v.captureAll === "boolean" ? v.captureAll : defaults.captureAll,
    captureTypes: storedList(v.captureTypes, normalizeType, MAX_CAPTURE_TYPES) ?? defaults.captureTypes,
    excludedSites: storedList(v.excludedSites, normalizeSitePattern, MAX_EXCLUDED_SITES) ?? defaults.excludedSites,
    minSizeBytes:
      typeof size === "number" && Number.isInteger(size) && size >= 0 && size <= MAX_MIN_SIZE_BYTES
        ? size
        : defaults.minSizeBytes,
  };
}

/**
 * Strict check for filters a page wants saved: every value must already be
 * in its stored form, since the options page normalizes before sending.
 */
export function validFilters(value: unknown): CaptureFilters | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  if (Object.keys(v).length !== 4) return null;
  const { captureAll, captureTypes, excludedSites, minSizeBytes } = v;
  if (typeof captureAll !== "boolean") return null;
  const exact = (list: unknown, normalize: (s: string) => string | null, max: number): list is string[] =>
    Array.isArray(list) &&
    list.length <= max &&
    new Set(list).size === list.length &&
    list.every((s) => typeof s === "string" && normalize(s) === s);
  if (!exact(captureTypes, normalizeType, MAX_CAPTURE_TYPES)) return null;
  if (!exact(excludedSites, normalizeSitePattern, MAX_EXCLUDED_SITES)) return null;
  if (
    typeof minSizeBytes !== "number" ||
    !Number.isInteger(minSizeBytes) ||
    minSizeBytes < 0 ||
    minSizeBytes > MAX_MIN_SIZE_BYTES
  ) {
    return null;
  }
  return { captureAll, captureTypes: [...captureTypes], excludedSites: [...excludedSites], minSizeBytes };
}

// ---------- The options form ----------

export interface FilterForm {
  captureAll: boolean;
  typesText: string;
  sitesText: string;
  sizeText: string;
  unit: SizeUnit;
}

export interface FormErrors {
  types: string[];
  sites: string[];
  size: string[];
}

export type FormResult = { ok: true; filters: CaptureFilters } | { ok: false; errors: FormErrors };

/** Everything the options page needs to save, or what to say under each field. */
export function filtersFromForm(form: FilterForm): FormResult {
  const types = parseTypeList(form.typesText);
  const sites = parseSiteList(form.sitesText);
  const size = parseMinSize(form.sizeText, form.unit);
  // With "Capture all file types" on, the list is greyed out and only kept
  // for later, so a bad entry there must not block saving; it is dropped.
  const errors: FormErrors = {
    types: form.captureAll ? [] : [...types.errors],
    sites: sites.errors,
    size: size.ok ? [] : [size.error],
  };
  // An empty list would quietly mean "everything"; say so instead.
  if (!form.captureAll && !types.errors.length && !types.values.length) {
    errors.types.push(t("options.errors.noTypes"));
  }
  if (errors.types.length || errors.sites.length || !size.ok) return { ok: false, errors };
  return {
    ok: true,
    filters: {
      captureAll: form.captureAll,
      captureTypes: types.values.slice(0, MAX_CAPTURE_TYPES),
      excludedSites: sites.values,
      minSizeBytes: size.bytes,
    },
  };
}
