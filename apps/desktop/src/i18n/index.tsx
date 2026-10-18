// React side of the translations: pick the language from Settings and the
// system, switch to it before anything renders, and let components follow it.
import { createContext, useContext, useEffect, type ReactNode } from "react";

import { CATALOGS } from "./catalogs";
import { activate, currentLocale, LOCALES, resolveLocale, t, type Locale } from "./core";

export { LOCALES, t, type Locale, type MessageKey } from "./core";

/** The languages the system prefers, most preferred first. */
function systemLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

/**
 * Activate the language for `setting` ("" follows the system) and return it.
 * Call it at the top of a window's root component, before any text is made.
 */
export function useLocale(setting: string): Locale {
  const locale = resolveLocale(setting, systemLanguages());
  // Plain modules (the menu, state labels) read the active language while
  // this render runs, so it has to switch now rather than in an effect.
  activate(locale, CATALOGS[locale]);
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return locale;
}

const LocaleContext = createContext<Locale>("en");

export function LocaleProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  return <LocaleContext value={locale}>{children}</LocaleContext>;
}

/** The active language and the translate function; re-renders when the language changes. */
export function useI18n(): { locale: Locale; t: typeof t } {
  const locale = useContext(LocaleContext);
  return { locale: locale ?? currentLocale(), t };
}

/** Name of the language the system would pick, for "System default (…)". */
export function systemLocaleName(): string {
  const code = resolveLocale("", systemLanguages());
  return LOCALES.find((l) => l.code === code)?.name ?? "English";
}
