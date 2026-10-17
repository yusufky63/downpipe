import { describe, expect, it } from "vitest";
import { decideCapture, linkFields, type DownloadItemLike } from "./capture";
import { defaultFilters, type CaptureFilters } from "./filters";

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

describe("decideCapture with filters", () => {
  const filters = (extra: Partial<CaptureFilters> = {}): CaptureFilters => ({ ...defaultFilters(), ...extra });
  const reason = (extra: Partial<DownloadItemLike>, f: Partial<CaptureFilters> = {}) => {
    const d = decideCapture(item(extra), SELF, filters(f));
    return d.capture ? "captured" : d.reason;
  };

  it("captures everything when no filters are passed", () => {
    expect(decideCapture(item({ filename: "page.html", url: "https://example.org/page.html" }), SELF).capture).toBe(true);
  });

  it("skips types that are not listed, judged by the browser's file name", () => {
    expect(reason({ filename: "C:\\Downloads\\photo.JPG" })).toBe("unlisted_type");
    expect(reason({ filename: "C:\\Downloads\\Setup.EXE" })).toBe("captured");
    expect(reason({ filename: "/home/me/Downloads/disk.iso" })).toBe("captured");
    // The server named it: the URL's own name does not matter then.
    expect(reason({ url: "https://example.org/get.php?id=4", filename: "report.pdf" })).toBe("captured");
    expect(reason({ url: "https://example.org/report.pdf", filename: "report.html" })).toBe("unlisted_type");
  });

  it("falls back to the URL path without its query string", () => {
    expect(reason({ filename: "", url: "https://example.org/a.zip?sig=abc.html" })).toBe("captured");
    expect(reason({ filename: undefined, url: "https://example.org/get.php?file=a.zip" })).toBe("unlisted_type");
    expect(reason({ filename: undefined, url: "https://example.org/download" })).toBe("unlisted_type");
  });

  it("captures every type when asked to, or when the list is empty", () => {
    expect(reason({ filename: "photo.jpg" }, { captureAll: true })).toBe("captured");
    expect(reason({ filename: "photo.jpg" }, { captureTypes: [] })).toBe("captured");
    expect(reason({ filename: "photo.jpg" }, { captureTypes: ["jpg"] })).toBe("captured");
  });

  it("skips excluded sites by download host, redirect target or page", () => {
    const excludedSites = ["*.example.org", "files.test"];
    expect(reason({ url: "https://cdn.example.org:8443/a.zip", referrer: "" }, { excludedSites })).toBe("excluded_site");
    expect(reason({ url: "https://other.net/a.zip", referrer: "https://www.Example.org/" }, { excludedSites })).toBe(
      "excluded_site",
    );
    expect(
      reason({ url: "https://other.net/a.zip", finalUrl: "http://files.test/x.zip", referrer: "" }, { excludedSites }),
    ).toBe("excluded_site");
    expect(reason({ url: "https://other.net/a.zip", referrer: "https://notexample.org/" }, { excludedSites })).toBe(
      "captured",
    );
  });

  it("matches international hosts in either spelling", () => {
    expect(reason({ url: "https://bücher.de/a.zip", referrer: "" }, { excludedSites: ["xn--bcher-kva.de"] })).toBe(
      "excluded_site",
    );
    expect(
      reason({ url: "https://xn--bcher-kva.de/a.zip", referrer: "" }, { excludedSites: ["*.xn--bcher-kva.de"] }),
    ).toBe("excluded_site");
  });

  it("skips files under the minimum size only when the size is known", () => {
    const minSizeBytes = 1024 * 1024;
    expect(reason({ totalBytes: 1000 }, { minSizeBytes })).toBe("too_small");
    expect(reason({ totalBytes: -1, fileSize: 1000 }, { minSizeBytes })).toBe("too_small");
    expect(reason({ totalBytes: minSizeBytes }, { minSizeBytes })).toBe("captured");
    expect(reason({ totalBytes: -1, fileSize: -1 }, { minSizeBytes })).toBe("captured");
    expect(reason({ totalBytes: 0 }, { minSizeBytes })).toBe("captured");
    expect(reason({}, { minSizeBytes })).toBe("captured");
    expect(reason({ totalBytes: 10 }, { minSizeBytes: 0 })).toBe("captured");
  });

  it("checks the browser-only reasons before the user's filters", () => {
    expect(reason({ incognito: true, filename: "photo.jpg" })).toBe("private_window");
    expect(reason({ url: "blob:https://example.org/1" }, { excludedSites: ["example.org"] })).toBe("unsupported_url");
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
