import { t, type MessageKey } from "../i18n/core";
import type { ManagerEvent, Task, TaskState, WaitReason } from "./api";
import { formatBytes, nameFromUrl, sentence } from "./format";

export type Filter = "all" | "active" | "queued" | "completed" | "failed";

export const FILTERS: readonly Filter[] = ["all", "active", "queued", "completed", "failed"];

export function filterLabel(filter: Filter): string {
  return t(`states.filters.${filter}`);
}

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
    for (const id of FILTERS) {
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
  if (task.site) return t("common.siteVideo", { site: task.site.site });
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

/**
 * Failures only a fresh link fixes: the old one expired (a web page came
 * back instead of the file, or a 403) or now leads to a sign-in page.
 * Retrying would get the same answer, so the list offers a new link instead.
 */
export function needsNewLink(task: Task): boolean {
  return task.state === "failed" && task.site === null && task.error_kind !== null && NEW_LINK_KINDS.has(task.error_kind);
}

const NEW_LINK_KINDS = new Set(["expired_link", "login_required", "forbidden"]);

const FAILURE_TEXTS: Partial<Record<string, MessageKey>> = {
  expired_link: "list.failure.expiredLink",
  login_required: "list.failure.loginRequired",
};

/** What went wrong: in the user's language for the kinds the app knows, else the engine's message. */
export function errorText(task: Task): string | null {
  const key = task.error_kind === null ? undefined : FAILURE_TEXTS[task.error_kind];
  if (key) return t(key);
  return task.error ? sentence(task.error) : null;
}

const WAIT_TEXTS: Record<WaitReason, MessageKey> = {
  no_data: "states.why.noData",
  network: "states.why.network",
  throttled: "states.why.throttled",
  server: "states.why.server",
};

/** Why a download is retrying; null unless it is. */
export function waitingText(task: Task): string | null {
  return task.state === "waiting" && task.waiting ? t(WAIT_TEXTS[task.waiting]) : null;
}

export const canPause = (t: Task) => isRunning(t.state) || t.state === "queued";
export const canResume = (t: Task) => t.state === "paused" || t.state === "failed" || t.state === "canceled";
export const canCancel = (t: Task) => t.state !== "completed" && t.state !== "canceled";

export type Tone = "primary" | "positive" | "danger" | "warning" | "neutral";

export function stateInfo(task: Task): { label: string; tone: Tone } {
  if (task.awaiting_confirmation) return { label: t("states.confirm"), tone: "warning" };
  return { label: t(`states.${task.state}`), tone: STATE_TONES[task.state] };
}

const STATE_TONES: Record<TaskState, Tone> = {
  queued: "neutral",
  starting: "primary",
  downloading: "primary",
  waiting: "warning",
  verifying: "primary",
  paused: "neutral",
  completed: "positive",
  failed: "danger",
  canceled: "neutral",
};

/** When a download last did something: when it finished, else its last change. */
export function lastActivity(task: Task): number {
  return task.completed_at ?? task.updated_at;
}
