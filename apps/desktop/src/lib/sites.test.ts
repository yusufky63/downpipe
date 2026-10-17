import { describe, expect, it } from "vitest";
// crates/providers/src/pages.rs reads the same cases, so the Add dialog asks
// for a quality on exactly the links the app downloads with yt-dlp.
import cases from "../../../../crates/providers/tests/fixtures/pages.json";
import { sitePage } from "./sites";

describe("sitePage", () => {
  it.each(cases)("$url", ({ url, site, canonical }) => {
    const page = sitePage(url);
    expect(page?.site ?? null).toBe(site);
    expect(page?.url ?? null).toBe(canonical);
  });

  it("keeps the video id and ignores surrounding space", () => {
    expect(sitePage("  https://youtu.be/jNQXAC9IVRw \n")?.id).toBe("jNQXAC9IVRw");
    expect(sitePage("")).toBeNull();
  });
});
