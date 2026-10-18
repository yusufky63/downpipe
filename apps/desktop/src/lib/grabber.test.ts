import { describe, expect, it } from "vitest";

import type { FoundFile, GrabProgress } from "./api";
import { clampInt, filterFiles, grabExtensions, grabFraction, parseExtensions, selectedSize, toggleShown } from "./grabber";

const file = (name: string, size: number | null): FoundFile => ({
  url: `https://site.test/${name}`,
  name,
  extension: name.split(".").pop() ?? "",
  size,
  found_on: "https://site.test/",
});

const progress = (p: Partial<GrabProgress>): GrabProgress => ({
  phase: "pages",
  pages_done: 0,
  pages_total: 0,
  files: 0,
  sized: 0,
  to_size: 0,
  current: null,
  blocked: 0,
  failed: 0,
  ...p,
});

describe("grabber helpers", () => {
  it("cleans typed extensions", () => {
    expect(parseExtensions("*.MP3, .zip  pdf;zip bad/ext toolongextension1")).toEqual(["mp3", "zip", "pdf"]);
    expect(parseExtensions("")).toEqual([]);
  });

  it("joins category and custom types", () => {
    const categories = [
      { name: "Music", extensions: ["mp3", "flac"], folder: "" },
      { name: "Video", extensions: ["mp4"], folder: "" },
    ];
    expect(grabExtensions(categories, new Set(["Music"]), "MP3 iso")).toEqual(["mp3", "flac", "iso"]);
    expect(grabExtensions(categories, new Set(), "")).toEqual([]);
  });

  it("clamps numbers", () => {
    expect(clampInt("9", 1, 5, 2)).toBe(5);
    expect(clampInt("0", 1, 5, 2)).toBe(1);
    expect(clampInt("x", 1, 5, 2)).toBe(2);
  });

  it("totals the chosen files", () => {
    const a = file("a.zip", 100);
    const b = file("b.zip", null);
    const files = [a, b, file("c.zip", 50)];
    const chosen = new Set([a.url, b.url]);
    expect(selectedSize(files, chosen)).toEqual({ bytes: 100, unknown: 1 });
  });

  it("filters and toggles what is shown", () => {
    const song = file("song.mp3", 1);
    const files = [song, file("Paper.pdf", 2)];
    expect(filterFiles(files, "paper")).toHaveLength(1);
    expect(filterFiles(files, "mp3")).toHaveLength(1);
    expect(filterFiles(files, " ")).toHaveLength(2);
    const on = toggleShown(new Set(), files);
    expect(on.size).toBe(2);
    expect(toggleShown(on, files).size).toBe(0);
    expect(toggleShown(new Set([song.url]), files).size).toBe(2);
  });

  it("measures progress by phase", () => {
    expect(grabFraction(progress({ pages_done: 1, pages_total: 4 }))).toBe(0.25);
    expect(grabFraction(progress({ phase: "sizes", sized: 3, to_size: 6 }))).toBe(0.5);
    expect(grabFraction(progress({ phase: "done" }))).toBe(1);
    expect(grabFraction(progress({}))).toBe(0);
  });
});
