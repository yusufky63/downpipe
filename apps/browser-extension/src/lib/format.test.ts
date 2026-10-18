import { afterEach, describe, expect, it } from "vitest";
import { CATALOGS } from "../i18n/catalogs";
import { activate, type Locale } from "../i18n/core";
import { formatBytes, formatDecimal, formatPercent, formatSpeed, percent, sentence } from "./format";

describe("format", () => {
  it("prints binary sizes with one decimal", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 KiB");
    expect(formatBytes(1048575)).toBe("1.0 MiB");
    expect(formatBytes(16 * 1048576)).toBe("16.0 MiB");
    expect(formatBytes(-1)).toBe("–");
    expect(formatBytes(Number.NaN)).toBe("–");
  });

  it("prints speeds and percentages", () => {
    expect(formatSpeed(2 * 1048576)).toBe("2.0 MiB/s");
    expect(formatSpeed(0)).toBe("–");
    expect(percent(5, 10)).toBe(50);
    expect(percent(5, undefined)).toBeNull();
    expect(percent(0, 0)).toBe(100);
    expect(percent(20, 10)).toBe(100);
  });

  it("capitalizes engine messages", () => {
    expect(sentence("could not connect")).toBe("Could not connect");
    expect(sentence("")).toBe("");
  });
});

// Intl writes French and German percents with a no-break space; compare
// with plain spaces.
const plain = (text: string) => text.replace(/[\u00a0\u202f]/g, " ");

describe("numbers in the browser's language", () => {
  const use = (locale: Locale) => activate(locale, CATALOGS[locale]);
  afterEach(() => use("en"));

  it("keeps English as it was", () => {
    expect(formatPercent(50)).toBe("50%");
    expect(formatDecimal(2.25, 1)).toBe("2.3");
  });

  it("writes Turkish, German and French numbers", () => {
    use("tr");
    expect(formatBytes(1.5 * 1048576)).toBe("1,5 MiB");
    expect(formatSpeed(1536)).toBe("1,5 KiB/s");
    expect(formatPercent(50)).toBe("%50");
    use("de");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(plain(formatPercent(50))).toBe("50 %");
    use("fr");
    expect(formatBytes(3 * 1048576)).toBe("3,0 MiB");
    expect(plain(formatPercent(12))).toBe("12 %");
  });
});
