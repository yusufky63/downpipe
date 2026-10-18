// What a download's progress window says (IDM's download dialog), from the
// task alone. The window itself is ProgressWindow.tsx.
import { t, type MessageKey } from "../i18n/core";
import type { Task } from "./api";
import { formatBytes, formatDecimal, formatDuration, formatSpeed, percent } from "./format";
import { displayName, errorText, isRunning, waitingText } from "./tasks";
import { clockText } from "./until";

/** The download a window follows, from its label (`progress-<id>`, src-tauri/src/progress.rs). */
export function progressId(label: string): string | null {
  return label.startsWith("progress-") && label.length > "progress-".length ? label.slice("progress-".length) : null;
}

/**
 * `active` covers queued and running downloads; `removed` is a download
 * that left the list while its window was open.
 */
export type ProgressPhase = "active" | "paused" | "done" | "failed" | "canceled" | "removed";

export function progressPhase(task: Task | null): ProgressPhase {
  if (task === null) return "removed";
  switch (task.state) {
    case "completed":
      return "done";
    case "failed":
      return "failed";
    case "canceled":
      return "canceled";
    case "paused":
      return "paused";
    default:
      return "active";
  }
}

/** The eyebrow: what is happening, and to what kind of download. */
export function progressEyebrow(task: Task | null): string {
  const what =
    task === null ? null : task.site ? task.site.site : task.media ? t("confirm.stream", { kind: task.media.kind.toUpperCase() }) : null;
  const phase: Record<ProgressPhase, MessageKey> = {
    active: task?.state === "queued" ? "confirm.progress.eyebrow.queued" : "confirm.progress.eyebrow.downloading",
    paused: "confirm.progress.eyebrow.paused",
    done: "confirm.progress.eyebrow.done",
    failed: "confirm.progress.eyebrow.failed",
    canceled: "confirm.progress.eyebrow.canceled",
    removed: "confirm.progress.eyebrow.removed",
  };
  const head = t(phase[progressPhase(task)]);
  return what ? `${head} · ${what}` : head;
}

/** IDM's status line. */
export function statusText(task: Task): string {
  // A torrent counts peers, not connections, and may seed once finished.
  if (task.torrent && task.swarm) {
    if (task.swarm.fetching_metadata) return t("torrent.progress.fetching");
    if (task.state === "completed" && task.swarm.seeding) return t("torrent.progress.seeding");
    if (task.state === "downloading") return t("torrent.progress.peers", { count: task.swarm.peers });
  }
  switch (task.state) {
    case "queued":
      return t("confirm.progress.status.queued");
    case "starting":
      return t("confirm.progress.status.connecting");
    case "downloading":
      if (task.fragments) return t("confirm.progress.status.fragments", { done: task.fragments.done, count: task.fragments.total });
      if (task.connections > 1) return t("confirm.progress.status.connections", { count: task.connections });
      return t("confirm.progress.status.receiving");
    case "waiting":
      return waitingText(task) ?? t("confirm.progress.status.waiting");
    case "verifying":
      return t("confirm.progress.status.verifying");
    case "paused":
      return task.resume_at !== null
        ? t("confirm.progress.status.pausedUntil", { time: clockText(task.resume_at) })
        : t("confirm.progress.status.paused");
    case "completed":
      return t("confirm.progress.status.finished");
    case "failed":
      return errorText(task) ?? t("confirm.progress.status.failed");
    case "canceled":
      return t("confirm.progress.status.canceled");
  }
}

/** Downloaded out of the size, and the percent; a stream's size is an estimate until it ends. */
export function sizeText(task: Task): string {
  if (task.state === "completed") return formatBytes(task.total ?? task.downloaded);
  const pct = percent(task.downloaded, task.total);
  if (task.total === null || pct === null) return formatBytes(task.downloaded);
  const approx = (task.media !== null || task.site !== null) && task.total > 0 ? "~" : "";
  return t("confirm.progress.size", {
    downloaded: formatBytes(task.downloaded),
    approx,
    total: formatBytes(task.total),
    // The catalog places the sign: "45.3%", Turkish "%45,3", French "45,3 %".
    percent: formatDecimal(pct, 1),
  });
}

/** Speed and time left, while it runs. */
export function speedText(task: Task): string | null {
  if (!isRunning(task.state)) return null;
  const speed = formatSpeed(task.speed);
  return task.eta_secs === null ? speed : t("confirm.progress.timeLeft", { speed, time: formatDuration(task.eta_secs) });
}

/** The download's own speed limit, while it has one and is not finished. */
export function limitText(task: Task): string | null {
  if (task.speed_limit === null || task.state === "completed" || task.state === "canceled") return null;
  return formatSpeed(task.speed_limit);
}

/** IDM's "Resume capability", known once the server answered. */
export function resumeText(task: Task): string | null {
  if (task.state !== "downloading" && task.state !== "verifying") return null;
  return task.resumable ? t("confirm.progress.resumeYes") : t("confirm.progress.resumeNo");
}

/**
 * The window's title. In the taskbar it is all that is seen of a download
 * running behind other windows, so it carries the progress.
 */
export function windowTitle(task: Task | null): string {
  if (task === null) return t("confirm.progress.title.window", { title: t("confirm.progress.title.removed") });
  const name = displayName(task);
  const pct = percent(task.downloaded, task.total);
  const what: Record<ProgressPhase, string> = {
    active:
      task.state === "queued"
        ? t("confirm.progress.title.queued", { name })
        : pct !== null && task.total !== null && task.total > 0
          ? t("confirm.progress.title.percent", { percent: formatDecimal(Math.floor(pct), 0), name })
          : name,
    paused: t("confirm.progress.title.paused", { name }),
    done: t("confirm.progress.title.done", { name }),
    failed: t("confirm.progress.title.failed", { name }),
    canceled: t("confirm.progress.title.canceled", { name }),
    removed: name,
  };
  return t("confirm.progress.title.window", { title: what[progressPhase(task)] });
}
