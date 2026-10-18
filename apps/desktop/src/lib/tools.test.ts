import { describe, expect, it } from "vitest";
import type { ToolStatus, ToolsStatus } from "./api";
import { PAGE_TOOLS, downloadSize, missingTools, pageNeedsTools, progressText, toolList, toolRows, toolsToInstall } from "./tools";

const none: ToolStatus = { path: null, source: null, version: null, problem: null };
const found = (patch: Partial<ToolStatus>): ToolStatus => ({ path: "C:/t/x.exe", source: "path", version: "1", problem: null, ...patch });
const MB = 1_000_000;

const status = (patch: Partial<ToolsStatus>): ToolsStatus => ({
  yt_dlp: none,
  js: none,
  js_kind: null,
  ffmpeg: none,
  managed_dir: "C:/Users/me/AppData/Roaming/dev.downpipe.app/tools",
  sizes: { yt_dlp: 18 * MB, deno: 42 * MB, ffmpeg: 171 * MB },
  yt_dlp_checked: null,
  ...patch,
});

describe("tool rows", () => {
  it("says what is found, where, and what is missing", () => {
    const rows = toolRows(
      status({
        yt_dlp: found({ path: "C:/t/yt-dlp.exe", version: "2026.08.19", source: "managed" }),
        js: found({ path: "C:/n/node.exe", version: "24.19.0" }),
        js_kind: "node",
      }),
    );
    expect(rows.map((r) => [r.name, r.badge, r.action])).toEqual([
      ["yt-dlp", "Found", "update"],
      // Node only stands in: Deno, yt-dlp's default, is still offered.
      ["Node", "Found", "download"],
      ["FFmpeg", "Not found", "download"],
    ]);
    expect(rows[0]?.detail).toBe("C:/t/yt-dlp.exe · version 2026.08.19 · downloaded by Downpipe");
    expect(rows[2]?.problem).toContain("Audio only");
  });

  it("flags a tool that is found but not usable, and offers a download over it", () => {
    const [, deno] = toolRows(status({ js: found({ version: "2.2.0", problem: "yt-dlp needs Deno 2.3.0 or newer." }), js_kind: "deno" }));
    expect(deno).toMatchObject({ name: "Deno", badge: "Problem", tone: "warning", action: "download" });
  });

  it("offers nothing a download would not replace", () => {
    const rows = toolRows(
      status({
        // A working copy on PATH, a path in the settings, and one set but missing.
        yt_dlp: found({ source: "path" }),
        js: found({ source: "settings" }),
        js_kind: "deno",
        ffmpeg: { ...none, problem: "There is no ffmpeg at this path." },
      }),
    );
    expect(rows.map((r) => r.action)).toEqual([null, null, null]);
  });

  it("says how to get FFmpeg where Downpipe has no build", () => {
    const [, , ffmpeg] = toolRows(status({ sizes: { yt_dlp: 37 * MB, deno: 38 * MB, ffmpeg: null } }));
    expect(ffmpeg?.action).toBeNull();
    expect(ffmpeg?.problem).toContain("brew install ffmpeg");
  });
});

describe("downloads", () => {
  it("installs everything missing at once", () => {
    const s = status({});
    expect(toolsToInstall(s)).toEqual(["yt_dlp", "deno", "ffmpeg"]);
    expect(downloadSize(s, toolsToInstall(s))).toBe("220 MiB");
    expect(toolsToInstall(status({ yt_dlp: found({ source: "managed" }) }))).toEqual(["deno", "ffmpeg"]);
  });

  it("offers a video page what it lacks", () => {
    const missing = missingTools(status({ js: found({ source: "path" }), js_kind: "node" }), PAGE_TOOLS);
    // Node does for reading the page; it is not fetched again.
    expect(missing).toEqual(["yt_dlp", "ffmpeg"]);
    expect(pageNeedsTools(missing)).toBe(true);
    expect(pageNeedsTools(["ffmpeg"])).toBe(false);
    expect(toolList(missing)).toBe("yt-dlp and FFmpeg");
    // Nothing to offer where there is no build, or a path is set.
    expect(missingTools(status({ sizes: { yt_dlp: 1, deno: 1, ffmpeg: null } }), ["ffmpeg"])).toEqual([]);
    expect(missingTools(status({ ffmpeg: { ...none, problem: "There is no ffmpeg at this path." } }), ["ffmpeg"])).toEqual([]);
  });

  it("tells how far a download is", () => {
    const MiB = 1024 * 1024;
    expect(progressText({ tool: "ffmpeg", done: 45 * MiB, total: 163 * MiB })).toBe("Downloading FFmpeg… 45 of 163 MiB");
    expect(progressText({ tool: "yt_dlp", done: 3 * MiB, total: null })).toBe("Downloading yt-dlp… 3 MiB");
    expect(progressText({ tool: "deno", done: 40 * MiB, total: 40 * MiB })).toBe("Installing Deno…");
  });
});
