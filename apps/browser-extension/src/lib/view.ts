import { t, type MessageKey } from "../i18n";
import { formatBytes, formatPercent, formatSpeed, percent, sentence } from "./format";
import { kindLabel, mediaName, type MediaCandidate } from "./media";
import type { Activity, PageItem } from "./messages";
import type { TaskState, TaskStatus } from "./protocol";
import { kindText, type BridgeStatus } from "./status";

export type Tone = "muted" | "primary" | "positive" | "warning" | "danger";

export interface ItemView {
  label: string;
  /** Short state word shown as a badge. */
  badge: string;
  tone: Tone;
  detail: string;
  /** 0–100 while running with a known size, else null (no bar). */
  progress: number | null;
  /** The app's own words for a failure the extension worded itself, for the tooltip. */
  technical?: string;
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

// Keys, not texts: the language is chosen after this module loads.
const STATE_VIEW: Record<TaskState, { badge: MessageKey; tone: Tone }> = {
  queued: { badge: "popup.state.queued", tone: "muted" },
  starting: { badge: "popup.state.starting", tone: "primary" },
  downloading: { badge: "popup.state.downloading", tone: "primary" },
  waiting: { badge: "popup.state.waiting", tone: "warning" },
  verifying: { badge: "popup.state.verifying", tone: "primary" },
  paused: { badge: "popup.state.paused", tone: "muted" },
  completed: { badge: "popup.state.completed", tone: "positive" },
  failed: { badge: "popup.state.failed", tone: "danger" },
  canceled: { badge: "popup.state.canceled", tone: "muted" },
};

const OUTCOME_VIEW: Record<Activity["outcome"], { badge: MessageKey; tone: Tone }> = {
  added: { badge: "popup.outcome.added", tone: "muted" },
  duplicate: { badge: "popup.outcome.duplicate", tone: "muted" },
  failed: { badge: "popup.outcome.failed", tone: "danger" },
  kept: { badge: "popup.outcome.kept", tone: "warning" },
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
    return { label: activity.label, badge: t(badge), tone, detail: activity.message, progress: null };
  }
  const label = task.fileName || activity.label;
  if (isAwaitingConfirmation(task)) {
    // Not "Paused": nothing stopped it, the app is asking where to save it.
    // The detail is short because the long badge leaves it little room.
    return { label, badge: t("popup.waitingBadge"), tone: "warning", detail: t("popup.needsAnswer"), progress: null };
  }
  const badge = t(STATE_VIEW[task.state].badge);
  const tone = STATE_VIEW[task.state].tone;
  const pct = percent(task.downloaded, task.total);
  const pctText = pct === null ? "" : `${formatPercent(Math.floor(pct))} · `;
  switch (task.state) {
    case "completed":
      return { label, badge, tone, detail: formatBytes(task.total ?? task.downloaded), progress: null };
    case "failed": {
      const raw = task.error ? sentence(task.error) : undefined;
      const worded = kindText(task.errorKind, task.error);
      if (worded) return { label, badge, tone, detail: worded, progress: null, ...(raw ? { technical: raw } : {}) };
      return { label, badge, tone, detail: raw ?? t("popup.downloadFailed"), progress: null };
    }
    case "canceled":
      return { label, badge, tone, detail: t("popup.canceledInApp"), progress: null };
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
    return { tone: "muted", title: t("status.checking"), detail: "", guide: false };
  }
  switch (status.state) {
    case "connected":
      return {
        tone: "positive",
        title: t("status.connected"),
        detail: t("status.connectedDetail", { app: status.appVersion, bridge: status.hostVersion }),
        guide: false,
      };
    case "app_not_running":
      return {
        tone: "warning",
        title: t("status.appNotRunning"),
        detail: t("status.appNotRunningDetail"),
        guide: false,
      };
    case "host_missing":
      return {
        tone: "danger",
        title: t("status.hostMissing"),
        detail: t("status.hostMissingDetail"),
        command: "downpipe-native-host install",
        guide: true,
      };
    case "host_forbidden": {
      const flag = extension.firefox ? "--firefox-extension-id" : "--extension-id";
      return {
        tone: "danger",
        title: t("status.hostForbidden"),
        detail: t("status.hostForbiddenDetail"),
        command: `downpipe-native-host install ${flag} ${extension.id}`,
        guide: true,
      };
    }
    case "error":
      return { tone: "danger", title: t("status.error"), detail: status.message, guide: true };
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
    detail: sendable ? parts.join(" · ") : t("media.updateForStreams"),
    sendable,
  };
}

/**
 * The row for a video page (YouTube) in the media list. `pages`: whether
 * the connected Downpipe has the site provider.
 */
export function pageRowView(page: PageItem, pages: boolean): MediaRowView {
  return {
    label: page.title ?? t("media.thisSiteVideo", { site: page.site }),
    // Short: the row leaves the detail little room. The quality is chosen in the app.
    detail: pages ? t("media.siteVideo", { site: page.site }) : t("media.updateForPages"),
    sendable: pages,
  };
}

/** What the media list says when it has no rows. */
export function mediaEmptyText(enabled: boolean, excludedBy?: string): string {
  if (!enabled) return t("media.emptyOff");
  if (excludedBy) return t("media.emptyExcluded", { site: excludedBy });
  return t("media.emptyNothing");
}
