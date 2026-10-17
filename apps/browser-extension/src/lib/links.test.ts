import { describe, expect, it } from "vitest";
import {
  COLLECTION_TTL_MS,
  MAX_LINK_TEXT,
  MAX_PAGE_LINKS,
  OTHER,
  cleanLinks,
  collectionFromPage,
  collectionKey,
  downloadLabel,
  extensionChips,
  filterLinks,
  linkExtension,
  linkView,
  parseCollection,
  staleCollectionKeys,
  summarize,
  unreadableCollection,
  withoutPages,
  type LinkCollection,
} from "./links";
import { MAX_URL_LENGTH } from "./url";

const ID = "4b0d1c9e-8f7a-4c3b-9d2e-1a2b3c4d5e6f";
const PAGE = "https://example.org/files/";

describe("collection keys", () => {
  it("accepts only the ids the service worker makes", () => {
    expect(collectionKey(ID)).toBe(`links:${ID}`);
    for (const bad of ["", "x", `${ID}x`, ID.toUpperCase(), "../activity", "settings"]) {
      expect(collectionKey(bad), bad).toBeNull();
    }
  });

  it("finds collections nobody opened in time", () => {
    const now = 1_000_000_000;
    const stored = {
      activity: [],
      [`links:${ID}`]: { at: now - 1000 },
      "links:old": { at: now - COLLECTION_TTL_MS - 1 },
      "links:broken": "x",
      "links:future": { at: now + 120_000 },
    };
    expect(staleCollectionKeys(stored, now).sort()).toEqual(["links:broken", "links:future", "links:old"]);
  });
});

describe("cleanLinks", () => {
  it("keeps valid http(s) links once each, without fragments", () => {
    const { links, dropped } = cleanLinks(
      [
        { url: "https://example.org/a.zip#x", text: "" },
        { url: "https://example.org/a.zip", text: "Archive" },
        { url: "javascript:alert(1)", text: "Bad" },
        { url: "data:text/plain,x", text: "Bad" },
        { url: 42, text: "Bad" },
        null,
        "https://example.org/string.zip",
        { url: PAGE, text: "The page itself" },
        { url: `https://example.org/${"a".repeat(MAX_URL_LENGTH)}`, text: "Too long" },
        { url: "https://example.org/b.pdf", text: 7 },
      ],
      PAGE,
    );
    expect(links).toEqual([
      { url: "https://example.org/a.zip", text: "Archive" },
      { url: "https://example.org/b.pdf", text: "" },
    ]);
    expect(dropped).toBe(false);
  });

  it("strips control characters and caps texts", () => {
    const { links } = cleanLinks([{ url: "https://x.org/a", text: `a\u0000b\nc\u009f${"d".repeat(400)}` }], PAGE);
    expect(links[0]?.text.startsWith("a b c d")).toBe(true);
    expect(links[0]?.text.length).toBe(MAX_LINK_TEXT);
  });

  it("caps the number of links", () => {
    const raw = Array.from({ length: 5 }, (_, i) => ({ url: `https://x.org/${i}.zip`, text: "" }));
    expect(cleanLinks(raw, PAGE, 3)).toMatchObject({ dropped: true });
    expect(cleanLinks(raw, PAGE, 3).links).toHaveLength(3);
    expect(cleanLinks("nope", PAGE)).toEqual({ links: [], dropped: false });
  });
});

describe("collectionFromPage", () => {
  it("checks what the page returned", () => {
    const now = 1234;
    expect(
      collectionFromPage(
        {
          pageUrl: "https://user:pw@example.org/files/#top",
          title: " Files\n\tfor you ",
          links: [{ url: "https://example.org/a.zip", text: "A" }],
          truncated: false,
        },
        now,
      ),
    ).toEqual({
      pageUrl: "https://example.org/files/",
      title: "Files for you",
      links: [{ url: "https://example.org/a.zip", text: "A" }],
      truncated: false,
      unreadable: false,
      at: now,
    });
  });

  it("refuses other shapes and odd page addresses", () => {
    expect(collectionFromPage(undefined, 1)).toBeNull();
    expect(collectionFromPage({ links: "x" }, 1)).toBeNull();
    expect(collectionFromPage({ pageUrl: "file:///C:/x.html", links: [] }, 1)).toMatchObject({ pageUrl: "", title: "" });
  });

  it("marks a capped list as truncated", () => {
    const links = Array.from({ length: MAX_PAGE_LINKS + 1 }, (_, i) => ({ url: `https://x.org/${i}`, text: "" }));
    const result = collectionFromPage({ pageUrl: PAGE, links, truncated: false }, 1);
    expect(result?.links).toHaveLength(MAX_PAGE_LINKS);
    expect(result?.truncated).toBe(true);
  });
});

describe("parseCollection", () => {
  const stored: LinkCollection = {
    pageUrl: PAGE,
    title: "Files",
    links: [{ url: "https://example.org/a.zip", text: "A" }],
    truncated: true,
    unreadable: false,
    at: 5,
  };

  it("reads back what the service worker stored", () => {
    expect(parseCollection(JSON.parse(JSON.stringify(stored)))).toEqual(stored);
    expect(parseCollection(unreadableCollection("chrome://settings", 9))).toEqual({
      pageUrl: "",
      title: "",
      links: [],
      truncated: false,
      unreadable: true,
      at: 9,
    });
  });

  it("refuses anything else", () => {
    expect(parseCollection(null)).toBeNull();
    expect(parseCollection({ ...stored, at: "now" })).toBeNull();
    expect(parseCollection({ ...stored, truncated: "yes" })).toBeNull();
    expect(parseCollection({ ...stored, unreadable: undefined })).toBeNull();
    expect(parseCollection({ ...stored, links: {} })).toBeNull();
  });

  it("checks every link again", () => {
    const tampered = { ...stored, links: [...stored.links, { url: "javascript:alert(1)", text: "x" }] };
    expect(parseCollection(tampered)?.links).toEqual(stored.links);
  });
});

