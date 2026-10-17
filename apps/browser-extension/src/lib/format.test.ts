import { describe, expect, it } from "vitest";
import { formatBytes, formatSpeed, percent, sentence } from "./format";

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
