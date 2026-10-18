// "Paste as cURL": a command copied from the browser's developer tools
// (Network, Copy as cURL) turned into the Add dialog's link, headers and
// file name. Only GET requests can be replayed as a download, so anything
// that sends data or uses another method is refused rather than changed.
//
// Both of the browsers' formats are read: bash (`'...'`, `$'...'`, `\` at a
// line's end) and Windows cmd (`^"...^"`, `^` escapes, `^` at a line's end).
import { MANAGED_HEADERS, type Header } from "./headers";

export interface CurlRequest {
  url: string;
  /** In the command's order; cookies (`-b`, `Cookie:`) joined into one header. */
  headers: Header[];
  /** From `-o name`, when the command saves to a file. */
  fileName: string | null;
  /** Options and headers that were left out, as written in the command. */
  ignored: string[];
}

export type CurlError =
  | { kind: "not_curl" }
  /** A quote that never closes: the command was cut off. */
  | { kind: "quote" }
  | { kind: "no_url" }
  | { kind: "method"; method: string }
  /** The command sends a body (`-d`, `-F`, `--json`, `-T`): a POST or PUT. */
  | { kind: "data"; option: string };

export type CurlParse = { ok: true; request: CurlRequest } | { ok: false; error: CurlError };

/** Options whose next word is their value, which is skipped. */
const IGNORED_WITH_VALUE = new Set([
  "-x", "--proxy", "-U", "--proxy-user", "--proxy-header", "-m", "--max-time", "--connect-timeout",
  "-r", "--range", "--retry", "--retry-delay", "--retry-max-time", "-w", "--write-out", "-c",
  "--cookie-jar", "-D", "--dump-header", "-E", "--cert", "--cert-type", "--cacert", "--capath",
  "--key", "--key-type", "--pass", "--limit-rate", "-Y", "--speed-limit", "-y", "--speed-time",
  "-z", "--time-cond", "-C", "--continue-at", "-K", "--config", "--resolve", "--connect-to",
  "--interface", "--max-redirs", "--ciphers", "--tls-max", "--dns-servers", "--local-port",
  "--expect100-timeout", "--keepalive-time", "--max-filesize", "--output-dir", "--trace",
  "--trace-ascii", "--stderr", "--aws-sigv4", "--variable", "--rate", "--unix-socket",
  "--abstract-unix-socket", "--happy-eyeballs-timeout-ms", "--ip-tos", "--parallel-max",
  "--retry-all-errors-max", "--noproxy", "--preproxy", "--socks5", "--socks5-hostname",
]);

/** Options that change nothing for a download, left out without a word. */
const SILENT = new Set([
  "--compressed", "-L", "--location", "-s", "--silent", "-S", "--show-error", "-g", "--globoff",
  "--http1.1", "--http2", "--http2-prior-knowledge", "--http3", "-f", "--fail", "--fail-with-body",
  "-v", "--verbose", "-i", "--include", "-O", "--remote-name", "-J", "--remote-header-name",
  "-#", "--progress-bar", "-N", "--no-buffer", "--location-trusted",
]);

const DATA_OPTIONS = new Set([
  "-d", "--data", "--data-raw", "--data-binary", "--data-ascii", "--data-urlencode",
]);
/** A body that a GET cannot carry even with `-G`. */
const BODY_OPTIONS = new Set(["-F", "--form", "--form-string", "--json", "-T", "--upload-file"]);

/** Short options that take a value, which may follow without a space (`-HAccept: x`). */
const SHORT_WITH_VALUE = new Set(["H", "b", "A", "e", "u", "X", "o", "d", "F", "T", "x", "U", "m", "r", "w", "c", "D", "E", "Y", "y", "z", "C", "K"]);

const LONG_ALIASES: Record<string, string> = {
  "--header": "-H",
  "--cookie": "-b",
  "--user-agent": "-A",
  "--referer": "-e",
  "--user": "-u",
  "--request": "-X",
  "--output": "-o",
  "--get": "-G",
  "--head": "-I",
};

/**
 * Words of the command line. Null when a quote is left open. In cmd form,
 * `^` escapes the next character first (and joins lines), which leaves the
 * double-quoted, backslash-escaped words Chrome and Firefox write.
 */