describe("linkExtension", () => {
  it("reads the last path segment, not the query", () => {
    expect(linkExtension("https://x.org/a/Setup.EXE")).toBe("exe");
    expect(linkExtension("https://x.org/a.tar.gz")).toBe("gz");
    expect(linkExtension("https://x.org/get.php?file=a.zip")).toBe("php");
    expect(linkExtension("https://x.org/My%20Song.mp3")).toBe("mp3");
    expect(linkExtension("https://x.org/archive.zip.001")).toBe("001");
  });

  it("finds none for folders, hosts, dotfiles and version numbers", () => {
    for (const url of [
      "https://x.org",
      "https://x.org/",
      "https://x.org/files/",
      "https://x.org/download",
      "https://x.org/.htaccess",
      "https://x.org/name.",
      "https://x.org/release/v1.2",
      "https://x.org/release/1.20",
      "https://x.org/a.verylongextension",
      "https://x.org/a.b-c",
      "mailto:x@y.z",
    ]) {
      expect(linkExtension(url), url).toBeNull();
    }
  });

  it("tells web pages from files", () => {
    expect(linkView({ url: "https://x.org/a.zip", text: "" })).toMatchObject({ ext: "zip", page: false, name: "a.zip" });
    expect(linkView({ url: "https://x.org/about.html", text: "About" })).toMatchObject({ ext: "html", page: true });
    expect(linkView({ url: "https://x.org/view.aspx?id=1", text: "" })).toMatchObject({ ext: "aspx", page: true });
    expect(linkView({ url: "https://x.org/blog/post", text: "" })).toMatchObject({ ext: null, page: true, name: "post" });
    expect(linkView({ url: "https://x.org/", text: "" })).toMatchObject({ ext: null, page: true, name: "x.org" });
  });
});

describe("grouping and filtering", () => {
  const views = [
    "https://x.org/a.zip",
    "https://x.org/b.zip",
    "https://x.org/c.zip",
    "https://x.org/d.pdf",
    "https://x.org/e.pdf",
    "https://x.org/f.iso",
    "https://x.org/index.html",
    "https://x.org/about",
  ].map((url, i) => linkView({ url, text: i === 3 ? "User Manual" : "" }));

  it("makes one chip per type, most frequent first, plus Other", () => {
    expect(extensionChips(views)).toEqual([
      { key: "zip", label: "zip", count: 3 },
      { key: "pdf", label: "pdf", count: 2 },
      { key: "html", label: "html", count: 1 },
      { key: "iso", label: "iso", count: 1 },
      { key: OTHER, label: "Other", count: 1 },
    ]);
    expect(extensionChips(views, 2)).toEqual([
      { key: "zip", label: "zip", count: 3 },
      { key: "pdf", label: "pdf", count: 2 },
      { key: OTHER, label: "Other", count: 3 },
    ]);
    expect(extensionChips([])).toEqual([]);
  });

  it("hides web pages when asked", () => {
    expect(withoutPages(views, true).map((v) => v.name)).toEqual(["a.zip", "b.zip", "c.zip", "d.pdf", "e.pdf", "f.iso"]);
    expect(withoutPages(views, false)).toHaveLength(views.length);
  });

  it("filters by type chips, Other included", () => {
    const names = (types: string[], hidePages = true) =>
      filterLinks(views, { query: "", hidePages, types: new Set(types) }).map((v) => v.name);
    expect(names([])).toHaveLength(6);
    expect(names(["pdf", "iso"])).toEqual(["d.pdf", "e.pdf", "f.iso"]);
    expect(names([OTHER], false)).toEqual(["about"]);
    expect(names([OTHER])).toEqual([]);
  });

  it("filters by every word of the query, in text, name or address", () => {
    const names = (query: string) => filterLinks(views, { query, hidePages: false, types: new Set() }).map((v) => v.name);
    expect(names("manual")).toEqual(["d.pdf"]);
    expect(names("  USER   pdf ")).toEqual(["d.pdf"]);
    expect(names("zip")).toEqual(["a.zip", "b.zip", "c.zip"]);
    expect(names("x.org about")).toEqual(["about"]);
    expect(names("nothing-like-this")).toEqual([]);
  });
});

describe("results", () => {
  it("sums up what happened", () => {
    expect(summarize(["added", "added", "duplicate", "failed"])).toBe("2 added, 1 already in Downpipe, 1 failed.");
    expect(summarize(["duplicate"])).toBe("1 already in Downpipe.");
    expect(summarize([])).toBe("Nothing sent.");
  });

  it("labels the button", () => {
    expect(downloadLabel(0)).toBe("Download 0 links");
    expect(downloadLabel(1)).toBe("Download 1 link");
    expect(downloadLabel(1200)).toBe("Download 1,200 links");
  });
});
