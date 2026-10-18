import { currentLocale } from "../i18n/core";

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB"] as const;

// Building a NumberFormat is far slower than using one, and a list of
// thousands of rows formats several numbers per row.
const formats = new Map<string, Intl.NumberFormat>();

function numberFormat(digits: number, style: "decimal" | "percent" = "decimal"): Intl.NumberFormat {
  const locale = currentLocale();
  const key = `${locale}|${style}|${digits}`;
  let format = formats.get(key);
  if (!format) {
    // No thousands separators: a size never needs one ("1023 B"), and a
    // grouped "1.023" would read as a decimal in German.
    format = new Intl.NumberFormat(locale, { style, minimumFractionDigits: digits, maximumFractionDigits: digits, useGrouping: false });
    formats.set(key, format);
  }
  return format;
}

/** A number with a fixed count of decimals, the active language's way: `1.5`, Turkish `1,5`. */
export function formatDecimal(n: number, digits: number): string {
  return numberFormat(digits).format(n);
}

/**
 * A percent (0–100) the active language's way: `50%`, Turkish `%50`,
 * German `50 %`.
 */
export function formatPercent(pct: number, digits = 0): string {
  return numberFormat(digits, "percent").format(pct / 100);
}

/** Binary units with one decimal: `12.3 MiB`, Turkish `12,3 MiB`. */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "–";
  if (n < 1024) return `${formatDecimal(Math.round(n), 0)} B`;
  let value = n;
  let unit = 0;
  // 1023.96 would print as "1024.0 KiB"; promote it to the next unit.
  while (value >= 1023.95 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${formatDecimal(value, 1)} ${UNITS[unit]}`;
}

/** Engine messages start lowercase (they compose into longer errors); capitalize for display. */
export function sentence(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** A message followed by more text needs its full stop: "(HTTP 404) It is…" reads as one broken sentence. */
export function withStop(text: string): string {
  return text === "" || /[.!?]$/.test(text) ? text : `${text}.`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return bytesPerSecond > 0 ? `${formatBytes(bytesPerSecond)}/s` : "–";
}

/** `42s`, `3:07`, `1:02:03`. */
export function formatDuration(totalSeconds: number | null): string {
  if (totalSeconds === null || !Number.isFinite(totalSeconds) || totalSeconds < 0) return "–";
  const s = Math.round(totalSeconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  if (m > 0) return `${m}:${String(sec).padStart(2, "0")}`;
  return `${sec}s`;
}

/**
 * Unix seconds in the active language and the local time zone. English
 * keeps the day-first, 24-hour `17 Oct 2026, 15:42` it always had rather
 * than the US order `en` alone would give.
 */
export function formatDate(unixSeconds: number | null): string {
  if (unixSeconds === null || !Number.isFinite(unixSeconds) || unixSeconds <= 0) return "–";
  const locale = currentLocale();
  return new Intl.DateTimeFormat(locale === "en" ? "en-GB" : locale, {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(unixSeconds * 1000));
}

/** 0–100, or null when the size is unknown. */
export function percent(downloaded: number, total: number | null): number | null {
  if (total === null) return null;
  if (total === 0) return 100;
  return Math.min(100, (downloaded / total) * 100);
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** Last path segment of a URL, decoded, as a stand-in name before the server answers. */
export function nameFromUrl(url: string): string {
  try {
    const u = new URL(url);
    const last = u.pathname.split("/").filter(Boolean).pop();
    return last ? decodeURIComponent(last) : u.host;
  } catch {
    return url;
  }
}

/** Every http(s) URL and magnet link in pasted text, deduplicated, in order. */
export function parseUrls(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const token of text.split(/\s+/)) {
    const candidate = token.trim().replace(/^[<("']+|[>)"',]+$/g, "");
    // A magnet link is kept as written: the app reads it, and the URL
    // parser would not tidy it anyway.
    if (/^magnet:\?/i.test(candidate) && /[?&]xt=urn:btih:/i.test(candidate)) {
      if (!seen.has(candidate)) {
        seen.add(candidate);
        out.push(candidate);
      }
      continue;
    }
    if (!/^https?:\/\//i.test(candidate)) continue;
    try {
      const normalized = new URL(candidate).toString();
      if (!seen.has(normalized)) {
        seen.add(normalized);
        out.push(normalized);
      }
    } catch {
      // Not a URL after all.
    }
  }
  return out;
}

export type RateUnit = "KiB" | "MiB";

export function rateToBytes(value: number, unit: RateUnit): number {
  if (!Number.isFinite(value) || value <= 0) return 0;
  return Math.round(value * (unit === "MiB" ? 1024 * 1024 : 1024));
}

/**
 * A decimal typed by the user: a dot or a comma before the decimals, as
 * either is what someone's keyboard and habit give. NaN for anything else.
 */
export function parseDecimal(text: string): number {
  const value = text.trim();
  if (!/^\d+(?:[.,]\d+)?$/.test(value)) return Number.NaN;
  return Number(value.replace(",", "."));
}

/** A stored decimal as an input field shows it, with the language's decimal sign. */
export function decimalField(value: number): string {
  const sign = formatDecimal(1.5, 1).replace(/\d/g, "");
  return String(value).replace(".", sign);
}

/** Pick the unit that shows a speed limit most naturally. */
export function bytesToRate(bytes: number): { value: number; unit: RateUnit } {
  if (bytes <= 0) return { value: 0, unit: "MiB" };
  if (bytes >= 1024 * 1024 && bytes % (1024 * 1024) === 0) return { value: bytes / (1024 * 1024), unit: "MiB" };
  if (bytes >= 1024 * 1024) return { value: Number((bytes / (1024 * 1024)).toFixed(2)), unit: "MiB" };
  return { value: Number((bytes / 1024).toFixed(1)), unit: "KiB" };
}
