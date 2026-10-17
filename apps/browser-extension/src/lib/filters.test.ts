import { describe, expect, it } from "vitest";
import {
  DEFAULT_CAPTURE_TYPES,
  MAX_CAPTURE_TYPES,
  MAX_EXCLUDED_SITES,
  MAX_MIN_SIZE_BYTES,
  defaultFilters,
  downloadName,
  filtersFromForm,
  hostMatches,
  hostOf,
  knownSize,
  matchingSite,
  normalizeSitePattern,
  normalizeType,
  parseFilters,
  parseMinSize,
  parseSiteList,
  parseTypeList,
  sizeFields,
  typeListed,
  validFilters,
  withSite,
} from "./filters";

describe("file types", () => {
  it("normalizes what people type", () => {
    expect(normalizeType("zip")).toBe("zip");
    expect(normalizeType(" .ZIP ")).toBe("zip");
    expect(normalizeType("*.Mp4")).toBe("mp4");
    expect(normalizeType("tar.gz")).toBe("tar.gz");
    expect(normalizeType("7z")).toBe("7z");
  });

  it("refuses things that are not extensions", () => {
    for (const bad of ["", ".", "*.", "*", "zip!", "a b", "tar..gz", ".tar.", "c++", "../exe", "pdf/x", "ü", "x".repeat(17)]) {
      expect(normalizeType(bad), bad).toBeNull();
    }
  });

  it("parses a list split by spaces, commas, semicolons and new lines", () => {
    expect(parseTypeList("zip, PDF;mp4\n.iso  *.7z zip")).toEqual({ values: ["zip", "pdf", "mp4", "iso", "7z"], errors: [] });
    expect(parseTypeList("   ")).toEqual({ values: [], errors: [] });
  });

  it("names each bad entry", () => {
    const { values, errors } = parseTypeList("zip what? pdf");
    expect(values).toEqual(["zip", "pdf"]);
    expect(errors).toEqual(['"what?" is not a file extension.']);
  });

  it("caps the list", () => {
    const many = Array.from({ length: MAX_CAPTURE_TYPES + 1 }, (_, i) => `t${i}`).join(" ");
    expect(parseTypeList(many).errors).toEqual([`List at most ${MAX_CAPTURE_TYPES} file types.`]);
  });

  it("matches the end of the name, ignoring case", () => {
    expect(typeListed("Setup.EXE", ["exe"])).toBe(true);
    expect(typeListed("backup.tar.gz", ["gz"])).toBe(true);
    expect(typeListed("backup.tar.gz", ["tar.gz"])).toBe(true);
    expect(typeListed("backup.tgz", ["tar.gz"])).toBe(false);
    expect(typeListed("notes.zipx", ["zip"])).toBe(false);
    expect(typeListed("zip", ["zip"])).toBe(false);
    expect(typeListed(".zip", ["zip"])).toBe(false);
    expect(typeListed("", ["zip"])).toBe(false);
  });

  it("ships a default list in stored form, without web pages or scripts", () => {
    for (const t of DEFAULT_CAPTURE_TYPES) expect(normalizeType(t)).toBe(t);
    expect(new Set(DEFAULT_CAPTURE_TYPES).size).toBe(DEFAULT_CAPTURE_TYPES.length);
    for (const t of ["zip", "exe", "msi", "iso", "pdf", "mp3", "mp4", "mkv", "7z", "rar", "dmg"]) {
      expect(DEFAULT_CAPTURE_TYPES).toContain(t);
    }
    for (const t of ["html", "htm", "php", "js", "css", "png", "jpg", "json"]) {
      expect(DEFAULT_CAPTURE_TYPES).not.toContain(t);
    }
  });
});

