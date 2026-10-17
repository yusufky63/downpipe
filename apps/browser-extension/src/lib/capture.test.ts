import { describe, expect, it } from "vitest";
import { decideCapture, linkFields, type DownloadItemLike } from "./capture";

const SELF = "efgicfboaldflckpncfdojkeoaamflpf";

function item(extra: Partial<DownloadItemLike> = {}): DownloadItemLike {
  return {
    id: 7,
    url: "https://example.org/file.zip",
    referrer: "https://example.org/downloads",
    mime: "application/zip",
    filename: "C:\\Users\\me\\Downloads\\file.zip",
    incognito: false,
    state: "in_progress",
    ...extra,
  };
}

describe("decideCapture", () => {
  it("captures ordinary web downloads with the browser's hints", () => {
    expect(decideCapture(item(), SELF)).toEqual({
      capture: true,
      fields: {
        resourceUrl: "https://example.org/file.zip",
        referrer: "https://example.org/downloads",
        mimeType: "application/zip",
        fileName: "C:\\Users\\me\\Downloads\\file.zip",
      },
    });
  });

  it("leaves empty hints out", () => {
    const d = decideCapture(item({ referrer: "", mime: "", filename: "" }), SELF);
    expect(d).toEqual({ capture: true, fields: { resourceUrl: "https://example.org/file.zip" } });
  });

  it("skips its own fallback downloads", () => {
    expect(decideCapture(item({ byExtensionId: SELF }), SELF)).toEqual({ capture: false, reason: "own_download" });
    expect(decideCapture(item({ byExtensionId: "other" }), SELF).capture).toBe(true);
  });

  it("skips private windows", () => {
    expect(decideCapture(item({ incognito: true }), SELF)).toEqual({ capture: false, reason: "private_window" });
  });

  it("skips downloads that are not running", () => {
    for (const state of ["complete", "interrupted"]) {
      expect(decideCapture(item({ state }), SELF)).toEqual({ capture: false, reason: "not_in_progress" });
    }
  });

  it("skips URLs that only exist inside the browser", () => {
    for (const url of ["blob:https://example.org/9f", "data:application/pdf;base64,AA", "file:///C:/x.zip"]) {
      expect(decideCapture(item({ url }), SELF)).toEqual({ capture: false, reason: "unsupported_url" });
    }
  });
});

describe("linkFields", () => {
  it("prefers the frame the link was in", () => {
    expect(
      linkFields({
        linkUrl: "https://files.example.org/a.zip",
        pageUrl: "https://example.org/",
        frameUrl: "https://files.example.org/list",
      }),
    ).toEqual({
      resourceUrl: "https://files.example.org/a.zip",
      pageUrl: "https://files.example.org/list",
      referrer: "https://files.example.org/list",
    });
  });

  it("refuses non-web links", () => {
    expect(linkFields({ linkUrl: "javascript:void(0)", pageUrl: "https://example.org/" })).toBeNull();
    expect(linkFields({ pageUrl: "https://example.org/" })).toBeNull();
  });
});
