// Dates in the active language and the local time zone.
import { currentLocale } from "./core";

/**
 * Compact date for a list column: the time for today, day and month for
 * this year, and the year too before that.
 */
export function shortDate(unixSeconds: number, now: Date = new Date()): string {
  if (!Number.isFinite(unixSeconds) || unixSeconds <= 0) return "–";
  const date = new Date(unixSeconds * 1000);
  const sameDay = date.toDateString() === now.toDateString();
  const sameYear = date.getFullYear() === now.getFullYear();
  const options: Intl.DateTimeFormatOptions = sameDay
    ? { hour: "2-digit", minute: "2-digit" }
    : sameYear
      ? { day: "numeric", month: "short" }
      : { day: "numeric", month: "short", year: "numeric" };
  return new Intl.DateTimeFormat(currentLocale(), options).format(date);
}

/** Full date and time, for tooltips and the details dialog. */
export function longDate(unixSeconds: number | null): string {
  if (unixSeconds === null || !Number.isFinite(unixSeconds) || unixSeconds <= 0) return "–";
  return new Intl.DateTimeFormat(currentLocale(), {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(unixSeconds * 1000));
}
