// Pure helpers of the site grabber dialog: file types, selection and totals.
import type { Category, FoundFile, GrabOptions, GrabProgress } from "./api";

export const MIN_DEPTH = 1;
export const MAX_DEPTH = 5;
export const MAX_PAGES = 2000;

export const GRAB_DEFAULTS: Omit<GrabOptions, "url" | "extensions"> = {
  depth: 2,
  max_pages: 100,
  same_host: true,
  respect_robots: true,
  delay_ms: 500,
  check_sizes: true,
};

/**
 * Extensions as people type them (`*.MP3, .zip pdf`), cleaned the way the
 * grabber compares them: lower case, no dots or wildcards, no duplicates.
 */
export function parseExtensions(text: string): string[] {
  const out: string[] = [];
  for (const part of text.split(/[\s,;]+/)) {
    const ext = part.trim().replace(/^\*/, "").replace(/^\.+/, "").toLowerCase();
    if (ext !== "" && ext.length <= 12 && /^[a-z0-9]+$/.test(ext) && !out.includes(ext)) out.push(ext);
  }
  return out;
}

/** The extensions of the chosen categories plus the custom ones. */
export function grabExtensions(categories: Category[], chosen: ReadonlySet<string>, custom: string): string[] {
  const all = categories.filter((c) => chosen.has(c.name)).flatMap((c) => c.extensions);
  return parseExtensions([...all, custom].join(" "));
}

/** A whole number within bounds, or the fallback when the field is not one. */
export function clampInt(text: string, min: number, max: number, fallback: number): number {
  const n = Number.parseInt(text, 10);
  return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : fallback;
}

/** Bytes of the chosen files whose size is known, and how many are not. */
export function selectedSize(files: FoundFile[], chosen: ReadonlySet<string>): { bytes: number; unknown: number } {
  let bytes = 0;
  let unknown = 0;
  for (const f of files) {
    if (!chosen.has(f.url)) continue;
    if (f.size === null) unknown += 1;
    else bytes += f.size;
  }
  return { bytes, unknown };
}

/** Files whose name, type or address contain the query; case ignored. */
export function filterFiles(files: FoundFile[], query: string): FoundFile[] {
  const q = query.trim().toLowerCase();
  if (q === "") return files;
  return files.filter((f) => f.name.toLowerCase().includes(q) || f.extension === q || f.url.toLowerCase().includes(q));
}

/** Toggle every shown file: all on unless they all are already. */
export function toggleShown(chosen: ReadonlySet<string>, shown: FoundFile[]): Set<string> {
  const next = new Set(chosen);
  const allOn = shown.length > 0 && shown.every((f) => next.has(f.url));
  for (const f of shown) {
    if (allOn) next.delete(f.url);
    else next.add(f.url);
  }
  return next;
}

/** 0 to 1 for the progress bar: pages first, then sizes. */
export function grabFraction(p: GrabProgress): number {
  if (p.phase === "done") return 1;
  if (p.phase === "sizes") return p.to_size === 0 ? 1 : p.sized / p.to_size;
  return p.pages_total === 0 ? 0 : p.pages_done / p.pages_total;
}
