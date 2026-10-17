import { describe, expect, it } from "vitest";

import { parseTheme } from "./theme";

describe("parseTheme", () => {
  it("keeps a stored choice", () => {
    expect(parseTheme("light")).toBe("light");
    expect(parseTheme("dark")).toBe("dark");
  });

  it("follows the system for anything else", () => {
    expect(parseTheme(null)).toBeNull();
    expect(parseTheme("")).toBeNull();
    expect(parseTheme("Dark")).toBeNull();
  });
});
