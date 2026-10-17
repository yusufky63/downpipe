// Same output as apps/desktop/src/lib/format.ts, so sizes and speeds read the
// same in the popup and in the app.
const UNITS = ["B", "KiB", "MiB", "GiB", "TiB"] as const;

/** Binary units with one decimal: `12.3 MiB`. */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "–";
  if (n < 1024) return `${Math.round(n)} B`;
  let value = n;
  let unit = 0;
  // 1023.96 would print as "1024.0 KiB"; promote it to the next unit.
  while (value >= 1023.95 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${UNITS[unit]}`;
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
