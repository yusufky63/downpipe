// What happens after a download completes (downpipe_core::post): which
// command and extraction a download gets, and its report as texts.
import { t } from "../i18n/core";
import type { CommandReport, ExtractReport, PostActions, ScanReport, Settings, Task } from "./api";
import type { Tone } from "./tasks";

/** Name endings Downpipe extracts itself, as the core's `ArchiveKind::of`. */
const ARCHIVE_ENDINGS = [".tar.gz", ".tar.xz", ".tgz", ".txz", ".tar", ".zip", ".7z"];

export function isArchive(name: string): boolean {
  const lower = name.toLowerCase();
  return ARCHIVE_ENDINGS.some((end) => lower.length > end.length && lower.endsWith(end));
}

/** Extensions in a category that make its Extract switch mean something. */
const ARCHIVE_EXTENSIONS = new Set(["zip", "tar", "tgz", "txz", "7z", "gz", "xz"]);

export function hasArchiveTypes(extensions: string[]): boolean {
  return extensions.some((e) => ARCHIVE_EXTENSIONS.has(e.toLowerCase()));
}

export const NO_POST_ACTIONS: PostActions = { command: "", extract: null };

/** The command a download gets when it has none of its own: its queue's, else the one in Settings. */
export function inheritedCommand(settings: Settings, queue: string): string {
  const own = settings.queues.find((q) => q.id === queue)?.command.trim() ?? "";
  return own || settings.post.command.trim();
}

/**
 * Whether an archive of this name is extracted when the download does not
 * say: its category's switch, whether or not files are sorted into folders.
 */
export function categoryExtracts(settings: Settings, name: string): boolean {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return false;
  const ext = name.slice(dot + 1).toLowerCase();
  return settings.categories.some((c) => c.extract && c.extensions.includes(ext));
}

/** The placeholder the command field shows: what runs when it stays empty. */
export function commandPlaceholder(inherited: string): string {
  return inherited ? t("post.fields.inherited", { command: inherited }) : t("post.fields.nothing");
}

export function threatFound(task: Task): boolean {
  return task.post?.scan?.result === "threat";
}

/** The badge of a completed download whose post-download steps matter now. */
export function postBadge(task: Task): { label: string; tone: Tone } | null {
  if (task.post_step) return { label: t(`post.steps.${task.post_step}`), tone: "primary" };
  if (threatFound(task)) return { label: t("post.threat"), tone: "danger" };
  return null;
}

/** One line for the list row: a threat, or a step that did not work; else null. */
export function postProblem(task: Task): string | null {
  const report = task.post;
  if (!report) return null;
  if (report.scan?.result === "threat") {
    return t("post.summary.threat", { threat: report.scan.detail ?? t("post.scan.unnamed") });
  }
  if (report.extract?.result === "failed") return t("post.summary.notExtracted");
  if (report.command && report.command.result !== "ok") return t("post.summary.commandFailed");
  return null;
}

export function scanText(scan: ScanReport): string {
  const scanner = scan.scanner || t("post.scan.theScanner");
  switch (scan.result) {
    case "clean":
      return t("post.scan.clean", { scanner });
    case "threat":
      return t("post.scan.threat", { scanner, threat: scan.detail ?? t("post.scan.unnamed") });
    case "missing":
      return scan.scanner ? t("post.scan.missingNamed", { scanner }) : t("post.scan.missing");
    case "failed":
      return t("post.scan.failed", { scanner, detail: scan.detail ?? "" });
  }
}

export function extractText(extract: ExtractReport): string {
  if (extract.result === "failed") {
    const reason = extract.error ? t(`post.extract.errors.${extract.error}`) : (extract.detail ?? "");
    return t("post.extract.failed", { reason });
  }
  return [
    t("post.extract.done", { count: extract.files }),
    extract.skipped > 0 ? t("post.extract.skipped", { count: extract.skipped }) : "",
    extract.archive_deleted ? t("post.extract.deleted") : "",
  ]
    .filter(Boolean)
    .join(" ");
}

export function commandText(command: CommandReport): string {
  const program = command.program || t("post.command.theCommand");
  switch (command.result) {
    case "ok":
      return t("post.command.ok", { program });
    case "failed":
      return command.exit_code === undefined
        ? t("post.command.ended", { program })
        : t("post.command.failed", { program, code: command.exit_code });
    case "timed_out":
      return t("post.command.timedOut", { program });
    case "not_started":
      return t("post.command.notStarted", { program, detail: command.detail ?? "" });
    case "invalid":
      return t("post.command.invalid", { detail: command.detail ?? "" });
  }
}

export function reportTone(result: string): Tone {
  switch (result) {
    case "clean":
    case "done":
    case "ok":
      return "positive";
    case "threat":
      return "danger";
    default:
      return "warning";
  }
}
