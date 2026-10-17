// What the popup and the service worker say to each other.
import type { TaskStatus } from "./protocol";
import type { BridgeStatus, Outcome } from "./status";

export type PopupRequest = { kind: "status" } | { kind: "tasks" } | { kind: "set_notify"; enabled: boolean };

export function parsePopupRequest(value: unknown): PopupRequest | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  const keys = Object.keys(v).length;
  if ((v.kind === "status" || v.kind === "tasks") && keys === 1) return { kind: v.kind };
  if (v.kind === "set_notify" && typeof v.enabled === "boolean" && keys === 2) {
    return { kind: "set_notify", enabled: v.enabled };
  }
  return null;
}

/** Stored in `storage.local`. */
export interface Settings {
  /** Show a notification when a download is handed over, or could not be. */
  notify: boolean;
}

export function parseSettings(value: unknown): Settings {
  const v = typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  return { notify: v.notify !== false };
}

export interface Activity {
  /** Milliseconds since the epoch. */
  at: number;
  /** File name or host, never the full URL. */
  label: string;
  source: "link" | "capture";
  /** `kept` means capture failed and the browser downloaded it instead. */
  outcome: Outcome | "kept";
  message: string;
  /** The app's id for the download, when it took it. */
  taskId?: string;
}

export interface StatusReply {
  bridge: BridgeStatus;
  /** The `downloads` permission is granted, so browser downloads are captured. */
  capture: boolean;
  notify: boolean;
  activity: Activity[];
  /** Live state of the downloads in `activity`, when the app could be asked. */
  tasks: TaskStatus[];
}

export interface TasksReply {
  /** Null when the app could not be asked. */
  tasks: TaskStatus[] | null;
}

/** The app's ids for the listed downloads, without repeats. */
export function taskIdsOf(list: readonly Activity[]): string[] {
  return [...new Set(list.flatMap((a) => (a.taskId ? [a.taskId] : [])))];
}

export const MAX_ACTIVITY = 5;

/** Newest first, capped. */
export function addActivity(list: readonly Activity[], entry: Activity): Activity[] {
  return [entry, ...list].slice(0, MAX_ACTIVITY);
}

export function parseActivity(value: unknown): Activity[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((a): a is Activity => {
      if (typeof a !== "object" || a === null) return false;
      const v = a as Record<string, unknown>;
      return (
        typeof v.at === "number" &&
        typeof v.label === "string" &&
        typeof v.message === "string" &&
        (v.source === "link" || v.source === "capture") &&
        ["added", "duplicate", "failed", "kept"].includes(v.outcome as string) &&
        (v.taskId === undefined || typeof v.taskId === "string")
      );
    })
    .slice(0, MAX_ACTIVITY);
}

/** "just now", "4 min ago", "2 h ago", or a date. */
export function relativeTime(at: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - at) / 1000));
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return new Date(at).toISOString().slice(0, 10);
}
