import { describe, expect, it } from "vitest";
import { isWebsiteSender, parseWebsiteRequest, sendGate } from "./website";

describe("isWebsiteSender", () => {
  it("accepts pages of downpipe.xyz", () => {
    expect(isWebsiteSender({ url: "https://downpipe.xyz/", origin: "https://downpipe.xyz" })).toBe(true);
    expect(isWebsiteSender({ url: "https://www.downpipe.xyz/?url=x" })).toBe(true);
  });

  it("rejects other sites, look-alikes and extensions", () => {
    expect(isWebsiteSender({ url: "https://evil.example/", origin: "https://evil.example" })).toBe(false);
    expect(isWebsiteSender({ url: "https://downpipe.xyz.evil.example/" })).toBe(false);
    expect(isWebsiteSender({ url: "http://downpipe.xyz/" })).toBe(false);
    // The origin wins over a URL that claims the site.
    expect(isWebsiteSender({ url: "https://downpipe.xyz/", origin: "https://evil.example" })).toBe(false);
    expect(isWebsiteSender({ id: "abc", url: "https://downpipe.xyz/" })).toBe(false);
    expect(isWebsiteSender({})).toBe(false);
    expect(isWebsiteSender({ url: "not a url" })).toBe(false);
  });
});

describe("parseWebsiteRequest", () => {
  it("reads hello and YouTube pages", () => {
    expect(parseWebsiteRequest({ kind: "hello" })).toEqual({ kind: "hello" });
    expect(parseWebsiteRequest({ kind: "send_page", url: "https://youtu.be/dQw4w9WgXcQ", title: "  Song  " })).toEqual({
      kind: "send_page",
      url: "https://youtu.be/dQw4w9WgXcQ",
      title: "Song",
    });
    expect(parseWebsiteRequest({ kind: "send_page", url: "https://www.youtube.com/playlist?list=PLx1y2z3" })).toEqual({
      kind: "send_page",
      url: "https://www.youtube.com/playlist?list=PLx1y2z3",
    });
  });

  it("refuses any other link or request", () => {
    expect(parseWebsiteRequest({ kind: "send_page", url: "https://example.com/file.zip" })).toBeNull();
    expect(parseWebsiteRequest({ kind: "send_page", url: "https://x.com/a/status/1" })).toBeNull();
    expect(parseWebsiteRequest({ kind: "send_page", url: `https://youtu.be/dQw4w9WgXcQ?${"a".repeat(3000)}` })).toBeNull();
    expect(parseWebsiteRequest({ kind: "send_link", url: "https://youtu.be/dQw4w9WgXcQ" })).toBeNull();
    expect(parseWebsiteRequest({ kind: "status" })).toBeNull();
    expect(parseWebsiteRequest("hello")).toBeNull();
    expect(parseWebsiteRequest(null)).toBeNull();
  });

  it("keeps titles short", () => {
    const parsed = parseWebsiteRequest({ kind: "send_page", url: "https://youtu.be/dQw4w9WgXcQ", title: "t".repeat(500) });
    expect(parsed?.kind === "send_page" && parsed.title?.length).toBe(200);
  });
});

describe("sendGate", () => {
  it("lets one send through per gap", () => {
    let now = 0;
    const allow = sendGate(3000, () => now);
    expect(allow()).toBe(true);
    now = 2999;
    expect(allow()).toBe(false);
    now = 3000;
    expect(allow()).toBe(true);
  });
});
