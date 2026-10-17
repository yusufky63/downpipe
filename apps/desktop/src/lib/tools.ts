// The site provider's tools (yt-dlp, Deno or Node, FFmpeg) in the settings:
// what each row says and what the download button fetches.
import type { SiteTool, ToolStatus, ToolsStatus } from "./api";
import type { Tone } from "./tasks";

export interface ToolRow {
  name: string;
  badge: string;
  tone: Tone;
  /** Path, version and where it was found. */
  detail: string | null;
  problem: string | null;
}

const SOURCES = { settings: "set in Settings", managed: "downloaded by Downpipe", path: "on PATH" } as const;

function row(name: string, tool: ToolStatus, missing: string): ToolRow {
  if (!tool.path) {
    return { name, badge: "Not found", tone: "warning", detail: null, problem: tool.problem ?? missing };
  }
  const where = tool.source ? SOURCES[tool.source] : null;
  const detail = [tool.path, tool.version ? `version ${tool.version}` : null, where].filter(Boolean).join(" · ");
  return tool.problem
    ? { name, badge: "Problem", tone: "warning", detail, problem: tool.problem }
    : { name, badge: "Found", tone: "positive", detail, problem: null };
}

export function toolRows(status: ToolsStatus): ToolRow[] {
  const runtime = status.js_kind === "node" ? "Node" : "Deno";
  return [
    row("yt-dlp", status.yt_dlp, "Needed for videos on pages."),
    row(runtime, status.js, "Without Deno (or Node) yt-dlp misses some YouTube qualities."),
    row("FFmpeg", status.ffmpeg, "Without it only Audio only and qualities with their own sound work."),
  ];
}

/**
 * What the button downloads: yt-dlp always (a new one fixes most YouTube
 * failures), and Deno, yt-dlp's default runtime, unless a working Deno comes
 * from elsewhere or a Deno path is set, which would win over a download
 * anyway. Node only stands in while there is no Deno.
 */
export function toolsToInstall(status: ToolsStatus, denoConfigured: boolean): SiteTool[] {
  const tools: SiteTool[] = ["yt_dlp"];
  const js = status.js;
  const denoElsewhere = status.js_kind === "deno" && js.path !== null && js.problem === null && js.source !== "managed";
  if (!denoConfigured && !denoElsewhere) tools.push("deno");
  return tools;
}

/** "Download yt-dlp and Deno", or "Update yt-dlp" when Downpipe's own copy is there. */
export function installButtonLabel(status: ToolsStatus, tools: SiteTool[]): string {
  const update = tools.every((t) => (t === "yt_dlp" ? status.yt_dlp : status.js).source === "managed");
  const names = tools.map((t) => (t === "yt_dlp" ? "yt-dlp" : "Deno")).join(" and ");
  return `${update ? "Update" : "Download"} ${names}`;
}
