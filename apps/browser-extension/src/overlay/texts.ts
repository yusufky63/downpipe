// The Download button's texts in every language, and nothing else: this
// script is injected into every frame while media detection is on, so it
// must not carry each language's whole catalog (about 200 KB).
import { activate, resolveLocale, type Catalog, type Locale } from "../i18n/core";
import { common as deCommon, results as deResults } from "../i18n/de/common";
import { overlay as deOverlay } from "../i18n/de/overlay";
import { common as enCommon, results as enResults } from "../i18n/en/common";
import { overlay as enOverlay } from "../i18n/en/overlay";
import { common as esCommon, results as esResults } from "../i18n/es/common";
import { overlay as esOverlay } from "../i18n/es/overlay";
import { common as frCommon, results as frResults } from "../i18n/fr/common";
import { overlay as frOverlay } from "../i18n/fr/overlay";
import { common as itCommon, results as itResults } from "../i18n/it/common";
import { overlay as itOverlay } from "../i18n/it/overlay";
import { common as jaCommon, results as jaResults } from "../i18n/ja/common";
import { overlay as jaOverlay } from "../i18n/ja/overlay";
import { common as ptCommon, results as ptResults } from "../i18n/pt-BR/common";
import { overlay as ptOverlay } from "../i18n/pt-BR/overlay";
import { common as ruCommon, results as ruResults } from "../i18n/ru/common";
import { overlay as ruOverlay } from "../i18n/ru/overlay";
import { common as trCommon, results as trResults } from "../i18n/tr/common";
import { overlay as trOverlay } from "../i18n/tr/overlay";
import { common as zhCommon, results as zhResults } from "../i18n/zh-CN/common";
import { overlay as zhOverlay } from "../i18n/zh-CN/overlay";

/** The namespaces the button and its result messages use. */
export const OVERLAY_CATALOGS: Record<Locale, Catalog> = {
  en: { common: enCommon, results: enResults, overlay: enOverlay },
  tr: { common: trCommon, results: trResults, overlay: trOverlay },
  es: { common: esCommon, results: esResults, overlay: esOverlay },
  de: { common: deCommon, results: deResults, overlay: deOverlay },
  fr: { common: frCommon, results: frResults, overlay: frOverlay },
  it: { common: itCommon, results: itResults, overlay: itOverlay },
  "pt-BR": { common: ptCommon, results: ptResults, overlay: ptOverlay },
  ru: { common: ruCommon, results: ruResults, overlay: ruOverlay },
  "zh-CN": { common: zhCommon, results: zhResults, overlay: zhOverlay },
  ja: { common: jaCommon, results: jaResults, overlay: jaOverlay },
};

/** The browser's language, as the service worker picks it, for the button. */
export function activateOverlayLanguage(): Locale {
  const tags = typeof navigator === "undefined" ? [] : navigator.languages?.length ? navigator.languages : [navigator.language];
  const locale = resolveLocale("", tags);
  activate(locale, OVERLAY_CATALOGS[locale]);
  return locale;
}
