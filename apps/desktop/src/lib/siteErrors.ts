// yt-dlp's common failures in the user's language, with what to do. The app
// sorts yt-dlp's English output into these codes (crates/providers/src/errors.rs,
// `site_*` and `unavailable`), so the list, the details and the quality
// dialog explain them without showing yt-dlp's own words.
import { t, type MessageKey } from "../i18n/core";

export const SITE_FAILURE_TEXTS: Partial<Record<string, MessageKey>> = {
  site_forbidden: "media.siteErrors.forbidden",
  site_sign_in: "media.siteErrors.signIn",
  site_age_restricted: "media.siteErrors.ageRestricted",
  site_js_runtime: "media.siteErrors.jsRuntime",
  site_po_token: "media.siteErrors.poToken",
  site_outdated: "media.siteErrors.outdated",
  unavailable: "media.siteErrors.unavailable",
};

/** Failures Settings, Media can fix: a missing tool, or a yt-dlp to update. */
export const SITE_SETUP_KINDS: ReadonlySet<string> = new Set([
  "tool_missing",
  "site_forbidden",
  "site_js_runtime",
  "site_po_token",
  "site_outdated",
]);

/** The translated explanation of a site failure, or null for other codes. */
export function siteFailureText(code: string | null | undefined): string | null {
  const key = code ? SITE_FAILURE_TEXTS[code] : undefined;
  return key ? t(key) : null;
}
