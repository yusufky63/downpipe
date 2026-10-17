import { describe, expect, it } from "vitest";
// crates/providers/src/pages.rs reads the same cases, so the extension
// offers its button on exactly the pages the app accepts.
import cases from "../../../../crates/providers/tests/fixtures/pages.json";
import { isSiteMedia, pageTitle, sitePage } from "./sites";

describe("sitePage", () => {
  it.each(cases)("$url", ({ url, site, canonical }) => {
    const page = sitePage(url);
    expect(page?.site ?? null).toBe(site);
    expect(page?.url ?? null).toBe(canonical);
  });

  it("keeps the video id", () => {
    expect(sitePage("https://youtu.be/jNQXAC9IVRw")?.id).toBe("jNQXAC9IVRw");
    expect(sitePage(undefined)).toBeNull();
  });
});

describe("isSiteMedia", () => {
  it("knows YouTube's video servers", () => {
    expect(isSiteMedia("https://rr1---sn-4g5e6nsz.googlevideo.com/videoplayback?id=1")).toBe(true);
    expect(isSiteMedia("https://googlevideo.com/x")).toBe(true);
    expect(isSiteMedia("https://notgooglevideo.com/x.mp4")).toBe(false);
    expect(isSiteMedia("https://cdn.example.org/clip.mp4")).toBe(false);
  });
});

describe("pageTitle", () => {
  it("drops the site name and the notification count", () => {
    expect(pageTitle("(3) Me at the zoo - YouTube")).toBe("Me at the zoo");
    expect(pageTitle("Song - YouTube Music")).toBe("Song");
    expect(pageTitle("A - B")).toBe("A - B");
    // Not yet the video's title.
    expect(pageTitle("YouTube")).toBeUndefined();
    expect(pageTitle("(1) YouTube")).toBeUndefined();
    expect(pageTitle(" ")).toBeUndefined();
    expect(pageTitle(undefined)).toBeUndefined();
  });
});
