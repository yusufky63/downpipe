// The Add dialog's request headers: one "Name: value" per line. They go to
// the core with the download and stay in memory only, like cookies in them.

/** A request header as the core takes it: `[name, value]`. */
export type Header = [string, string];

/**
 * Headers the engine sets itself (ranges, compression, the connection),
 * the same list as `FORBIDDEN_HEADERS` in crates/http-engine/src/http.rs.
 */
export const MANAGED_HEADERS: ReadonlySet<string> = new Set([
  "range",
  "if-range",
  "accept-encoding",
  "host",
  "content-length",
  "transfer-encoding",
  "connection",
  "keep-alive",
  "proxy-connection",
  "te",
  "trailer",
  "upgrade",
]);

/** RFC 9110's token characters, which a header name is made of. */
const TOKEN = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;

export type HeaderProblem =
  /** No colon, or nothing before it. */
  | { line: number; kind: "format" }
  | { line: number; kind: "name"; name: string }
  | { line: number; kind: "managed"; name: string };

export interface HeaderParse {
  headers: Header[];
  problems: HeaderProblem[];
}

/** Whether `name` is a header the user may set. */
export function settableHeader(name: string): boolean {
  return TOKEN.test(name) && !MANAGED_HEADERS.has(name.toLowerCase());
}

/**
 * The field's text as headers. Blank lines are skipped; `line` counts from
 * 1 so a message can point at it. A value may contain colons (`Referer:
 * https://…`); only the first one separates.
 */
export function parseHeaders(text: string): HeaderParse {
  const headers: Header[] = [];
  const problems: HeaderProblem[] = [];
  text.split(/\r?\n/).forEach((raw, i) => {
    const line = i + 1;
    if (raw.trim() === "") return;
    const colon = raw.indexOf(":");
    const name = colon > 0 ? raw.slice(0, colon).trim() : "";
    if (!name) {
      problems.push({ line, kind: "format" });
      return;
    }
    if (!TOKEN.test(name)) {
      problems.push({ line, kind: "name", name });
      return;
    }
    if (MANAGED_HEADERS.has(name.toLowerCase())) {
      problems.push({ line, kind: "managed", name });
      return;
    }
    headers.push([name, raw.slice(colon + 1).trim()]);
  });
  return { headers, problems };
}

/** Headers back as the field's text. */
export function headersText(headers: readonly Header[]): string {
  return headers.map(([name, value]) => `${name}: ${value}`).join("\n");
}
