import { describe, expect, it } from "vitest";
import { isTrustedSender } from "./sender";

const ID = "efgicfboaldflckpncfdojkeoaamflpf";
const BASE = `chrome-extension://${ID}/`;

describe("isTrustedSender", () => {
  it("accepts this extension's popup in Chrome", () => {
    expect(isTrustedSender({ id: ID, url: `${BASE}popup.html`, origin: `chrome-extension://${ID}` }, ID, BASE)).toBe(
      true,
    );
  });

  it("accepts the popup in Firefox, which may omit the origin", () => {
    const base = "moz-extension://4b0d-uuid/";
    expect(isTrustedSender({ id: "downpipe@downpipe.dev", url: `${base}popup.html` }, "downpipe@downpipe.dev", base)).toBe(
      true,
    );
  });

  it("rejects content scripts and web pages", () => {
    expect(isTrustedSender({ id: ID, url: "https://evil.example/", origin: "https://evil.example" }, ID, BASE)).toBe(
      false,
    );
    // Right URL prefix but a page origin: a forged or confused sender.
    expect(isTrustedSender({ id: ID, url: `${BASE}popup.html`, origin: "https://evil.example" }, ID, BASE)).toBe(false);
  });

  it("rejects other extensions", () => {
    const other = "aaaabbbbccccddddeeeeffffgggghhhh";
    expect(isTrustedSender({ id: other, url: `chrome-extension://${other}/x.html` }, ID, BASE)).toBe(false);
    // An id that merely starts like ours.
    expect(isTrustedSender({ id: ID, url: `chrome-extension://${ID}evil/x.html` }, ID, BASE)).toBe(false);
  });

  it("rejects senders without an id or URL", () => {
    expect(isTrustedSender({}, ID, BASE)).toBe(false);
    expect(isTrustedSender({ id: ID }, ID, BASE)).toBe(false);
    expect(isTrustedSender({ id: "", url: BASE }, "", BASE)).toBe(false);
  });
});
