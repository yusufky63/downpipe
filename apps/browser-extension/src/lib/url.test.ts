import { describe, expect, it } from "vitest";
import { MAX_URL_LENGTH, baseName, linkLabel, referrerFor, webUrl, withoutSecrets } from "./url";

describe("webUrl", () => {
  it("accepts http and https only", () => {
    expect(webUrl("https://example.org/a")?.href).toBe("https://example.org/a");
    expect(webUrl("  http://example.org  ")?.href).toBe("http://example.org/");
    for (const bad of [
      "javascript:alert(1)",
      "data:text/plain,x",
      "blob:https://example.org/1",
      "file:///etc/passwd",
      "ftp://example.org/",
      "chrome://settings",
      "not a url",
      "",
      undefined,
      null,
    ]) {
      expect(webUrl(bad), String(bad)).toBeNull();
    }
  });

  it("refuses control characters instead of letting the parser drop them", () => {
    expect(webUrl("https://example.org/a\nb")).toBeNull();
    expect(webUrl("https://example.org/\tx")).toBeNull();
    expect(webUrl("https://example.org/\u0085")).toBeNull();
  });

  it("caps the length", () => {
    expect(webUrl(`https://example.org/${"a".repeat(MAX_URL_LENGTH)}`)).toBeNull();
  });
});

describe("withoutSecrets", () => {
  it("drops credentials and the fragment", () => {
    const url = withoutSecrets(new URL("https://user:pw@example.org/p?q=1#top"));
    expect(url.href).toBe("https://example.org/p?q=1");
  });
});

describe("referrerFor", () => {
  it("sends the full page to the same origin", () => {
    expect(referrerFor("https://example.org/dl?x=1#frag", "https://example.org/file.zip")).toBe(
      "https://example.org/dl?x=1",
    );
  });

  it("sends only the origin to other sites", () => {
    expect(referrerFor("https://example.org/private/page", "https://cdn.example.net/f.zip")).toBe(
      "https://example.org/",
    );
    expect(referrerFor("http://example.org:8080/a", "http://example.org/b")).toBe("http://example.org:8080/");
  });

  it("sends nothing on a downgrade or from a non-web page", () => {
    expect(referrerFor("https://example.org/", "http://example.org/f.zip")).toBeUndefined();
    expect(referrerFor("file:///C:/page.html", "https://example.org/f.zip")).toBeUndefined();
    expect(referrerFor(undefined, "https://example.org/f.zip")).toBeUndefined();
  });

  it("never carries credentials", () => {
    expect(referrerFor("https://me:secret@example.org/a", "https://example.org/b")).toBe("https://example.org/a");
  });
});

describe("baseName and linkLabel", () => {
  it("takes the last component of either path style", () => {
    expect(baseName("C:\\Users\\me\\Downloads\\a.zip")).toBe("a.zip");
    expect(baseName("/home/me/Downloads/b.tar.gz")).toBe("b.tar.gz");
    expect(baseName("plain.txt")).toBe("plain.txt");
  });

  it("labels links by file name, falling back to the host", () => {
    expect(linkLabel("https://example.org/files/r%C3%A9sum%C3%A9.pdf?x=1")).toBe("résumé.pdf");
    expect(linkLabel("https://example.org/")).toBe("example.org");
    expect(linkLabel("https://example.org/%E0%A4%A")).toBe("%E0%A4%A");
  });
});
