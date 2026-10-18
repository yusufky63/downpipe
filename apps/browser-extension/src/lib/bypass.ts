// Alt+click (or Shift+click, as the options page sets it) on a link lets the
// browser download that one file itself, past capture. The content script
// (bypass/bypass.ts) spots the click and asks the service worker, which
// starts the download as the extension's own: capture always leaves those
// alone (`decideCapture`'s `own_download`), so nothing else is needed to let
// exactly this one through.
import { webUrl } from "./url";

export type BypassKey = "alt" | "shift" | "off";

export const BYPASS_KEYS: readonly BypassKey[] = ["alt", "shift", "off"];

/** Alt+click is how Chrome and Edge save a link anyway, so it is the natural default. */
export const DEFAULT_BYPASS_KEY: BypassKey = "alt";

export function isBypassKey(value: unknown): value is BypassKey {
  return BYPASS_KEYS.includes(value as BypassKey);
}

export function parseBypassKey(value: unknown): BypassKey {
  return isBypassKey(value) ? value : DEFAULT_BYPASS_KEY;
}

/** The parts of a `MouseEvent` the decision needs. */
export interface ClickLike {
  button: number;
  altKey: boolean;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  /** False for clicks a page script made up, which must never start a download. */
  isTrusted: boolean;
}

/**
 * A real primary-button click with the chosen key and no other modifier:
 * Ctrl or Cmd with it is the browser's own "open in a new tab" and similar,
 * which stay the browser's.
 */
export function isBypassClick(e: ClickLike, key: BypassKey): boolean {
  if (key === "off" || !e.isTrusted || e.button !== 0 || e.ctrlKey || e.metaKey) return false;
  return key === "alt" ? e.altKey && !e.shiftKey : e.shiftKey && !e.altKey;
}

/** The link's address when it is one capture would take (http or https), else null. */
export function bypassUrl(href: string | null | undefined): string | null {
  return webUrl(href)?.href ?? null;
}

/** The content script's only request: download this link in the browser, once. */
export interface BypassRequest {
  kind: "bypass_download";
  url: string;
}

/** Exact shape only, like the other requests from content scripts. */
export function parseBypassRequest(value: unknown): BypassRequest | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const v = value as Record<string, unknown>;
  const keys = Object.keys(v);
  if (keys.length !== 2 || v.kind !== "bypass_download" || typeof v.url !== "string") return null;
  const url = bypassUrl(v.url);
  return url ? { kind: "bypass_download", url } : null;
}
