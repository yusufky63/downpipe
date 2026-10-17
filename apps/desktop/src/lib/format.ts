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

/** Engine messages start lowercase (they compose into longer errors); capitalize for display. */
export function sentence(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
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

const DATE_FORMAT = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

/** Unix seconds as `17 Oct 2026, 15:42` in the local time zone. */
export function formatDate(unixSeconds: number | null): string {
  if (unixSeconds === null || !Number.isFinite(unixSeconds) || unixSeconds <= 0) return "–";
  return DATE_FORMAT.format(new Date(unixSeconds * 1000));
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

/** Every http(s) URL in pasted text, deduplicated, in order. */
export function parseUrls(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const token of text.split(/\s+/)) {
    const candidate = token.trim().replace(/^[<("']+|[>)"',]+$/g, "");
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

/** Pick the unit that shows a speed limit most naturally. */
export function bytesToRate(bytes: number): { value: number; unit: RateUnit } {
  if (bytes <= 0) return { value: 0, unit: "MiB" };
  if (bytes >= 1024 * 1024 && bytes % (1024 * 1024) === 0) return { value: bytes / (1024 * 1024), unit: "MiB" };
  if (bytes >= 1024 * 1024) return { value: Number((bytes / (1024 * 1024)).toFixed(2)), unit: "MiB" };
  return { value: Number((bytes / 1024).toFixed(1)), unit: "KiB" };
}
