// The extension pages hold translation keys instead of English (ui/dom.ts
// fills them at load). The type checker cannot see into HTML, so this checks
// that every key exists and that no English was left in the markup.
import { describe, expect, it } from "vitest";

import linksHtml from "../links/links.html?raw";
import optionsHtml from "../options/options.html?raw";
import popupHtml from "../popup/popup.html?raw";
import { en } from "./en";

const PAGES: [string, string][] = [
  ["popup", popupHtml],
  ["options", optionsHtml],
  ["links", linksHtml],
];

/** Text the markup may keep: the product name, units and code samples. */
const LITERAL = new Set(["Downpipe", "KiB", "MiB"]);

function english(key: string): unknown {
  let node: unknown = en;
  for (const part of key.split(".")) node = (node as Record<string, unknown> | undefined)?.[part];
  return node;
}

describe.each(PAGES)("%s page", (_name, html) => {
  it("uses only keys English has, as plain texts", () => {
    const keys = [...html.matchAll(/data-i18n(?:-[a-z-]+)?="([^"]+)"/g)].map((m) => m[1] ?? "");
    expect(keys.length).toBeGreaterThan(0);
    expect(keys.filter((key) => typeof english(key) !== "string")).toEqual([]);
  });

  it("leaves no English in the markup", () => {
    const withoutSamples = html.replace(/<code[^>]*data-var="[^"]*"[^>]*>[^<]*<\/code>/g, "");
    const texts = [...withoutSamples.matchAll(/>([^<]+)</g)].map((m) => (m[1] ?? "").trim()).filter(Boolean);
    expect(texts.filter((text) => !LITERAL.has(text))).toEqual([]);
    // Attributes people read come from the catalog too.
    expect(html).not.toMatch(/\s(?:title|aria-label)="/);
  });

  it("gives each code sample a place in its text", () => {
    for (const m of html.matchAll(/<p\b[^>]*data-i18n="([^"]+)"[^>]*>([\s\S]*?)<\/p>/g)) {
      const names = [...(m[2] ?? "").matchAll(/data-var="(\w+)"/g)].map((v) => `{${v[1]}}`);
      const text = String(english(m[1] ?? ""));
      for (const name of names) expect(text, m[1]).toContain(name);
    }
  });
});