describe("downloadName", () => {
  it("prefers the browser's file name, from a full path on any platform", () => {
    expect(downloadName("C:\\Users\\me\\Downloads\\report.pdf", "https://x.org/get?id=1")).toBe("report.pdf");
    expect(downloadName("/home/me/Downloads/a.iso", "https://x.org/b.zip")).toBe("a.iso");
    expect(downloadName("setup.exe", "https://x.org/download")).toBe("setup.exe");
  });

  it("falls back to the URL path and ignores the query string", () => {
    expect(downloadName(undefined, "https://x.org/files/a.zip?token=abc#x")).toBe("a.zip");
    expect(downloadName("", "https://x.org/get.php?file=a.zip")).toBe("get.php");
    expect(downloadName("  ", "https://x.org/My%20File.pdf")).toBe("My File.pdf");
    expect(downloadName(undefined, "https://x.org/bad%E0%A4%A.zip")).toBe("bad%E0%A4%A.zip");
    expect(downloadName(undefined, "https://x.org/")).toBe("");
    expect(downloadName(undefined, "blob:https://x.org/1")).toBe("");
  });
});

describe("site patterns", () => {
  it("stores hosts in lower case without a trailing dot", () => {
    expect(normalizeSitePattern("Example.COM")).toBe("example.com");
    expect(normalizeSitePattern("example.com.")).toBe("example.com");
    expect(normalizeSitePattern("*.Example.com")).toBe("*.example.com");
    expect(normalizeSitePattern("localhost")).toBe("localhost");
    expect(normalizeSitePattern("127.0.0.1")).toBe("127.0.0.1");
    expect(normalizeSitePattern("[::1]")).toBe("[::1]");
  });

  it("takes the host from pasted URLs, ports and paths", () => {
    expect(normalizeSitePattern("https://www.example.com/downloads?x=1#top")).toBe("www.example.com");
    expect(normalizeSitePattern("http://user:pw@files.example.com:8080/")).toBe("files.example.com");
    expect(normalizeSitePattern("example.com:8443")).toBe("example.com");
    expect(normalizeSitePattern("example.com/some/path")).toBe("example.com");
    expect(normalizeSitePattern("*.example.com:8080/x")).toBe("*.example.com");
  });

  it("stores international names as punycode, as URLs carry them", () => {
    expect(normalizeSitePattern("bücher.de")).toBe("xn--bcher-kva.de");
    expect(normalizeSitePattern("*.BÜCHER.de")).toBe("*.xn--bcher-kva.de");
    expect(normalizeSitePattern("xn--bcher-kva.de")).toBe("xn--bcher-kva.de");
    expect(hostOf("https://bücher.de/a.zip")).toBe("xn--bcher-kva.de");
  });

  it("refuses what it cannot match safely", () => {
    for (const bad of [
      "",
      "   ",
      "*",
      "*.",
      "*example.com",
      "ex*ample.com",
      "*.*.example.com",
      "example .com",
      "evil.com@example.com",
      "ftp://example.com/",
      "https://*.example.com/",
      "javascript:alert(1)",
      "*.127.0.0.1",
      "*.[::1]",
      "a\\b.com",
      "x".repeat(301),
    ]) {
      expect(normalizeSitePattern(bad), bad).toBeNull();
    }
  });

  it("matches exact hosts and wildcards", () => {
    expect(hostMatches("example.com", "example.com")).toBe(true);
    expect(hostMatches("www.example.com", "example.com")).toBe(false);
    expect(hostMatches("example.com", "*.example.com")).toBe(true);
    expect(hostMatches("a.b.example.com", "*.example.com")).toBe(true);
    expect(hostMatches("badexample.com", "*.example.com")).toBe(false);
    expect(hostMatches("example.com.evil.net", "*.example.com")).toBe(false);
  });

  it("checks every host it is given and ignores ports and case", () => {
    const patterns = ["*.example.com", "files.test"];
    expect(matchingSite([hostOf("https://CDN.Example.com:8443/a.zip")], patterns)).toBe("*.example.com");
    expect(matchingSite([hostOf("https://other.net/a.zip"), hostOf("http://files.test/page")], patterns)).toBe("files.test");
    expect(matchingSite([hostOf("https://other.net/a.zip"), null, hostOf(undefined)], patterns)).toBeUndefined();
    expect(matchingSite([hostOf("https://example.com./a.zip")], patterns)).toBe("*.example.com");
  });

  it("parses one site per line and points at bad lines", () => {
    expect(parseSiteList("example.com\r\n\n  *.cdn.net  \nexample.com\n")).toEqual({
      values: ["example.com", "*.cdn.net"],
      errors: [],
    });
    const { values, errors } = parseSiteList("good.org\nnot a site\n*bad.org");
    expect(values).toEqual(["good.org"]);
    expect(errors).toHaveLength(2);
    expect(errors[0]).toMatch(/^Line 2: "not a site" is not a site\./);
    expect(errors[1]).toMatch(/^Line 3: /);
  });

  it("caps the site list", () => {
    const many = Array.from({ length: MAX_EXCLUDED_SITES + 1 }, (_, i) => `s${i}.example`).join("\n");
    expect(parseSiteList(many).errors).toEqual([`List at most ${MAX_EXCLUDED_SITES} sites.`]);
  });

  it("adds and removes one exact entry", () => {
    expect(withSite(["a.com", "*.b.com"], "c.com", true)).toEqual(["a.com", "*.b.com", "c.com"]);
    expect(withSite(["a.com", "c.com"], "c.com", true)).toEqual(["a.com", "c.com"]);
    expect(withSite(["a.com", "*.b.com"], "a.com", false)).toEqual(["*.b.com"]);
    expect(withSite(["*.b.com"], "x.b.com", false)).toEqual(["*.b.com"]);
  });
});

