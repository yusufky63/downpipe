import type { ManagerEvent, Task, TaskState } from "./api";
import { formatBytes, nameFromUrl } from "./format";

export type Filter = "all" | "active" | "queued" | "completed" | "failed";

export const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "active", label: "Active" },
  { id: "queued", label: "Queued" },
  { id: "completed", label: "Completed" },
  { id: "failed", label: "Failed" },
];

const RUNNING: ReadonlySet<TaskState> = new Set(["starting", "downloading", "waiting", "verifying"]);

export function isRunning(state: TaskState): boolean {
  return RUNNING.has(state);
}

export function applyEvent(tasks: Task[], event: ManagerEvent): Task[] {
  switch (event.type) {
    case "updated": {
      const idx = tasks.findIndex((t) => t.id === event.task.id);
      if (idx === -1) return [...tasks, event.task];
      const next = tasks.slice();
      next[idx] = event.task;
      return next;
    }
    case "removed":
      return tasks.filter((t) => t.id !== event.id);
    case "settings":
      return tasks;
  }
}

export function matchesFilter(task: Task, filter: Filter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "active":
      return isRunning(task.state);
    case "queued":
      return task.state === "queued" || task.state === "paused";
    case "completed":
      return task.state === "completed";
    case "failed":
      return task.state === "failed" || task.state === "canceled";
  }
}

export function countByFilter(tasks: Task[]): Record<Filter, number> {
  const counts: Record<Filter, number> = { all: 0, active: 0, queued: 0, completed: 0, failed: 0 };
  for (const task of tasks) {
    for (const { id } of FILTERS) {
      if (matchesFilter(task, id)) counts[id] += 1;
    }
  }
  return counts;
}

export function totals(tasks: Task[]): { speed: number; active: number; queued: number } {
  let speed = 0;
  let active = 0;
  let queued = 0;
  for (const t of tasks) {
    if (isRunning(t.state)) {
      active += 1;
      speed += t.speed;
    } else if (t.state === "queued") {
      queued += 1;
    }
  }
  return { speed, active, queued };
}

/** Newest first, so a freshly added link appears at the top. */
export function sortForDisplay(tasks: Task[]): Task[] {
  return tasks.slice().sort((a, b) => b.created_at - a.created_at || b.id.localeCompare(a.id));
}

export function displayName(task: Task): string {
  if (task.file_name) return task.file_name;
  // A page's address names nothing ("watch"); its title arrives later.
  if (task.site) return `${task.site.site} video`;
  return nameFromUrl(task.url);
}

/**
 * Where a download waiting for its answer is asked: a video page added in
 * the app in the main window, where the user already is; whatever the
 * browser sent in the confirmation window, over the browser.
 */
export type AskedIn = "main" | "window";

/**
 * The downloads waiting for their answer in that place, in the order they
 * came: `created_at` counts seconds, and a page often sends several in one,
 * so their place in the queue (given as they are added) decides then.
 */
export function awaitingIn(tasks: Task[], place: AskedIn): Task[] {
  return tasks
    .filter((t) => t.awaiting_confirmation && t.from_app === (place === "main"))
    .sort((a, b) => a.created_at - b.created_at || a.position - b.position);
}

/**
 * The size to show before a download starts: the real one once known, else
 * a stream's or video's estimate for the chosen quality. Nothing is fetched
 * for it, so a plain file has none yet.
 */
export function knownSize(total: number | null, estimate: number | undefined): string | null {
  if (total !== null && total > 0) return formatBytes(total);
  if (estimate !== undefined && estimate > 0) return `~${formatBytes(estimate)}`;
  return null;
}

export const canPause = (t: Task) => isRunning(t.state) || t.state === "queued";
export const canResume = (t: Task) => t.state === "paused" || t.state === "failed" || t.state === "canceled";
export const canCancel = (t: Task) => t.state !== "completed" && t.state !== "canceled";

export type Tone = "primary" | "positive" | "danger" | "warning" | "neutral";

export function stateInfo(task: Task): { label: string; tone: Tone } {
  if (task.awaiting_confirmation) return { label: "Confirm", tone: "warning" };
  switch (task.state) {
    case "queued":
      return { label: "Queued", tone: "neutral" };
    case "starting":
      return { label: "Connecting", tone: "primary" };
    case "downloading":
      return { label: "Downloading", tone: "primary" };
    case "waiting":
      return { label: "Retrying", tone: "warning" };
    case "verifying":
      return { label: "Verifying", tone: "primary" };
    case "paused":
      return { label: "Paused", tone: "neutral" };
    case "completed":
      return { label: "Done", tone: "positive" };
    case "failed":
      return { label: "Failed", tone: "danger" };
    case "canceled":
      return { label: "Canceled", tone: "neutral" };
  }
}
