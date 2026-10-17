import { describe, expect, it } from "vitest";

import { MAX_EXPANDED, expandPatterns, linksFromList } from "./patterns";

describe("expandPatterns", () => {
  it("expands numeric ranges and keeps zero padding", () => {
    const { urls, expanded } = expandPatterns("https://example.org/img[008-011].jpg");
    expect(expanded).toBe(true);
    expect(urls).toEqual([
      "https://example.org/img008.jpg",
      "https://example.org/img009.jpg",
      "https://example.org/img010.jpg",
      "https://example.org/img011.jpg",
    ]);
    expect(expandPatterns("https://e.org/p[9-11]").urls).toEqual(["https://e.org/p9", "https://e.org/p10", "https://e.org/p11"]);
  });

  it("expands letters, descending ranges and several ranges", () => {
    expect(expandPatterns("https://e.org/part[a-c].rar").urls).toEqual([
      "https://e.org/parta.rar",
      "https://e.org/partb.rar",
      "https://e.org/partc.rar",
    ]);
    expect(expandPatterns("https://e.org/[3-1]").urls).toEqual(["https://e.org/3", "https://e.org/2", "https://e.org/1"]);
    expect(expandPatterns("https://e.org/s[1-2]/e[01-02].mp4").urls).toEqual([
      "https://e.org/s1/e01.mp4",
      "https://e.org/s1/e02.mp4",
      "https://e.org/s2/e01.mp4",
      "https://e.org/s2/e02.mp4",
    ]);
  });

  it("leaves plain links and other text alone", () => {
    const r = expandPatterns("https://e.org/a.zip\nnot-a-link https://e.org/b.zip");
    expect(r.expanded).toBe(false);
    expect(r.urls).toEqual(["https://e.org/a.zip", "not-a-link", "https://e.org/b.zip"]);
  });

  it("caps huge expansions", () => {
    const r = expandPatterns("https://e.org/[1-5000].bin");
    expect(r.urls).toHaveLength(MAX_EXPANDED);
    expect(r.dropped).toBeGreaterThan(0);
  });
});

describe("linksFromList", () => {
  it("drops blank lines and comments", () => {
    expect(linksFromList("# saved\r\nhttps://a.example/1\n\n  https://a.example/2  \n")).toBe("https://a.example/1\nhttps://a.example/2");
  });
});
