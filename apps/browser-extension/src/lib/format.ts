// Same output as apps/desktop/src/lib/format.ts, so sizes and speeds read the
// same in the popup and in the app: binary units, numbers written the
// browser language's way.
import { currentLocale } from "../i18n/core";

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB"] as const;

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

/** A number with a fixed count of decimals: `1.5`, Turkish `1,5`. */
export function formatDecimal(n: number, digits: number): string {
  return numberFormat(digits).format(n);
}

/** A percent (0–100): `50%`, Turkish `%50`, German `50 %`. */
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

export function formatSpeed(bytesPerSecond: number): string {
  return bytesPerSecond > 0 ? `${formatBytes(bytesPerSecond)}/s` : "–";
}

/** 0–100, or null when the size is unknown. */
export function percent(downloaded: number, total: number | undefined): number | null {
  if (total === undefined) return null;
  if (total === 0) return 100;
  return Math.min(100, (downloaded / total) * 100);
}

/** Engine messages start lowercase; capitalize for display. */
export function sentence(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