describe("minimum size", () => {
  it("reads whole and decimal numbers in either unit", () => {
    expect(parseMinSize("", "MiB")).toEqual({ ok: true, bytes: 0 });
    expect(parseMinSize("0", "KiB")).toEqual({ ok: true, bytes: 0 });
    expect(parseMinSize("512", "KiB")).toEqual({ ok: true, bytes: 512 * 1024 });
    expect(parseMinSize(" 1.5 ", "MiB")).toEqual({ ok: true, bytes: 1.5 * 1024 * 1024 });
  });

  it("refuses anything else", () => {
    for (const bad of ["-1", "1e3", "1,5", "abc", "1.", ".5", "Infinity"]) {
      expect(parseMinSize(bad, "MiB").ok, bad).toBe(false);
    }
    expect(parseMinSize("1048577", "MiB")).toEqual({ ok: false, error: "Enter a size under 1 TiB." });
  });

  it("shows stored sizes in the larger unit that fits", () => {
    expect(sizeFields(0)).toEqual({ value: "0", unit: "MiB" });
    expect(sizeFields(512 * 1024)).toEqual({ value: "512", unit: "KiB" });
    expect(sizeFields(1.5 * 1024 * 1024)).toEqual({ value: "1.5", unit: "MiB" });
    expect(sizeFields(1000)).toEqual({ value: "0.98", unit: "KiB" });
  });

  it("treats -1, 0 and missing sizes as unknown", () => {
    expect(knownSize(-1, -1)).toBeUndefined();
    expect(knownSize(0, undefined)).toBeUndefined();
    expect(knownSize(undefined, undefined)).toBeUndefined();
    expect(knownSize(Number.NaN, 5)).toBe(5);
    expect(knownSize(-1, 2048)).toBe(2048);
    expect(knownSize(4096, 2048)).toBe(4096);
  });
});

