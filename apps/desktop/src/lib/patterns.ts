// Batch links like IDM's "Add batch download": `file[001-120].jpg`,
// `part[a-f].rar`. Several ranges in one link multiply.

/** More than this many links from one paste is almost certainly a typo. */
export const MAX_EXPANDED = 1000;

const RANGE = /\[(\d+)-(\d+)\]|\[([a-z])-([a-z])\]|\[([A-Z])-([A-Z])\]/;

export interface Expansion {
  urls: string[];
  /** Links that were cut off at MAX_EXPANDED. */
  dropped: number;
  /** Whether any range was found at all. */
  expanded: boolean;
}

function values(match: RegExpExecArray): string[] {
  const [, a, b, la, lb, ua, ub] = match;
  if (a !== undefined && b !== undefined) {
    const from = Number.parseInt(a, 10);
    const to = Number.parseInt(b, 10);
    // `[001-100]` keeps three digits; `[1-100]` does not pad.
    const width = a.length > 1 && a.startsWith("0") ? a.length : 0;
    const step = from <= to ? 1 : -1;
    const out: string[] = [];
    for (let n = from; step > 0 ? n <= to : n >= to; n += step) {
      out.push(width ? String(n).padStart(width, "0") : String(n));
      if (out.length > MAX_EXPANDED) break;
    }
    return out;
  }
  const lo = (la ?? ua ?? "a").charCodeAt(0);
  const hi = (lb ?? ub ?? "a").charCodeAt(0);
  const step = lo <= hi ? 1 : -1;
  const out: string[] = [];
  for (let c = lo; step > 0 ? c <= hi : c >= hi; c += step) out.push(String.fromCharCode(c));
  return out;
}

function expandOne(link: string, budget: number): string[] {
  const match = RANGE.exec(link);
  if (!match) return [link];
  const head = link.slice(0, match.index);
  const tail = link.slice(match.index + match[0].length);
  const out: string[] = [];
  for (const v of values(match)) {
    for (const rest of expandOne(tail, budget - out.length)) {
      out.push(head + v + rest);
      if (out.length > budget) return out;
    }
  }
  return out;
}

/** Expand every bracket range in the http(s) tokens of `text`. */
export function expandPatterns(text: string): Expansion {
  const urls: string[] = [];
  let dropped = 0;
  let expanded = false;
  for (const token of text.split(/\s+/)) {
    if (!/^https?:\/\//i.test(token)) {
      if (token) urls.push(token);
      continue;
    }
    if (!RANGE.test(token)) {
      urls.push(token);
      continue;
    }
    expanded = true;
    const budget = MAX_EXPANDED - urls.length;
    const all = expandOne(token, budget);
    if (all.length > budget) {
      urls.push(...all.slice(0, Math.max(0, budget)));
      dropped += all.length - Math.max(0, budget);
    } else {
      urls.push(...all);
    }
  }
  return { urls, dropped, expanded };
}

/** Links from a dropped or imported text file: one per line, `#` comments allowed. */
export function linksFromList(text: string): string {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .join("\n");
}
