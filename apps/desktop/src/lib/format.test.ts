import { afterEach, describe, expect, it } from "vitest";

import { CATALOGS } from "../i18n/catalogs";
import { activate, type Locale } from "../i18n/core";

import {
  bytesToRate,
  decimalField,
  formatBytes,
  formatDate,
  formatDecimal,
  formatDuration,
  formatPercent,
  formatSpeed,
  hostOf,
  nameFromUrl,
  parseDecimal,
  parseUrls,
  percent,
  rateToBytes,
  sentence,
  withStop,
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

describe("formatDate", () => {
  it("formats unix seconds and rejects missing values", () => {
    // Local time zone varies between machines, so check the shape.
    expect(formatDate(1_792_181_400)).toMatch(/^\d{1,2} [A-Z][a-z]{2} 2026, \d{2}:\d{2}$/);
    expect(formatDate(null)).toBe("–");
    expect(formatDate(0)).toBe("–");
  });
});

describe("sentence", () => {
  it("capitalizes the first letter only", () => {
    expect(sentence("the file was not found (HTTP 404)")).toBe("The file was not found (HTTP 404)");
    expect(sentence("")).toBe("");
  });

  it("ends a message that more text follows", () => {
    expect(withStop("The file was not found on the server (HTTP 404)")).toBe("The file was not found on the server (HTTP 404).");
    expect(withStop("Yt-dlp is not set up.")).toBe("Yt-dlp is not set up.");
    expect(withStop("")).toBe("");
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
    const magnet = "magnet:?xt=urn:btih:c9e15763f722f23e98a29decdfae341b98d53056&dn=a";
    expect(parseUrls(`${magnet} ${magnet} magnet:?dn=no-hash https://example.org/a.iso`)).toEqual([magnet, "https://example.org/a.iso"]);
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

// Intl writes French and German percents with a no-break space (U+202F,
// U+00A0); compare with plain spaces.
const plain = (text: string) => text.replace(/[\u00a0\u202f]/g, " ");

describe("numbers in the user's language", () => {
  const use = (locale: Locale) => activate(locale, CATALOGS[locale]);
  afterEach(() => use("en"));

  it("keeps English as it was", () => {
    expect(formatPercent(50)).toBe("50%");
    expect(formatPercent(45.25, 1)).toBe("45.3%");
    expect(formatDecimal(2.5, 1)).toBe("2.5");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("writes Turkish decimals and percents", () => {
    use("tr");
    expect(formatBytes(1536)).toBe("1,5 KiB");
    expect(formatSpeed(2 * 1024 * 1024)).toBe("2,0 MiB/s");
    expect(formatPercent(50)).toBe("%50");
    expect(formatPercent(45.25, 1)).toBe("%45,3");
  });

  it("writes German and French decimals and percents", () => {
    use("de");
    expect(formatBytes(5.25 * 1024 ** 3)).toBe("5,3 GiB");
    expect(plain(formatPercent(50))).toBe("50 %");
    expect(formatBytes(1023)).toBe("1023 B");
    use("fr");
    expect(formatBytes(1536)).toBe("1,5 KiB");
    expect(plain(formatPercent(7))).toBe("7 %");
  });
});

describe("decimal fields", () => {
  afterEach(() => activate("en", CATALOGS.en));

  it("accepts a dot or a comma", () => {
    expect(parseDecimal("1.5")).toBe(1.5);
    expect(parseDecimal(" 1,5 ")).toBe(1.5);
    expect(parseDecimal("12")).toBe(12);
    expect(parseDecimal("1,5,0")).toBeNaN();
    expect(parseDecimal("1.000,5")).toBeNaN();
    expect(parseDecimal("-1")).toBeNaN();
    expect(parseDecimal("")).toBeNaN();
  });

  it("shows a stored value with the language's decimal sign", () => {
    expect(decimalField(1.5)).toBe("1.5");
    activate("tr", CATALOGS.tr);
    expect(decimalField(1.5)).toBe("1,5");
    expect(decimalField(512)).toBe("512");
  });
});
