import { describe, expect, it } from "vitest";

import { CATALOGS } from "./catalogs";
import { LOCALES, makeT, resolveLocale, type PluralForms } from "./core";
import { en } from "./en";

type Leaf = string | PluralForms;

/** Every text in a catalog by its dotted key. */
function leaves(tree: unknown, prefix = ""): Map<string, Leaf> {
  const out = new Map<string, Leaf>();
  if (typeof tree !== "object" || tree === null) return out;
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string" || (typeof value === "object" && value !== null && "other" in value)) {
      out.set(path, value as Leaf);
    } else {
      for (const [k, v] of leaves(value, path)) out.set(k, v);
    }
  }
  return out;
}

const placeholders = (text: string) => new Set([...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1] ?? ""));

/** Placeholders a text may use: those of English, in any of its forms. */
function allowed(leaf: Leaf): Set<string> {
  if (typeof leaf === "string") return placeholders(leaf);
  return new Set(Object.values(leaf).flatMap((text) => [...placeholders(text ?? "")]));
}

const english = leaves(en);

describe.each(LOCALES.filter((l) => l.code !== "en").map((l) => [l.code]))("%s catalog", (code) => {
  const catalog = leaves(CATALOGS[code as keyof typeof CATALOGS]);

  it("has every English text", () => {
    const missing = [...english.keys()].filter((key) => !catalog.has(key));
    expect(missing).toEqual([]);
  });

  it("has no texts English does not", () => {
    const extra = [...catalog.keys()].filter((key) => !english.has(key));
    expect(extra).toEqual([]);
  });

  it("keeps placeholders and plural forms", () => {
    const rules = new Intl.PluralRules(code);
    const problems: string[] = [];
    for (const [key, source] of english) {
      const text = catalog.get(key);
      if (text === undefined) continue;
      if ((typeof source === "string") !== (typeof text === "string")) {
        problems.push(`${key}: plural in one language only`);
        continue;
      }
      const may = allowed(source);
      const forms = typeof text === "string" ? [text] : Object.values(text).filter((v): v is string => typeof v === "string");
      for (const form of forms) {
        for (const name of placeholders(form)) {
          if (!may.has(name)) problems.push(`${key}: unknown {${name}}`);
        }
      }
      // Every plain-text placeholder must survive; a plural may drop {count} in a form that names the number.
      if (typeof source === "string" && typeof text === "string") {
        const kept = placeholders(text);
        for (const name of placeholders(source)) {
          if (!kept.has(name)) problems.push(`${key}: lost {${name}}`);
        }
      }
      if (typeof text !== "string") {
        // Each form the language uses (Russian: one, few, many, other).
        for (const category of rules.resolvedOptions().pluralCategories) {
          if (text[category] === undefined) problems.push(`${key}: no ${category} form`);
        }
      }
    }
    expect(problems).toEqual([]);
  });
});

describe("makeT", () => {
  it("picks plural forms and fills placeholders", () => {
    const t = makeT("en", en);
    expect(t("links.found", { count: 1 })).toBe("1 link found.");
    expect(t("links.found", { count: 3 })).toBe("3 links found.");
    expect(t("media.siteVideo", { site: "YouTube" })).toBe("YouTube video");
  });

  it("writes numbers the language's way and leaves strings alone", () => {
    expect(makeT("en", en)("links.downloadCount", { count: 1200 })).toBe("Download 1,200 links");
    expect(makeT("de", {})("links.downloadCount", { count: 1200 })).toBe("Download 1.200 links");
    expect(makeT("en", en)("options.errors.notSite", { line: "1200", value: "x" })).toContain("Line 1200:");
  });

  it("falls back to English for a missing text", () => {
    const t = makeT("tr", {});
    expect(t("menus.link")).toBe("Download with Downpipe");
  });
});

describe("resolveLocale", () => {
  it("follows the browser's languages, then English", () => {
    expect(resolveLocale("", ["de-DE", "en-US"])).toBe("de");
    expect(resolveLocale("", ["pt-PT"])).toBe("pt-BR");
    expect(resolveLocale("", ["zh-Hans-CN"])).toBe("zh-CN");
    expect(resolveLocale("", ["zh-TW", "ja-JP"])).toBe("ja");
    expect(resolveLocale("", ["xx"])).toBe("en");
    expect(resolveLocale("", [])).toBe("en");
  });
});
