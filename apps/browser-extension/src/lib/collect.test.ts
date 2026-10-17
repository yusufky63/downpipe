import { afterEach, describe, expect, it, vi } from "vitest";
import { collectPageLinks } from "./collect";

/**
 * The function as `scripting.executeScript` ships it: rebuilt from its
 * source text, so a reference to anything outside it (an import, a module
 * constant) fails here as it would in the page.
 */
const injected = new Function(`return (${collectPageLinks.toString()})`)() as typeof collectPageLinks;

interface FakeLink {
  href: string | null;
  text?: string;
  title?: string;
  aria?: string;
  alt?: string;
  selected?: boolean;
}

function element(link: FakeLink) {
  const attrs: Record<string, string | null | undefined> = {
    href: link.href,
    title: link.title,
    "aria-label": link.aria,
  };
  return {
    link,
    innerText: link.text ?? "",
    textContent: link.text ?? "",
    getAttribute: (name: string) => attrs[name] ?? null,
    querySelector: (selector: string) =>
      selector === "img[alt]" && link.alt !== undefined ? { getAttribute: () => link.alt } : null,
  };
}

function page(links: FakeLink[], options: { url?: string; selection?: "none" | "collapsed" | "some" } = {}) {
  const url = options.url ?? "https://example.org/files/index.html#top";
  const elements = links.map(element);
  const selection = {
    isCollapsed: options.selection !== "some",
    rangeCount: options.selection === "some" ? 1 : 0,
    containsNode: (node: ReturnType<typeof element>) => node.link.selected === true,
  };
  vi.stubGlobal("location", { href: url });
  vi.stubGlobal("document", {
    baseURI: url,
    title: "Downloads  ",
    querySelectorAll: (selector: string) => {
      expect(selector).toBe("a[href], area[href]");
      return elements;
    },
    getSelection: () => (options.selection === "none" ? null : selection),
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("collectPageLinks", () => {
  it("runs from its own source text, without imports", () => {
    page([{ href: "a.zip", text: "A" }]);
    expect(injected(false, 10).links).toEqual([{ url: "https://example.org/files/a.zip", text: "A" }]);
  });

  it("keeps http(s) links, resolved against the page", () => {
    page([
      { href: "setup.exe", text: "Installer" },
      { href: "/docs/manual.pdf", text: "Manual" },
      { href: "https://cdn.example.net/v2/app.dmg?sig=1", text: "Mac" },
      { href: "//mirror.example.com/app.tar.gz", text: "Mirror" },
      { href: "http://old.example.org/a.zip", text: "Old" },
    ]);
    expect(injected(false, 100)).toEqual({
      pageUrl: "https://example.org/files/index.html#top",
      title: "Downloads  ",
      truncated: false,
      links: [
        { url: "https://example.org/files/setup.exe", text: "Installer" },
        { url: "https://example.org/docs/manual.pdf", text: "Manual" },
        { url: "https://cdn.example.net/v2/app.dmg?sig=1", text: "Mac" },
        { url: "https://mirror.example.com/app.tar.gz", text: "Mirror" },
        { url: "http://old.example.org/a.zip", text: "Old" },
      ],
    });
  });

  it("leaves out scripts, mail, data, blobs, in-page anchors and the page itself", () => {
    page([
      { href: "javascript:void(0)" },
      { href: " JavaScript:alert(1)" },
      { href: "mailto:me@example.org" },
      { href: "data:text/plain,hi" },
      { href: "blob:https://example.org/1" },
      { href: "ftp://example.org/a.zip" },
      { href: "#section" },
      { href: "" },
      { href: null },
      { href: "http://[broken" },
      { href: "index.html" },
      { href: "index.html#other" },
      { href: "real.zip", text: "Real" },
    ]);
    expect(injected(false, 100).links).toEqual([{ url: "https://example.org/files/real.zip", text: "Real" }]);
  });

  it("lists each address once, ignoring fragments, and keeps the first text it finds", () => {
    page([
      { href: "a.zip#part1" },
      { href: "a.zip", text: "Archive" },
      { href: "a.zip#part2", text: "Later" },
      { href: "A.zip", text: "Other case" },
    ]);
    expect(injected(false, 100).links).toEqual([
      { url: "https://example.org/files/a.zip", text: "Archive" },
      { url: "https://example.org/files/A.zip", text: "Other case" },
    ]);
  });

  it("finds a text for links that show none", () => {
    page([
      { href: "1.zip", text: "  Two\n\n words  " },
      { href: "2.zip", title: "From the title" },
      { href: "3.zip", aria: "From aria-label" },
      { href: "4.zip", alt: "From the image" },
      { href: "5.zip" },
      { href: "6.zip", text: "x".repeat(500) },
    ]);
    expect(injected(false, 100).links.map((l) => l.text)).toEqual([
      "Two words",
      "From the title",
      "From aria-label",
      "From the image",
      "",
      "x".repeat(200),
    ]);
  });

  it("stops at the cap and says so", () => {
    page(Array.from({ length: 5 }, (_, i) => ({ href: `${i}.zip` })));
    const result = injected(false, 3);
    expect(result.links).toHaveLength(3);
    expect(result.truncated).toBe(true);
    page([{ href: "a.zip" }, { href: "a.zip#x" }]);
    expect(injected(false, 1).truncated).toBe(false);
  });

  it("keeps to the selection when there is one", () => {
    const links = [
      { href: "in.zip", selected: true },
      { href: "out.zip", selected: false },
    ];
    page(links, { selection: "some" });
    expect(injected(true, 100).links.map((l) => l.url)).toEqual(["https://example.org/files/in.zip"]);
    // Asked for all links: the selection does not matter.
    expect(injected(false, 100).links).toHaveLength(2);
    // Nothing selected after all: every link.
    page(links, { selection: "collapsed" });
    expect(injected(true, 100).links).toHaveLength(2);
    page(links, { selection: "none" });
    expect(injected(true, 100).links).toHaveLength(2);
  });
});
