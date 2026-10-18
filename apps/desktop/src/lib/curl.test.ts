import { describe, expect, it } from "vitest";

import { looksLikeCurl, parseCurl, shellWords } from "./curl";

function request(text: string) {
  const parsed = parseCurl(text);
  if (!parsed.ok) throw new Error(`refused: ${JSON.stringify(parsed.error)}`);
  return parsed.request;
}

describe("shellWords", () => {
  it("reads bash quoting and line continuations", () => {
    expect(shellWords(`curl 'https://a.test/x y' \\\n  -H "A: \\"b\\"" $'c\\'d\\x41'`)).toEqual([
      "curl",
      "https://a.test/x y",
      "-H",
      'A: "b"',
      "c'dA",
    ]);
  });

  it("reads Windows cmd quoting with caret escapes", () => {
    expect(shellWords(`curl ^"https://a.test/?q=1^&r=2^" ^\r\n  -H ^"x-test: ^\\^"v^%^\\^"^"`)).toEqual([
      "curl",
      "https://a.test/?q=1&r=2",
      "-H",
      'x-test: "v%"',
    ]);
  });

  it("reports a quote that never closes", () => {
    expect(shellWords("curl 'https://a.test/")).toBeNull();
    expect(shellWords('curl "https://a.test/')).toBeNull();
  });
});

describe("parseCurl", () => {
  it("takes the link, headers and cookies from Chrome's bash copy", () => {
    const r = request(
      [
        "curl 'https://cdn.example.org/files/big.iso?token=abc' \\",
        "  -H 'accept: */*' \\",
        "  -H 'accept-language: en-US,en;q=0.9' \\",
        "  -b 'session=s1; theme=dark' \\",
        "  -H 'referer: https://example.org/downloads' \\",
        "  -H 'user-agent: Mozilla/5.0'",
      ].join("\n"),
    );
    expect(r.url).toBe("https://cdn.example.org/files/big.iso?token=abc");
    expect(r.headers).toEqual([
      ["accept", "*/*"],
      ["accept-language", "en-US,en;q=0.9"],
      ["referer", "https://example.org/downloads"],
      ["user-agent", "Mozilla/5.0"],
      ["Cookie", "session=s1; theme=dark"],
    ]);
    expect(r.ignored).toEqual([]);
  });

  it("reads Chrome's cmd copy", () => {
    const r = request(`curl ^"https://a.test/f.zip^" ^\r\n  -H ^"accept: */*^" ^\r\n  -H ^"cookie: a=1^" ^\r\n  --compressed`);
    expect(r.url).toBe("https://a.test/f.zip");
    expect(r.headers).toEqual([
      ["accept", "*/*"],
      ["Cookie", "a=1"],
    ]);
  });

  it("joins cookies from headers and -b into one header", () => {
    const r = request(`curl https://a.test/ -H 'Cookie: a=1' --cookie 'b=2'`);
    expect(r.headers).toEqual([["Cookie", "a=1; b=2"]]);
  });

  it("leaves out headers the engine manages and says so", () => {
    const r = request(`curl https://a.test/f -H 'Range: bytes=0-' -H 'Accept-Encoding: gzip' -H 'X-Keep: 1' -k`);
    expect(r.headers).toEqual([["X-Keep", "1"]]);
    expect(r.ignored).toEqual(["Range", "Accept-Encoding", "-k"]);
  });

  it("maps the user agent, referer, basic auth, bearer and output name", () => {
    const r = request(
      `curl -sSL -A 'Agent/1' -e 'https://ref.test/;auto' -u 'me:pässword' --oauth2-bearer T0K -o /tmp/out.bin --url https://a.test/x`,
    );
    expect(r.url).toBe("https://a.test/x");
    expect(r.fileName).toBe("out.bin");
    expect(r.headers).toEqual([
      ["User-Agent", "Agent/1"],
      ["Referer", "https://ref.test/"],
      ["Authorization", "Bearer T0K"],
    ]);
  });

  it("encodes basic credentials as UTF-8", () => {
    const r = request(`curl -u 'me:pässword' https://a.test/`);
    expect(r.headers).toEqual([["Authorization", "Basic bWU6cMOkc3N3b3Jk"]]);
  });

  it("takes values written right after a short option", () => {
    const r = request(`curl -HX-One:1 -bk=v https://a.test/`);
    expect(r.headers).toEqual([
      ["X-One", "1"],
      ["Cookie", "k=v"],
    ]);
  });

  it("accepts an explicit GET and turns -G data into the query", () => {
    expect(request(`curl -X get https://a.test/`).url).toBe("https://a.test/");
    expect(request(`curl -G https://a.test/s?a=1 -d b=2 --data-urlencode 'q=x y'`).url).toBe("https://a.test/s?a=1&b=2&q=x%20y");
  });

  it("refuses POST, other methods and bodies", () => {
    expect(parseCurl(`curl -X POST https://a.test/`)).toEqual({ ok: false, error: { kind: "method", method: "POST" } });
    expect(parseCurl(`curl --request put https://a.test/`)).toEqual({ ok: false, error: { kind: "method", method: "PUT" } });
    expect(parseCurl(`curl -I https://a.test/`)).toEqual({ ok: false, error: { kind: "method", method: "HEAD" } });
    expect(parseCurl(`curl https://a.test/ --data-raw '{"a":1}'`)).toEqual({ ok: false, error: { kind: "data", option: "--data-raw" } });
    expect(parseCurl(`curl https://a.test/ -F file=@x`)).toEqual({ ok: false, error: { kind: "data", option: "-F" } });
    expect(parseCurl(`curl -G https://a.test/ --json '{}'`)).toEqual({ ok: false, error: { kind: "data", option: "--json" } });
  });

  it("refuses what is not a usable command", () => {
    expect(parseCurl("wget https://a.test/")).toEqual({ ok: false, error: { kind: "not_curl" } });
    expect(parseCurl("https://a.test/")).toEqual({ ok: false, error: { kind: "not_curl" } });
    expect(parseCurl("curl -H 'a: b'")).toEqual({ ok: false, error: { kind: "no_url" } });
    expect(parseCurl("curl ftp://a.test/f")).toEqual({ ok: false, error: { kind: "no_url" } });
    expect(parseCurl("curl 'https://a.test/")).toEqual({ ok: false, error: { kind: "quote" } });
  });

  it("knows curl.exe and a path to it", () => {
    expect(request(`curl.exe https://a.test/`).url).toBe("https://a.test/");
    expect(request(`/usr/bin/curl https://a.test/`).url).toBe("https://a.test/");
  });
});

describe("looksLikeCurl", () => {
  it("spots a pasted command", () => {
    expect(looksLikeCurl("  curl 'https://a.test/'")).toBe(true);
    expect(looksLikeCurl("CURL.EXE https://a.test/")).toBe(true);
    expect(looksLikeCurl("https://a.test/curl")).toBe(false);
  });
});
