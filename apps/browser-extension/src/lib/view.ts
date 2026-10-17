import { formatBytes, formatSpeed, percent, sentence } from "./format";
import type { Activity } from "./messages";
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
  const { badge, tone } = STATE_VIEW[task.state];
  const label = task.fileName || activity.label;
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
