import { ext } from "../lib/browser";
import { relativeTime, type Activity, type StatusReply, type TasksReply } from "../lib/messages";
import type { TaskStatus } from "../lib/protocol";
import { isRunning, itemView, statusView } from "../lib/view";

const DOWNLOADS_PERMISSION: chrome.permissions.Permissions = { permissions: ["downloads"] };
/** While a listed download runs, its progress is refreshed this often. */
const POLL_MS = 1_000;

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} is missing from popup.html`);
  return el as T;
}

const els = {
  version: byId("version"),
  status: byId("status"),
  title: byId("status-title"),
  detail: byId("status-detail"),
  command: byId("status-command"),
  recheck: byId<HTMLButtonElement>("recheck"),
  guide: byId<HTMLAnchorElement>("guide"),
  actions: byId("status-actions"),
  capture: byId<HTMLInputElement>("capture"),
  notify: byId<HTMLInputElement>("notify"),
  activity: byId<HTMLUListElement>("activity"),
  empty: byId("activity-empty"),
};

const extension = {
  id: ext.runtime.id,
  firefox: ext.runtime.getURL("").startsWith("moz-extension:"),
};

/** What the list shows: the sent downloads and their last known state. */
let listed: Activity[] = [];
let live = new Map<string, TaskStatus>();
let pollTimer: ReturnType<typeof setTimeout> | undefined;

function isStatusReply(value: unknown): value is StatusReply {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.bridge === "object" &&
    typeof v.capture === "boolean" &&
    typeof v.notify === "boolean" &&
    Array.isArray(v.activity) &&
    Array.isArray(v.tasks)
  );
}

function isTasksReply(value: unknown): value is TasksReply {
  if (typeof value !== "object" || value === null) return false;
  const tasks = (value as Record<string, unknown>).tasks;
  return tasks === null || Array.isArray(tasks);
}

function renderStatus(reply: StatusReply | null): void {
  const view = statusView(reply?.bridge ?? null, extension);
  els.status.dataset.tone = view.tone;
  els.title.textContent = view.title;
  els.detail.textContent = view.detail;
  els.detail.hidden = !view.detail;
  els.command.textContent = view.command ?? "";
  els.command.hidden = !view.command;
  els.guide.hidden = !view.guide;
  // Nothing to do while connected; the popup checks again each time it opens.
  els.actions.hidden = view.tone === "positive";
}

function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

function activityItem(entry: Activity, now: number): HTMLLIElement {
  const view = itemView(entry, entry.taskId ? live.get(entry.taskId) : undefined);
  const li = document.createElement("li");
  li.className = "item";
  li.dataset.tone = view.tone;
  const label = span("item-label", view.label);
  label.title = view.label;
  const detail = span("item-detail num", view.detail);
  detail.title = view.detail;
  li.append(label, span("badge", view.badge), detail);
  // A running download shows live numbers across the row; a finished one
  // also says when it was sent.
  if (view.progress === null) li.append(span("item-time num", relativeTime(entry.at, now)));
  else detail.classList.add("item-detail-wide");
  if (view.progress !== null) {
    const bar = document.createElement("div");
    bar.className = "bar";
    bar.setAttribute("role", "progressbar");
    bar.setAttribute("aria-label", `${view.label} progress`);
    bar.setAttribute("aria-valuemin", "0");
    bar.setAttribute("aria-valuemax", "100");
    bar.setAttribute("aria-valuenow", String(Math.floor(view.progress)));
    const fill = document.createElement("div");
    fill.className = "bar-fill";
    fill.style.width = `${view.progress}%`;
    bar.append(fill);
    li.append(bar);
  }
  return li;
}

function renderList(): void {
  const now = Date.now();
  els.activity.replaceChildren(...listed.map((entry) => activityItem(entry, now)));
  els.empty.hidden = listed.length > 0;
}

function applyTasks(tasks: TaskStatus[]): void {
  live = new Map(tasks.map((t) => [t.taskId, t]));
  renderList();
  // Keep refreshing only while something the popup shows is still running.
  clearTimeout(pollTimer);
  if (tasks.some((t) => isRunning(t.state))) pollTimer = setTimeout(() => void poll(), POLL_MS);
}

async function poll(): Promise<void> {
  const reply: unknown = await ext.runtime.sendMessage({ kind: "tasks" }).catch(() => null);
  if (isTasksReply(reply) && reply.tasks) applyTasks(reply.tasks);
}

async function refresh(): Promise<void> {
  els.recheck.disabled = true;
  try {
    const reply: unknown = await ext.runtime.sendMessage({ kind: "status" });
    const status = isStatusReply(reply) ? reply : null;
    renderStatus(
      status ?? {
        bridge: { state: "error", message: "The extension did not answer." },
        capture: false,
        notify: true,
        activity: [],
        tasks: [],
      },
    );
    if (status) {
      els.capture.checked = status.capture;
      els.notify.checked = status.notify;
      listed = status.activity;
      applyTasks(status.tasks);
    }
  } finally {
    els.recheck.disabled = false;
  }
}

// The toggle is the optional `downloads` permission itself: granted means
// capture is on. The request has to come straight from this click.
els.capture.addEventListener("change", () => {
  const wanted = els.capture.checked;
  els.capture.disabled = true;
  const change = wanted ? ext.permissions.request(DOWNLOADS_PERMISSION) : ext.permissions.remove(DOWNLOADS_PERMISSION);
  void change
    .catch(() => false)
    .then(() => ext.permissions.contains(DOWNLOADS_PERMISSION))
    .then((granted) => {
      els.capture.checked = granted;
    })
    .finally(() => {
      els.capture.disabled = false;
    });
});

els.notify.addEventListener("change", () => {
  els.notify.disabled = true;
  void ext.runtime
    .sendMessage({ kind: "set_notify", enabled: els.notify.checked })
    .catch(() => undefined)
    .finally(() => {
      els.notify.disabled = false;
    });
});

els.recheck.addEventListener("click", () => {
  renderStatus(null);
  void refresh();
});

els.version.textContent = `v${ext.runtime.getManifest().version}`;
void refresh();