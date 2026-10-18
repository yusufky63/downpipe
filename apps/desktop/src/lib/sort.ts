import { currentLocale } from "../i18n/core";
import type { Task } from "./api";
import { percent } from "./format";
import { displayName, isRunning, lastActivity } from "./tasks";

/** `order` is the order downloads start in: queue position. `date` is the last activity. */
export type SortKey = "added" | "name" | "size" | "progress" | "speed" | "left" | "order" | "date";
export type SortDir = "asc" | "desc";

export interface Sort {
  key: SortKey;
  dir: SortDir;
}

export const DEFAULT_SORT: Sort = { key: "added", dir: "desc" };

/** Clicking a column sorts by it; clicking it again flips the direction. */
export function nextSort(current: Sort, key: SortKey): Sort {
  if (current.key === key) return { key, dir: current.dir === "asc" ? "desc" : "asc" };
  // Text reads best A to Z; numbers are most useful largest first.
  return { key, dir: key === "name" || key === "left" || key === "order" ? "asc" : "desc" };
}


/** Unknown values (no size yet, not running) sort last in either direction. */
function value(task: Task, key: SortKey): number | string | null {
  switch (key) {
    case "added":
      return task.created_at;
    case "name":
      return displayName(task);
    case "size":
      return task.total;
    case "progress":
      return percent(task.downloaded, task.total);
    case "speed":
      return isRunning(task.state) ? task.speed : null;
    case "left":
      return isRunning(task.state) ? task.eta_secs : null;
    case "order":
      return task.position;
    case "date":
      return lastActivity(task);
  }
}

export function sortTasks(tasks: readonly Task[], sort: Sort): Task[] {
  const sign = sort.dir === "asc" ? 1 : -1;
  // Names sort as the reader's language expects.
  const collator = new Intl.Collator(currentLocale(), { numeric: true, sensitivity: "base" });
  return tasks.slice().sort((a, b) => {
    const va = value(a, sort.key);
    const vb = value(b, sort.key);
    if (va === null && vb === null) return b.created_at - a.created_at;
    if (va === null) return 1;
    if (vb === null) return -1;
    const cmp = typeof va === "string" || typeof vb === "string" ? collator.compare(String(va), String(vb)) : va - vb;
    return cmp !== 0 ? sign * cmp : b.created_at - a.created_at || a.id.localeCompare(b.id);
  });
}

/** Case-insensitive match on name, link, folder and category; every word must match. */
export function matchesSearch(task: Task, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const haystack = [displayName(task), task.url, task.path ?? task.dir, task.category ?? ""].join("\n").toLowerCase();
  return words.every((w) => haystack.includes(w));
}

export function parseSort(raw: string | null): Sort {
  if (!raw) return DEFAULT_SORT;
  const [key, dir] = raw.split(":");
  const keys: SortKey[] = ["added", "name", "size", "progress", "speed", "left", "order", "date"];
  if (keys.includes(key as SortKey) && (dir === "asc" || dir === "desc")) return { key: key as SortKey, dir };
  return DEFAULT_SORT;
}

export function formatSort(sort: Sort): string {
  return `${sort.key}:${sort.dir}`;
}
