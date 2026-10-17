import { describe, expect, it } from "vitest";
import type { ToolStatus, ToolsStatus } from "./api";
import { installButtonLabel, toolRows, toolsToInstall } from "./tools";

const none: ToolStatus = { path: null, source: null, version: null, problem: null };
const found = (patch: Partial<ToolStatus>): ToolStatus => ({ path: "C:/t/x.exe", source: "path", version: "1", problem: null, ...patch });

const status = (patch: Partial<ToolsStatus>): ToolsStatus => ({
  yt_dlp: none,
  js: none,
  js_kind: null,
  ffmpeg: none,
  managed_dir: "C:/Users/me/AppData/Roaming/dev.downpipe.app/tools",
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
    expect(rows.map((r) => [r.name, r.badge])).toEqual([
      ["yt-dlp", "Found"],
      ["Node", "Found"],
      ["FFmpeg", "Not found"],
    ]);
    expect(rows[0]?.detail).toBe("C:/t/yt-dlp.exe · version 2026.08.19 · downloaded by Downpipe");
    expect(rows[2]?.problem).toContain("Audio only");
  });

  it("flags a tool that is found but not usable", () => {
    const [, deno] = toolRows(status({ js: found({ version: "2.2.0", problem: "yt-dlp needs Deno 2.3.0 or newer." }), js_kind: "deno" }));
    expect(deno).toMatchObject({ name: "Deno", badge: "Problem", tone: "warning" });
  });
});

describe("the download button", () => {
  it("fetches both when nothing is set up", () => {
    const s = status({});
    expect(toolsToInstall(s, false)).toEqual(["yt_dlp", "deno"]);
    expect(installButtonLabel(s, ["yt_dlp", "deno"])).toBe("Download yt-dlp and Deno");
  });

  it("leaves a working Deno from elsewhere alone", () => {
    const s = status({ js: found({ source: "path" }), js_kind: "deno" });
    expect(toolsToInstall(s, false)).toEqual(["yt_dlp"]);
    // A Deno path in the settings wins over a download, so none is fetched.
    expect(toolsToInstall(status({}), true)).toEqual(["yt_dlp"]);
    // Node only stands in; Deno is still offered.
    expect(toolsToInstall(status({ js: found({ source: "path" }), js_kind: "node" }), false)).toEqual(["yt_dlp", "deno"]);
  });

  it("updates Downpipe's own copies", () => {
    const s = status({ yt_dlp: found({ source: "managed" }), js: found({ source: "managed" }), js_kind: "deno" });
    expect(toolsToInstall(s, false)).toEqual(["yt_dlp", "deno"]);
    expect(installButtonLabel(s, ["yt_dlp", "deno"])).toBe("Update yt-dlp and Deno");
    expect(installButtonLabel(status({ yt_dlp: found({ source: "path" }) }), ["yt_dlp"])).toBe("Download yt-dlp");
  });
});
