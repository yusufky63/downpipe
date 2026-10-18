// Translation core without React, so plain modules and tests can use it.
// English is the source: every other language has the same shape, which the
// type checker enforces, and anything missing at run time falls back to it.
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
 * texts fall back to English at run time; catalogs.test.ts fails the build
 * until every language has all of them.
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

/** A translate function for one language; `count` in `vars` picks the plural form. */
export function makeT(locale: Locale, messages: Catalog | undefined): Translate {
  const rules = new Intl.PluralRules(locale);
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
    return text.replace(/\{(\w+)\}/g, (match, name: string) => (vars[name] === undefined ? match : String(vars[name])));
  };
}

const CODES: readonly string[] = LOCALES.map((l) => l.code);

/**
 * The language to show: the one picked in Settings, else the first of the
 * system's languages Downpipe has, else English. `pt` means Brazilian
 * Portuguese and `zh` simplified Chinese; traditional Chinese falls back to
 * English rather than to a script its readers did not ask for.
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

let active: { locale: Locale; translate: Translate } = { locale: "en", translate: makeT("en", en) };

/**
 * Switch the language for everything, including plain modules such as the
 * menu builder that run during rendering. The provider calls it before its
 * children render.
 */
export function activate(locale: Locale, messages: Catalog | undefined): void {
  if (active.locale !== locale) active = { locale, translate: makeT(locale, messages) };
}

/** Translate with the active language. */
export const t: Translate = (key, vars) => active.translate(key, vars);

export function currentLocale(): Locale {
  return active.locale;
}