export function shellWords(text: string): string[] | null {
  const cmd = /\^["\r\n]/.test(text);
  const src = cmd ? text.replace(/\^(\r?\n|[\s\S])/g, (_, c: string) => (c.startsWith("\n") || c.startsWith("\r") ? " " : c)) : text;
  const words: string[] = [];
  let word = "";
  let inWord = false;
  let i = 0;
  const end = () => {
    if (inWord) words.push(word);
    word = "";
    inWord = false;
  };
  while (i < src.length) {
    const c = src[i] as string;
    if (/\s/.test(c)) {
      end();
      i += 1;
    } else if (c === "\\" && !cmd) {
      const next = src[i + 1];
      if (next === "\n") i += 2;
      else if (next === "\r" && src[i + 2] === "\n") i += 3;
      else if (next === undefined) i += 1;
      else {
        word += next;
        inWord = true;
        i += 2;
      }
    } else if (c === "'" && !cmd) {
      const close = src.indexOf("'", i + 1);
      if (close < 0) return null;
      word += src.slice(i + 1, close);
      inWord = true;
      i = close + 1;
    } else if (c === "$" && src[i + 1] === "'" && !cmd) {
      const read = ansiC(src, i + 2);
      if (read === null) return null;
      word += read.text;
      inWord = true;
      i = read.next;
    } else if (c === '"') {
      i += 1;
      let closed = false;
      while (i < src.length) {
        const d = src[i] as string;
        if (d === '"') {
          closed = true;
          i += 1;
          break;
        }
        const next = src[i + 1];
        if (d === "\\" && next !== undefined && (next === '"' || next === "\\" || (!cmd && (next === "$" || next === "`")))) {
          word += next;
          i += 2;
        } else if (d === "\\" && next === "\n" && !cmd) {
          i += 2;
        } else {
          word += d;
          i += 1;
        }
      }
      if (!closed) return null;
      inWord = true;
    } else {
      word += c;
      inWord = true;
      i += 1;
    }
  }
  end();
  return words;
}

/** Bash's `$'...'` from just after the opening quote. */
function ansiC(src: string, start: number): { text: string; next: number } | null {
  let text = "";
  let i = start;
  const simple: Record<string, string> = { n: "\n", t: "\t", r: "\r", "\\": "\\", "'": "'", '"': '"', a: "\x07", b: "\b", e: "\x1b", f: "\f", v: "\v", "?": "?" };
  while (i < src.length) {
    const c = src[i] as string;
    if (c === "'") return { text, next: i + 1 };
    if (c !== "\\") {
      text += c;
      i += 1;
      continue;
    }
    const e = src[i + 1];
    if (e === undefined) return null;
    if (e in simple) {
      text += simple[e];
      i += 2;
    } else if (e === "x" || e === "u" || e === "U") {
      const max = e === "x" ? 2 : e === "u" ? 4 : 8;
      const hex = /^[0-9a-fA-F]+/.exec(src.slice(i + 2, i + 2 + max))?.[0] ?? "";
      if (hex) text += String.fromCodePoint(Number.parseInt(hex, 16));
      else text += `\\${e}`;
      i += 2 + hex.length;
    } else if (/[0-7]/.test(e)) {
      const oct = /^[0-7]{1,3}/.exec(src.slice(i + 1))?.[0] ?? e;
      text += String.fromCharCode(Number.parseInt(oct, 8));
      i += 1 + oct.length;
    } else {
      text += `\\${e}`;
      i += 2;
    }
  }
  return null;
}

function isCurl(word: string | undefined): boolean {
  if (!word) return false;
  const base = word.split(/[\\/]/).pop()?.toLowerCase() ?? "";
  return base === "curl" || base === "curl.exe";
}

function webLink(raw: string): string | null {
  try {
    const url = new URL(raw);
    return url.protocol === "http:" || url.protocol === "https:" ? url.href : null;
  } catch {
    return null;
  }
}

function basicAuth(user: string): string {
  const bytes = new TextEncoder().encode(user);
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return `Basic ${btoa(binary)}`;
}

function withQuery(url: string, data: readonly string[]): string {
  if (data.length === 0) return url;
  const u = new URL(url);
  const extra = data.join("&");
  u.search = u.search ? `${u.search}&${extra}` : `?${extra}`;
  return u.href;
}

/** `--data-urlencode`'s forms: `content`, `=content`, `name=content`, as curl encodes them. */
function urlencoded(value: string): string {
  const eq = value.indexOf("=");
  if (eq < 0) return encodeURIComponent(value);
  const name = value.slice(0, eq);
  const content = encodeURIComponent(value.slice(eq + 1));
  return name ? `${name}=${content}` : content;
}

export function parseCurl(text: string): CurlParse {
  const words = shellWords(text.trim());
  if (words === null) return { ok: false, error: { kind: "quote" } };
  if (!isCurl(words[0])) return { ok: false, error: { kind: "not_curl" } };

  let method: string | null = null;
  let get = false;
  let fileName: string | null = null;
  const headers: Header[] = [];
  const cookies: string[] = [];
  const data: string[] = [];
  let dataOption: string | null = null;
  const ignored: string[] = [];

  const header = (name: string, value: string) => {
    const lower = name.toLowerCase();
    if (lower === "cookie") {
      if (value) cookies.push(value);
      return;
    }
    if (MANAGED_HEADERS.has(lower)) {
      ignored.push(name);
      return;
    }
    // Replaces what an earlier option set, as curl does.
    const at = headers.findIndex(([n]) => n.toLowerCase() === lower);
    if (at >= 0) headers.splice(at, 1);
    headers.push([name, value]);
  };

  // Each option becomes its canonical name and value; flags have no value.
  const options: { name: string; value: string | null }[] = [];
  const positional: string[] = [];
  for (let i = 1; i < words.length; i += 1) {
    const w = words[i] as string;
    if (w === "--") {
      positional.push(...words.slice(i + 1));
      break;
    }
    if (w.startsWith("--")) {
      const name = LONG_ALIASES[w] ?? w;
      const takes =
        name === "--url" ||
        DATA_OPTIONS.has(name) ||
        BODY_OPTIONS.has(name) ||
        IGNORED_WITH_VALUE.has(name) ||
        name === "--oauth2-bearer" ||
        (name.startsWith("-") && name.length === 2 && SHORT_WITH_VALUE.has(name.slice(1)));
      if (takes) {
        options.push({ name, value: words[i + 1] ?? "" });
        i += 1;
      } else {
        options.push({ name, value: null });
      }
      continue;
    }
    if (w.startsWith("-") && w.length > 1) {
      // Bundled short options (`-sSL`); one that takes a value ends the bundle.
      for (let j = 1; j < w.length; j += 1) {
        const letter = w[j] as string;
        if (SHORT_WITH_VALUE.has(letter)) {
          const rest = w.slice(j + 1);
          if (rest) options.push({ name: `-${letter}`, value: rest });
          else {
            options.push({ name: `-${letter}`, value: words[i + 1] ?? "" });
            i += 1;
          }
          break;
        }
        options.push({ name: `-${letter}`, value: null });
      }
      continue;
    }
    positional.push(w);
  }

  for (const { name, value } of options) {
    const v = value ?? "";
    switch (name) {
      case "-H": {
        const colon = v.indexOf(":");
        // `Name;` sends an empty header and `Name:` removes one: neither matters here.
        if (colon > 0 && v.slice(colon + 1).trim()) header(v.slice(0, colon).trim(), v.slice(colon + 1).trim());
        break;
      }
      case "-b":
        // Without `=` it names a cookie file, which is not in the command.
        if (v.includes("=")) cookies.push(v);
        else ignored.push(name);
        break;
      case "-A":
        header("User-Agent", v);
        break;
      case "-e":
        // `;auto` is curl's own: the referer of a redirect.
        if (v.replace(/;auto$/, "")) header("Referer", v.replace(/;auto$/, ""));
        break;
      case "-u":
        header("Authorization", basicAuth(v));
        break;
      case "--oauth2-bearer":
        header("Authorization", `Bearer ${v}`);
        break;
      case "-X":
        method = v.toUpperCase();
        break;
      case "-I":
        method = "HEAD";
        break;
      case "-G":
        get = true;
        break;
      case "-o": {
        const base = v.split(/[\\/]/).pop() ?? "";
        if (v !== "-" && base && base !== "null") fileName = base;
        break;
      }
      case "--url":
        positional.unshift(v);
        break;
      default:
        if (DATA_OPTIONS.has(name)) {
          dataOption ??= name;
          data.push(name === "--data-urlencode" ? urlencoded(v) : v);
        } else if (BODY_OPTIONS.has(name)) {
          return { ok: false, error: { kind: "data", option: name } };
        } else if (!SILENT.has(name)) {
          ignored.push(name);
        }
    }
  }

  if (method !== null && method !== "GET") return { ok: false, error: { kind: "method", method } };
  if (dataOption !== null && !get) return { ok: false, error: { kind: "data", option: dataOption } };
  const link = positional.map(webLink).find((u) => u !== null) ?? null;
  if (link === null) return { ok: false, error: { kind: "no_url" } };
  if (cookies.length > 0) headers.push(["Cookie", cookies.join("; ")]);
  return {
    ok: true,
    request: { url: withQuery(link, data), headers, fileName, ignored: [...new Set(ignored)] },
  };
}

/** A pasted text that is a cURL command rather than links. */
export function looksLikeCurl(text: string): boolean {
  return /^\s*curl(\.exe)?\s/i.test(text);
}
