// The site provider's tools (yt-dlp, Deno or Node, FFmpeg) in the settings:
// what each row says and what the download button fetches.
import { currentLocale, t } from "../i18n/core";
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

function row(name: string, tool: ToolStatus, missing: string): ToolRow {
  if (!tool.path) {
    return { name, badge: t("media.tools.notFound"), tone: "warning", detail: null, problem: tool.problem ?? missing };
  }
  const where = tool.source ? t(`media.tools.sources.${tool.source}`) : null;
  const version = tool.version ? t("media.tools.version", { version: tool.version }) : null;
  const detail = [tool.path, version, where].filter(Boolean).join(" · ");
  return tool.problem
    ? { name, badge: t("media.tools.problem"), tone: "warning", detail, problem: tool.problem }
    : { name, badge: t("media.tools.found"), tone: "positive", detail, problem: null };
}

export function toolRows(status: ToolsStatus): ToolRow[] {
  const runtime = status.js_kind === "node" ? "Node" : "Deno";
  return [
    row("yt-dlp", status.yt_dlp, t("media.tools.missing.ytDlp")),
    row(runtime, status.js, t("media.tools.missing.runtime")),
    row("FFmpeg", status.ffmpeg, t("media.tools.missing.ffmpeg")),
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
  const update = tools.every((tool) => (tool === "yt_dlp" ? status.yt_dlp : status.js).source === "managed");
  // The language's own "a and b", so no list words need translating.
  const names = new Intl.ListFormat(currentLocale(), { type: "conjunction" }).format(
    tools.map((tool) => (tool === "yt_dlp" ? "yt-dlp" : "Deno")),
  );
  return t(update ? "media.tools.update" : "media.tools.download", { tools: names });
}
