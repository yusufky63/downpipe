// "Pause until…": the times the right-click menu offers, and how a paused
// download with such a time is described. The core resumes it then
// (`Manager::pause_until`); times travel as Unix seconds.
import { currentLocale, t } from "../i18n/core";
import type { Task } from "./api";
import { canPause } from "./tasks";

export type UntilPreset = "30m" | "1h" | "tonight";

export const UNTIL_PRESETS: readonly UntilPreset[] = ["30m", "1h", "tonight"];

/** The night-time preset's hour, IDM's and most schedulers' quiet time. */
export const TONIGHT = { hour: 2, minute: 0 };

/** The next time the clock shows `hour:minute`, strictly after `now`. */
export function nextClockTime(now: Date, hour: number, minute: number): Date {
  const at = new Date(now);
  at.setHours(hour, minute, 0, 0);
  if (at.getTime() <= now.getTime()) at.setDate(at.getDate() + 1);
  return at;
}

export function presetTime(preset: UntilPreset, now: Date): Date {
  switch (preset) {
    case "30m":
      return new Date(now.getTime() + 30 * 60_000);
    case "1h":
      return new Date(now.getTime() + 60 * 60_000);
    case "tonight":
      return nextClockTime(now, TONIGHT.hour, TONIGHT.minute);
  }
}

export function unixSeconds(date: Date): number {
  return Math.floor(date.getTime() / 1000);
}

/** Running, queued, paused or failed: everything that can wait for a time. */
export function canPauseUntil(task: Task): boolean {
  return !task.awaiting_confirmation && (canPause(task) || task.state === "paused" || task.state === "failed");
}

/** `YYYY-MM-DDTHH:MM` in local time, what `<input type="datetime-local">` takes. */
export function toLocalInput(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** The input's value back as a date; null when it is empty or malformed. */
export function fromLocalInput(value: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(value.trim());
  if (!m) return null;
  const [, y, mo, d, h, mi] = m.map(Number) as [number, number, number, number, number, number];
  const date = new Date(y, mo - 1, d, h, mi, 0, 0);
  // `new Date` rolls 31 February over into March; that is not what was typed.
  return date.getMonth() === mo - 1 && date.getDate() === d && date.getHours() === h ? date : null;
}

/**
 * A time to resume at, as short as it can be said: the time alone today,
 * the weekday within a week, else the date.
 */
export function clockText(unix: number, now: Date = new Date()): string {
  const date = new Date(unix * 1000);
  const time: Intl.DateTimeFormatOptions = { hour: "2-digit", minute: "2-digit" };
  const days = (startOfDay(date) - startOfDay(now)) / 86_400_000;
  const options: Intl.DateTimeFormatOptions =
    days === 0 ? time : days > 0 && days < 7 ? { ...time, weekday: "short" } : { ...time, day: "numeric", month: "short" };
  return new Intl.DateTimeFormat(currentLocale(), options).format(date);
}

function startOfDay(date: Date): number {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

/** "Resumes at 02:00" for a download paused until a time; null otherwise. */
export function resumesText(task: Task, now: Date = new Date()): string | null {
  if (task.resume_at === null || (task.state !== "paused" && task.state !== "failed")) return null;
  return t("list.until.resumesAt", { time: clockText(task.resume_at, now) });
}
