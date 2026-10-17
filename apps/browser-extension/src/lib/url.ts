/** Matches `MAX_URL_LEN` in crates/native-host/src/protocol.rs. */
export const MAX_URL_LENGTH = 8 * 1024;

function hasControlCharacters(text: string): boolean {
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    if (code < 0x20 || (code >= 0x7f && code <= 0x9f)) return true;
  }
  return false;
}

/**
 * Parse an http(s) URL the way the host will accept it, or return null.
 * Control characters are refused rather than left to the URL parser, which
 * silently drops tabs and newlines.
 */
export function webUrl(raw: string | undefined | null): URL | null {
  if (!raw || raw.length > MAX_URL_LENGTH || hasControlCharacters(raw)) return null;
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return null;
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") return null;
  if (!url.hostname) return null;
  return url;
}

/** What the Fetch standard strips before a URL is used as a referrer. */
export function withoutSecrets(url: URL): URL {
  const copy = new URL(url.href);
  copy.username = "";
  copy.password = "";
  copy.hash = "";
  return copy;
}

/**
 * The referrer a browser sends under its default policy,
 * `strict-origin-when-cross-origin`: the full page URL to the same origin,
 * only the origin to other sites, nothing from https to http. Pages can set
 * a different policy, which the extension cannot see; erring towards less
 * is the private choice.
 */
export function referrerFor(pageUrl: string | undefined, targetUrl: string): string | undefined {
  const page = webUrl(pageUrl);
  const target = webUrl(targetUrl);
  if (!page || !target) return undefined;
  if (page.protocol === "https:" && target.protocol === "http:") return undefined;
  if (page.origin === target.origin) return withoutSecrets(page).href;
  return `${page.origin}/`;
}

/** Last path component of a Windows or POSIX path. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? "";
}

/** A short label for a link in the activity list: its file name or host. */
export function linkLabel(raw: string): string {
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
