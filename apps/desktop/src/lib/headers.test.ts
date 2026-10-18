import { describe, expect, it } from "vitest";

import { headersText, parseHeaders, settableHeader } from "./headers";

describe("parseHeaders", () => {
  it("reads one header per line and skips blank lines", () => {
    expect(parseHeaders("Referer: https://a.test/page\r\n\n  Cookie:  a=1; b=2  \nX-Empty:")).toEqual({
      headers: [
        ["Referer", "https://a.test/page"],
        ["Cookie", "a=1; b=2"],
        ["X-Empty", ""],
      ],
      problems: [],
    });
  });

  it("points at lines it cannot use", () => {
    expect(parseHeaders("no colon here\n: value\nBad Name: x\nRange: bytes=0-\nOk: 1").problems).toEqual([
      { line: 1, kind: "format" },
      { line: 2, kind: "format" },
      { line: 3, kind: "name", name: "Bad Name" },
      { line: 4, kind: "managed", name: "Range" },
    ]);
  });

  it("round-trips through the field's text", () => {
    const headers: [string, string][] = [
      ["Authorization", "Bearer x:y"],
      ["Accept", "*/*"],
    ];
    expect(parseHeaders(headersText(headers)).headers).toEqual(headers);
  });
});

describe("settableHeader", () => {
  it("refuses names the engine manages and malformed ones", () => {
    expect(settableHeader("Cookie")).toBe(true);
    expect(settableHeader("accept-encoding")).toBe(false);
    expect(settableHeader("Host")).toBe(false);
    expect(settableHeader("a b")).toBe(false);
  });
});
