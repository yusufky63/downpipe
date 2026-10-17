import { formatBytes, formatSpeed, percent, sentence } from "./format";
import { kindLabel, mediaName, type MediaCandidate } from "./media";
import type { Activity, PageItem } from "./messages";
import type { TaskState, TaskStatus } from "./protocol";
import type { BridgeStatus } from "./status";

export type Tone = "muted" | "primary" | "positive" | "warning" | "danger";

export interface ItemView {
  label: string;
  /** Short state word shown as a badge. */
  badge: string;
  tone: Tone;
  detail: string;
  /** 0–100 while running with a known size, else null (no bar). */
  progress: number | null;
}

const RUNNING: readonly TaskState[] = ["queued", "starting", "downloading", "waiting", "verifying"];

export function isRunning(state: TaskState): boolean {
  return RUNNING.includes(state);
}

/** Paused only because the app's confirmation dialog has not been answered. */
export function isAwaitingConfirmation(task: TaskStatus): boolean {
  return task.state === "paused" && task.awaitingConfirmation === true;
}

/**
 * Worth asking about again while the popup is open: a running download
 * moves on by itself, and a held one starts as soon as the user answers.
 */
export function mayChange(task: TaskStatus): boolean {
  return isRunning(task.state) || isAwaitingConfirmation(task);
}

const STATE_VIEW: Record<TaskState, { badge: string; tone: Tone }> = {
  queued: { badge: "Queued", tone: "muted" },
  starting: { badge: "Starting", tone: "primary" },
  downloading: { badge: "Downloading", tone: "primary" },
  waiting: { badge: "Waiting", tone: "warning" },
  verifying: { badge: "Verifying", tone: "primary" },
  paused: { badge: "Paused", tone: "muted" },
  completed: { badge: "Done", tone: "positive" },
  failed: { badge: "Failed", tone: "danger" },
  canceled: { badge: "Canceled", tone: "muted" },
};

const OUTCOME_VIEW: Record<Activity["outcome"], { badge: string; tone: Tone }> = {
  added: { badge: "Sent", tone: "muted" },
  duplicate: { badge: "In Downpipe", tone: "muted" },
  failed: { badge: "Not sent", tone: "danger" },
  kept: { badge: "In browser", tone: "warning" },
};

function sizeLine(task: TaskStatus): string {
  const done = formatBytes(task.downloaded);
  return task.total === undefined ? done : `${done} / ${formatBytes(task.total)}`;
}

/**
 * One row of the popup's download list: the live state from the app when it
 * could be asked, otherwise what happened when the download was sent.
 */
export function itemView(activity: Activity, task: TaskStatus | undefined): ItemView {
  if (!task) {
    const { badge, tone } = OUTCOME_VIEW[activity.outcome];
    return { label: activity.label, badge, tone, detail: activity.message, progress: null };
  }
  const label = task.fileName || activity.label;
  if (isAwaitingConfirmation(task)) {
    // Not "Paused": nothing stopped it, the app is asking where to save it.
    // The detail is short because the long badge leaves it little room.
    return { label, badge: "Waiting in Downpipe", tone: "warning", detail: "Needs your answer", progress: null };
  }
  const { badge, tone } = STATE_VIEW[task.state];
  const pct = percent(task.downloaded, task.total);
  const pctText = pct === null ? "" : `${Math.floor(pct)}% · `;
  switch (task.state) {
    case "completed":
      return { label, badge, tone, detail: formatBytes(task.total ?? task.downloaded), progress: null };
    case "failed":
      return { label, badge, tone, detail: task.error ? sentence(task.error) : "The download failed", progress: null };
    case "canceled":
      return { label, badge, tone, detail: "Canceled in Downpipe", progress: null };
    case "paused":
      return { label, badge, tone, detail: `${pctText}${sizeLine(task)}`, progress: null };
    default: {
      const speed = task.speed > 0 ? ` · ${formatSpeed(task.speed)}` : "";
      return { label, badge, tone, detail: `${pctText}${sizeLine(task)}${speed}`, progress: pct };
    }
  }
}

export interface StatusView {
  tone: Tone;
  title: string;
  detail: string;
  /** A command the user can copy, when there is one to run. */
  command?: string;
  /** Link to the setup guide. */
  guide: boolean;
}

/** What the popup shows for each bridge state. */
export function statusView(
  status: BridgeStatus | null,
  extension: { id: string; firefox: boolean },
): StatusView {
  if (!status) {
    return { tone: "muted", title: "Checking the connection", detail: "", guide: false };
  }
  switch (status.state) {
    case "connected":
      return {
        tone: "positive",
        title: "Connected to Downpipe",
        detail: `App ${status.appVersion}, browser bridge ${status.hostVersion}`,
        guide: false,
      };
    case "app_not_running":
      return {
        tone: "warning",
        title: "Downpipe is not running",
        detail: "Open the Downpipe app. Links sent while it is closed are not queued.",
        guide: false,
      };
    case "host_missing":
      return {
        tone: "danger",
        title: "Browser bridge not installed",
        detail: "Open the Downpipe app once; it registers the bridge when it starts. For a build from source, run:",
        command: "downpipe-native-host install",
        guide: true,
      };
    case "host_forbidden": {
      const flag = extension.firefox ? "--firefox-extension-id" : "--extension-id";
      return {
        tone: "danger",
        title: "Bridge does not allow this extension",
        detail: "Register it for this extension's id, then restart the browser:",
        command: `downpipe-native-host install ${flag} ${extension.id}`,
        guide: true,
      };
    }
    case "error":
      return { tone: "danger", title: "Bridge error", detail: status.message, guide: true };
  }
}

/** One row of the popup's media list. */
export interface MediaRowView {
  label: string;
  detail: string;
  /** False when this Downpipe cannot take the stream (it is too old). */
  sendable: boolean;
}

/**
 * `streams`: whether the connected Downpipe can download HLS and DASH
 * streams; single media files go through the ordinary download message.
 * `label`: the row's name from `mediaLabels`, the file name without one.
 */
export function mediaRowView(c: MediaCandidate, streams: boolean, label = mediaName(c.url)): MediaRowView {
  let host = "";
  try {
    host = new URL(c.url).hostname;
  } catch {
    // Stored entries are validated; keep the row anyway.
  }
  const parts = [kindLabel(c), c.size !== undefined ? formatBytes(c.size) : "", host].filter(Boolean);
  const sendable = c.kind === "direct" || streams;
  return {
    label,
    detail: sendable ? parts.join(" · ") : "Update Downpipe to download streams",
    sendable,
  };
}

/**
 * The row for a video page (YouTube) in the media list. `pages`: whether
 * the connected Downpipe has the site provider.
 */
export function pageRowView(page: PageItem, pages: boolean): MediaRowView {
  return {
    label: page.title ?? `This ${page.site} video`,
    // Short: the row leaves the detail little room. The quality is chosen in the app.
    detail: pages ? `${page.site} video` : "Update Downpipe to download videos from pages",
    sendable: pages,
  };
}

/** What the media list says when it has no rows. */
export function mediaEmptyText(enabled: boolean, excludedBy?: string): string {
  if (!enabled) return "Turn on to list the videos and audio pages play. The browser asks for access to all sites first.";
  if (excludedBy) return `Not listed here: ${excludedBy} is an excluded site in Filters.`;
  return "Nothing yet. Play the video; if it was already playing, reload the page.";
}
