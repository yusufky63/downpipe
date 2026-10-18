// Translation core, shared by the service worker, the extension pages and the
// overlay content script. It mirrors apps/desktop/src/i18n/core.ts so
// translators work on both the same way. English is the source: every other
// language has the same shape, which the type checker enforces, and anything
// missing at run time falls back to it.
import { en } from "./en";

export const LOCALES = [
  { code: "en", name: "English" },
  { code: "tr", name: "Türkçe" },
  { code: "es", name: "Español" },
  { code: "de", name: "Deutsch" },
  { code: "fr", name: "Français" },
  { code: "it", name: "Italiano" },
  { code: "pt-BR", name: "Português (Brasil)" },
  { code: "ru", name: "Русский" },
  { code: "zh-CN", name: "简体中文" },
  { code: "ja", name: "日本語" },
] as const;

export type Locale = (typeof LOCALES)[number]["code"];

/** Plural forms as `Intl.PluralRules` names them; `other` is always required. */
export interface PluralForms {
  zero?: string;
  one?: string;
  two?: string;
  few?: string;
  many?: string;
  other: string;
}

/** English's structure with every text widened, so other languages can say anything. */
export type Shape<T> = T extends string ? string : T extends { other: string } ? PluralForms : { [K in keyof T]: Shape<T[K]> };

export type Messages = Shape<typeof en>;

/**
 * What a translation provides: any part of the English structure. Missing
 * texts fall back to English at run time; catalogs.test.ts fails until
 * every language has all of them.
 */
export type Catalog = DeepPartial<Messages>;

type DeepPartial<T> = { [K in keyof T]?: T[K] extends string ? string : T[K] extends PluralForms ? PluralForms : DeepPartial<T[K]> };

type KeyPaths<T, P extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : T[K] extends { other: string } ? `${P}${K}` : KeyPaths<T[K], `${P}${K}.`>;
}[keyof T & string];

export type MessageKey = KeyPaths<typeof en>;

export type Vars = Record<string, string | number>;

export type Translate = (key: MessageKey, vars?: Vars) => string;

function lookup(tree: unknown, key: string): string | PluralForms | undefined {
  let node: unknown = tree;
  for (const part of key.split(".")) {
    if (typeof node !== "object" || node === null) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  if (typeof node === "string") return node;
  if (typeof node === "object" && node !== null && "other" in node) return node as PluralForms;
  return undefined;
}

/**
 * A translate function for one language; `count` in `vars` picks the plural
 * form. Numbers are written the language's way ("1,200" in English); pass a
 * string to keep the digits as they are.
 */
export function makeT(locale: Locale, messages: Catalog | undefined): Translate {
  const rules = new Intl.PluralRules(locale);
  const numbers = new Intl.NumberFormat(locale);
  return (key, vars) => {
    const value = lookup(messages, key) ?? lookup(en, key) ?? key;
    let text: string;
    if (typeof value === "string") {
      text = value;
    } else {
      const form = rules.select(Number(vars?.count ?? 0));
      text = value[form] ?? value.other;
    }
    if (!vars) return text;
    return text.replace(/\{(\w+)\}/g, (match, name: string) => {
      const v = vars[name];
      if (v === undefined) return match;
      return typeof v === "number" ? numbers.format(v) : v;
    });
  };
}

const CODES: readonly string[] = LOCALES.map((l) => l.code);

/**
 * The language to show: `setting` if one is given (the extension has none
 * and passes ""), else the first of the browser's languages Downpipe has,
 * else English. `pt` means Brazilian Portuguese and `zh` simplified Chinese;
 * traditional Chinese falls back to English rather than to a script its
 * readers did not ask for.
 */
export function resolveLocale(setting: string, preferred: readonly string[]): Locale {
  const candidates = setting ? [setting, ...preferred] : preferred;
  for (const raw of candidates) {
    const tag = raw.trim();
    if (!tag) continue;
    const exact = CODES.find((c) => c.toLowerCase() === tag.toLowerCase());
    if (exact) return exact as Locale;
    const lower = tag.toLowerCase();
    if (lower.startsWith("zh")) {
      if (/hant|-tw|-hk|-mo/.test(lower)) continue;
      return "zh-CN";
    }
    if (lower.startsWith("pt")) return "pt-BR";
    const base = lower.split("-")[0];
    const match = CODES.find((c) => c.toLowerCase() === base);
    if (match) return match as Locale;
  }
  return "en";
}

let active: { locale: Locale; translate: Translate; numbers: Intl.NumberFormat } = {
  locale: "en",
  translate: makeT("en", en),
  numbers: new Intl.NumberFormat("en"),
};

/**
 * Switch the language for everything in this script. Library modules read it
 * when they build a text, never at load, so activating at the start of an
 * entry point covers them all.
 */
export function activate(locale: Locale, messages: Catalog | undefined): void {
  if (active.locale !== locale) {
    active = { locale, translate: makeT(locale, messages), numbers: new Intl.NumberFormat(locale) };
  }
}

/** Translate with the active language. */
export const t: Translate = (key, vars) => active.translate(key, vars);

export function currentLocale(): Locale {
  return active.locale;
}

/** A number outside a text (a chip's count), written the active language's way. */
export function formatNumber(n: number): string {
  return active.numbers.format(n);
}
