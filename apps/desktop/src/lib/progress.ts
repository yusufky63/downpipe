// What a download's progress window says (IDM's download dialog), from the
// task alone. The window itself is ProgressWindow.tsx.
import type { Task } from "./api";
import { formatBytes, formatDuration, formatSpeed, percent, sentence } from "./format";
import { displayName, isRunning } from "./tasks";

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
  const what = task === null ? null : task.site ? task.site.site : task.media ? `${task.media.kind.toUpperCase()} stream` : null;
  const phase: Record<ProgressPhase, string> = {
    active: task?.state === "queued" ? "Queued" : "Downloading",
    paused: "Paused",
    done: "Download complete",
    failed: "Download failed",
    canceled: "Canceled",
    removed: "Removed",
  };
  const head = phase[progressPhase(task)];
  return what ? `${head} · ${what}` : head;
}

/** IDM's status line. */
export function statusText(task: Task): string {
  switch (task.state) {
    case "queued":
      return "Waiting for a free place in the queue";
    case "starting":
      return "Connecting";
    case "downloading":
      if (task.fragments) return `Receiving data · ${task.fragments.done} of ${task.fragments.total} fragments`;
      if (task.connections > 1) return `Receiving data · ${task.connections} connections`;
      return "Receiving data";
    case "waiting":
      return "Connection lost, trying again";
    case "verifying":
      return "Checking the file";
    case "paused":
      return "Paused";
    case "completed":
      return "Finished";
    case "failed":
      return task.error ? sentence(task.error) : "Failed";
    case "canceled":
      return "Canceled; what was downloaded is deleted";
  }
}

/** Downloaded out of the size, and the percent; a stream's size is an estimate until it ends. */
export function sizeText(task: Task): string {
  if (task.state === "completed") return formatBytes(task.total ?? task.downloaded);
  const pct = percent(task.downloaded, task.total);
  if (task.total === null || pct === null) return formatBytes(task.downloaded);
  const approx = (task.media !== null || task.site !== null) && task.total > 0 ? "~" : "";
  return `${formatBytes(task.downloaded)} of ${approx}${formatBytes(task.total)} · ${pct.toFixed(1)}%`;
}

/** Speed and time left, while it runs. */
export function speedText(task: Task): string | null {
  if (!isRunning(task.state)) return null;
  const speed = formatSpeed(task.speed);
  return task.eta_secs === null ? speed : `${speed} · ${formatDuration(task.eta_secs)} left`;
}

/** IDM's "Resume capability", known once the server answered. */
export function resumeText(task: Task): string | null {
  if (task.state !== "downloading" && task.state !== "verifying") return null;
  return task.resumable ? "Yes" : "No: pausing starts it over";
}

/**
 * The window's title. In the taskbar it is all that is seen of a download
 * running behind other windows, so it carries the progress.
 */
export function windowTitle(task: Task | null): string {
  if (task === null) return "Removed - Downpipe";
  const name = displayName(task);
  const pct = percent(task.downloaded, task.total);
  const what: Record<ProgressPhase, string> = {
    active:
      task.state === "queued" ? `Queued · ${name}` : pct !== null && task.total !== null && task.total > 0 ? `${Math.floor(pct)}% ${name}` : name,
    paused: `Paused · ${name}`,
    done: `Done · ${name}`,
    failed: `Failed · ${name}`,
    canceled: `Canceled · ${name}`,
    removed: name,
  };
  return `${what[progressPhase(task)]} - Downpipe`;
}
