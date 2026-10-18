import { describe, expect, it } from "vitest";

import { bypassUrl, isBypassClick, parseBypassKey, parseBypassRequest, type ClickLike } from "./bypass";

const click = (patch: Partial<ClickLike> = {}): ClickLike => ({
  button: 0,
  altKey: false,
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  isTrusted: true,
  ...patch,
});

describe("isBypassClick", () => {
  it("takes the chosen key alone on a real primary click", () => {
    expect(isBypassClick(click({ altKey: true }), "alt")).toBe(true);
    expect(isBypassClick(click({ shiftKey: true }), "shift")).toBe(true);
    expect(isBypassClick(click({ altKey: true }), "shift")).toBe(false);
    expect(isBypassClick(click({ shiftKey: true }), "alt")).toBe(false);
    expect(isBypassClick(click(), "alt")).toBe(false);
  });

  it("leaves the browser's own shortcuts and made-up clicks alone", () => {
    expect(isBypassClick(click({ altKey: true, ctrlKey: true }), "alt")).toBe(false);
    expect(isBypassClick(click({ altKey: true, metaKey: true }), "alt")).toBe(false);
    expect(isBypassClick(click({ altKey: true, shiftKey: true }), "alt")).toBe(false);
    expect(isBypassClick(click({ altKey: true, button: 1 }), "alt")).toBe(false);
    expect(isBypassClick(click({ altKey: true, isTrusted: false }), "alt")).toBe(false);
  });

  it("does nothing when switched off", () => {
    expect(isBypassClick(click({ altKey: true }), "off")).toBe(false);
    expect(isBypassClick(click({ shiftKey: true }), "off")).toBe(false);
  });
});

describe("bypassUrl", () => {
  it("passes only links capture would take", () => {
    expect(bypassUrl("https://a.test/f.zip")).toBe("https://a.test/f.zip");
    expect(bypassUrl("javascript:alert(1)")).toBeNull();
    expect(bypassUrl("blob:https://a.test/x")).toBeNull();
    expect(bypassUrl(undefined)).toBeNull();
  });
});

describe("parseBypassKey", () => {
  it("defaults to Alt for missing or unknown values", () => {
    expect(parseBypassKey(undefined)).toBe("alt");
    expect(parseBypassKey("ctrl")).toBe("alt");
    expect(parseBypassKey("shift")).toBe("shift");
    expect(parseBypassKey("off")).toBe("off");
  });
});

describe("parseBypassRequest", () => {
  it("accepts the exact shape with a web link only", () => {
    expect(parseBypassRequest({ kind: "bypass_download", url: "https://a.test/f" })).toEqual({
      kind: "bypass_download",
      url: "https://a.test/f",
    });
    expect(parseBypassRequest({ kind: "bypass_download", url: "file:///C:/x" })).toBeNull();
    expect(parseBypassRequest({ kind: "bypass_download", url: "https://a.test/", tabId: 1 })).toBeNull();
    expect(parseBypassRequest({ kind: "overlay_page" })).toBeNull();
    expect(parseBypassRequest(null)).toBeNull();
  });
});
