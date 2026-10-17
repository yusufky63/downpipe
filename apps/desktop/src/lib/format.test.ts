import { describe, expect, it } from "vitest";

import {
  bytesToRate,
  formatBytes,
  formatDuration,
  formatSpeed,
  hostOf,
  nameFromUrl,
  parseUrls,
  percent,
  rateToBytes,
  sentence,
} from "./format";

describe("formatBytes", () => {
  it("uses binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1536)).toBe("1.5 KiB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5.0 GiB");
    expect(formatBytes(-1)).toBe("–");
    expect(formatBytes(1024 * 1024 - 1)).toBe("1.0 MiB");
  });

  it("formats speeds and hides zero", () => {
    expect(formatSpeed(2 * 1024 * 1024)).toBe("2.0 MiB/s");
    expect(formatSpeed(0)).toBe("–");
  });
});

describe("sentence", () => {
  it("capitalizes the first letter only", () => {
    expect(sentence("the file was not found (HTTP 404)")).toBe("The file was not found (HTTP 404)");
    expect(sentence("")).toBe("");
  });
});

describe("formatDuration", () => {
  it("scales from seconds to hours", () => {
    expect(formatDuration(42)).toBe("42s");
    expect(formatDuration(187)).toBe("3:07");
    expect(formatDuration(3723)).toBe("1:02:03");
    expect(formatDuration(null)).toBe("–");
  });
});

describe("percent", () => {
  it("handles unknown and empty sizes", () => {
    expect(percent(5, null)).toBeNull();
    expect(percent(0, 0)).toBe(100);
    expect(percent(25, 100)).toBe(25);
    expect(percent(200, 100)).toBe(100);
  });
});

describe("urls", () => {
  it("extracts http links from pasted text", () => {
    const text = `Grab <https://example.org/a.iso> and
      https://example.org/b.zip, plus ftp://nope and https://example.org/a.iso again`;
    expect(parseUrls(text)).toEqual(["https://example.org/a.iso", "https://example.org/b.zip"]);
    expect(parseUrls("no links here")).toEqual([]);
  });

  it("derives display names", () => {
    expect(nameFromUrl("https://example.org/pub/ubuntu%2024.04.iso?x=1")).toBe("ubuntu 24.04.iso");
    expect(nameFromUrl("https://example.org/")).toBe("example.org");
    expect(hostOf("https://cdn.example.org:8443/x")).toBe("cdn.example.org:8443");
    expect(hostOf("not a url")).toBe("not a url");
  });
});

describe("rates", () => {
  it("round-trips the speed limit field", () => {
    expect(rateToBytes(2, "MiB")).toBe(2 * 1024 * 1024);
    expect(rateToBytes(512, "KiB")).toBe(512 * 1024);
    expect(rateToBytes(0, "MiB")).toBe(0);
    expect(rateToBytes(Number.NaN, "MiB")).toBe(0);
    expect(bytesToRate(2 * 1024 * 1024)).toEqual({ value: 2, unit: "MiB" });
    expect(bytesToRate(512 * 1024)).toEqual({ value: 512, unit: "KiB" });
    expect(bytesToRate(0)).toEqual({ value: 0, unit: "MiB" });
  });
});
