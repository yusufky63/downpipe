// The site provider's tools (yt-dlp, Deno or Node, FFmpeg): what each row in
// the settings says and offers, and what a dialog that needs a missing tool
// offers to download.
import { currentLocale, t } from "../i18n/core";
import type { SiteTool, ToolStatus, ToolsProgress, ToolsStatus } from "./api";
import { formatDecimal } from "./format";
import type { Tone } from "./tasks";

export const TOOL_NAMES: Record<SiteTool, string> = { yt_dlp: "yt-dlp", deno: "Deno", ffmpeg: "FFmpeg" };

export interface ToolRow {
  tool: SiteTool;
  name: string;
  badge: string;
  tone: Tone;
  /** Path, version and where it was found. */
  detail: string | null;
  problem: string | null;
  /**
   * The row's button: download a missing tool, or update Downpipe's own
   * copy. Null when a download would not be used (a path is set in the
   * settings, or a working copy is found elsewhere) or Downpipe has no
   * build for this system.
   */
  action: "download" | "update" | null;
}

/** The tool's status; Deno's row shows Node while Node stands in. */
function statusOf(status: ToolsStatus, tool: SiteTool): ToolStatus {
  return tool === "yt_dlp" ? status.yt_dlp : tool === "deno" ? status.js : status.ffmpeg;
}

/** A path set in the settings wins over a download, found or not. */
function configured(tool: ToolStatus): boolean {
  return tool.source === "settings" || (tool.path === null && tool.problem !== null);
}

function actionFor(status: ToolsStatus, tool: SiteTool): ToolRow["action"] {
  const s = statusOf(status, tool);
  if (status.sizes[tool] === null || configured(s)) return null;
  if (s.source === "managed") return "update";
  // Node only stands in for Deno, yt-dlp's default runtime.
  if (tool === "deno" && status.js_kind === "node") return "download";
  // Downpipe's copy is used before one on PATH, so it fixes a broken one.
  return s.path === null || s.problem !== null ? "download" : null;
}

function row(status: ToolsStatus, tool: SiteTool, name: string, missing: string): ToolRow {
  const s = statusOf(status, tool);
  const action = actionFor(status, tool);
  if (!s.path) {
    return { tool, name, badge: t("media.tools.notFound"), tone: "warning", detail: null, problem: s.problem ?? missing, action };
  }
  const where = s.source ? t(`media.tools.sources.${s.source}`) : null;
  const version = s.version ? t("media.tools.version", { version: s.version }) : null;
  const detail = [s.path, version, where].filter(Boolean).join(" · ");
  return s.problem
    ? { tool, name, badge: t("media.tools.problem"), tone: "warning", detail, problem: s.problem, action }
    : { tool, name, badge: t("media.tools.found"), tone: "positive", detail, problem: null, action };
}

export function toolRows(status: ToolsStatus): ToolRow[] {
  const runtime = status.js_kind === "node" ? "Node" : "Deno";
  // Where there is no build to download (FFmpeg on macOS), say how to get one.
  const ffmpegMissing = status.sizes.ffmpeg === null ? t("media.tools.missing.ffmpegManual") : t("media.tools.missing.ffmpeg");
  return [
    row(status, "yt_dlp", "yt-dlp", t("media.tools.missing.ytDlp")),
    row(status, "deno", runtime, t("media.tools.missing.runtime")),
    row(status, "ffmpeg", "FFmpeg", ffmpegMissing),
  ];
}

/** What "Install all" downloads: every tool that is missing and that Downpipe can download. */
export function toolsToInstall(status: ToolsStatus): SiteTool[] {
  return toolRows(status)
    .filter((r) => r.action === "download")
    .map((r) => r.tool);
}

/**
 * The tools a download needs that are missing and that Downpipe can
 * download. A video page needs yt-dlp and a JavaScript runtime (Node will
 * do), and FFmpeg for most qualities; a stream's separate tracks need FFmpeg.
 */
export function missingTools(status: ToolsStatus, needs: readonly SiteTool[]): SiteTool[] {
  return needs.filter((tool) => {
    const s = statusOf(status, tool);
    return s.path === null && status.sizes[tool] !== null && !configured(s);
  });
}

/** Whether a video page cannot be read as it is: yt-dlp, or any JavaScript runtime, is missing. */
export function pageNeedsTools(missing: readonly SiteTool[]): boolean {
  return missing.includes("yt_dlp") || missing.includes("deno");
}

/** The tools a video page asks for: all three, so one download sets it up for every quality. */
export const PAGE_TOOLS: readonly SiteTool[] = ["yt_dlp", "deno", "ffmpeg"];

/** "yt-dlp, Deno and FFmpeg", in the language's own list words. */
export function toolList(tools: readonly SiteTool[]): string {
  return new Intl.ListFormat(currentLocale(), { type: "conjunction" }).format(tools.map((tool) => TOOL_NAMES[tool]));
}

function mib(bytes: number): string {
  return formatDecimal(Math.round(bytes / (1024 * 1024)), 0);
}

/** Whole MiB, at least one: the sizes are estimates. */
export function approxSize(bytes: number): string {
  return `${mib(Math.max(bytes, 1024 * 1024))} MiB`;
}

/** About how much downloading `tools` is. */
export function downloadSize(status: ToolsStatus, tools: readonly SiteTool[]): string {
  return approxSize(tools.reduce((sum, tool) => sum + (status.sizes[tool] ?? 0), 0));
}

/** "Downloading FFmpeg… 45 of 163 MiB", then "Installing FFmpeg…" while it is unpacked and checked. */
export function progressText(p: ToolsProgress): string {
  const tool = TOOL_NAMES[p.tool];
  if (p.total !== null && p.done >= p.total) return t("media.tools.installing", { tool });
  return p.total !== null
    ? t("media.tools.progress", { tool, done: mib(p.done), total: approxSize(p.total) })
    : t("media.tools.progressUnknown", { tool, done: `${mib(p.done)} MiB` });
}
