// The extension speaks the browser's language and has no setting of its own:
// the service worker, the pages and the overlay all read `navigator.languages`,
// so they always agree, and a change takes effect when the browser restarts.
import { CATALOGS } from "./catalogs";
import { activate, resolveLocale, type Locale } from "./core";

export { LOCALES, currentLocale, formatNumber, t, type Locale, type MessageKey } from "./core";

/** The languages the browser prefers, most preferred first. */
function browserLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

/**
 * A language without any translated text is passed over: its English
 * fallback would otherwise be shown under its `lang` (CSS upper-cases
 * "file" to "FİLE" under Turkish) and with its number format.
 */
function hasTexts(locale: Locale): boolean {
  return locale === "en" || Object.keys(CATALOGS[locale]).length > 0;
}

/**
 * Activate the browser's language and return it. Every entry point calls
 * this first, before it builds a menu, a notification or a page.
 */
export function activateBrowserLanguage(): Locale {
  // Tags Downpipe has no language for resolve to "en" on their own and are
  // kept, so resolveLocale still skips them in favour of the next one.
  const usable = browserLanguages().filter((tag) => hasTexts(resolveLocale("", [tag])));
  const locale = resolveLocale("", usable);
  activate(locale, CATALOGS[locale]);
  return locale;
}