describe("stored filters", () => {
  it("gives settings saved before filters existed the defaults", () => {
    expect(parseFilters({ notify: false })).toEqual(defaultFilters());
    expect(defaultFilters().captureTypes).toEqual([...DEFAULT_CAPTURE_TYPES]);
  });

  it("keeps an empty type list, which means every type", () => {
    expect(parseFilters({ captureTypes: [] }).captureTypes).toEqual([]);
  });

  it("drops bad entries one by one and repairs bad values", () => {
    expect(
      parseFilters({
        captureAll: "yes",
        captureTypes: ["ZIP", "zip", 7, "bad type", ".pdf"],
        excludedSites: ["Example.com", "not a site", null, "*.cdn.net"],
        minSizeBytes: 1.5,
      }),
    ).toEqual({
      captureAll: false,
      captureTypes: ["zip", "pdf"],
      excludedSites: ["example.com", "*.cdn.net"],
      minSizeBytes: 0,
    });
    expect(parseFilters({ minSizeBytes: -5 }).minSizeBytes).toBe(0);
    expect(parseFilters({ minSizeBytes: MAX_MIN_SIZE_BYTES + 1 }).minSizeBytes).toBe(0);
    expect(parseFilters({ minSizeBytes: 2048 }).minSizeBytes).toBe(2048);
    expect(parseFilters({ excludedSites: "example.com" }).excludedSites).toEqual([]);
  });

  it("accepts only exact, normalized filters from a page", () => {
    const good = { captureAll: false, captureTypes: ["zip"], excludedSites: ["*.example.com"], minSizeBytes: 1024 };
    expect(validFilters(good)).toEqual(good);
    expect(validFilters({ ...good, extra: true })).toBeNull();
    expect(validFilters({ ...good, captureTypes: ["ZIP"] })).toBeNull();
    expect(validFilters({ ...good, captureTypes: ["zip", "zip"] })).toBeNull();
    expect(validFilters({ ...good, excludedSites: ["Example.com"] })).toBeNull();
    expect(validFilters({ ...good, excludedSites: "example.com" })).toBeNull();
    expect(validFilters({ ...good, minSizeBytes: 1.5 })).toBeNull();
    expect(validFilters({ ...good, minSizeBytes: -1 })).toBeNull();
    expect(validFilters({ ...good, captureAll: 1 })).toBeNull();
    expect(validFilters([good])).toBeNull();
    expect(validFilters(null)).toBeNull();
  });
});

describe("filtersFromForm", () => {
  const form = { captureAll: false, typesText: "zip pdf", sitesText: "example.com", sizeText: "1", unit: "MiB" as const };

  it("returns filters ready to save", () => {
    expect(filtersFromForm(form)).toEqual({
      ok: true,
      filters: { captureAll: false, captureTypes: ["zip", "pdf"], excludedSites: ["example.com"], minSizeBytes: 1048576 },
    });
  });

  it("puts each problem under its own field", () => {
    const result = filtersFromForm({ ...form, typesText: "zip ?", sitesText: "a b", sizeText: "x" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.errors.types).toHaveLength(1);
    expect(result.errors.sites).toHaveLength(1);
    expect(result.errors.size).toHaveLength(1);
  });

  it("asks for a type instead of quietly capturing everything", () => {
    const result = filtersFromForm({ ...form, typesText: " , " });
    expect(result).toEqual({
      ok: false,
      errors: { types: ["Add at least one file type, or turn on Capture all file types."], sites: [], size: [] },
    });
  });

  it("does not let the greyed-out list block saving while every type is captured", () => {
    const result = filtersFromForm({ ...form, captureAll: true, typesText: "zip ??? " });
    expect(result).toEqual({
      ok: true,
      filters: { captureAll: true, captureTypes: ["zip"], excludedSites: ["example.com"], minSizeBytes: 1048576 },
    });
    expect(filtersFromForm({ ...form, captureAll: true, typesText: "" }).ok).toBe(true);
  });

  it("produces only values the service worker accepts", () => {
    const result = filtersFromForm({ ...form, typesText: "ZIP *.Pdf zip", sitesText: "HTTPS://Bücher.de/x\n*.CDN.net" });
    expect(result.ok).toBe(true);
    if (result.ok) expect(validFilters(result.filters)).toEqual(result.filters);
  });
});
