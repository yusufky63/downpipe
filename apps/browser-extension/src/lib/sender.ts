/** The parts of `runtime.MessageSender` the check needs. */
export interface SenderLike {
  id?: string;
  url?: string;
  origin?: string;
}

/**
 * Accept messages only from this extension's own pages (the popup, or the
 * popup opened in a tab). Content scripts would carry the page's URL and
 * origin, and other extensions a different id.
 *
 * `extensionBaseUrl` is `runtime.getURL("")`, e.g. `chrome-extension://<id>/`
 * or `moz-extension://<uuid>/`. Compared as strings: `new URL()` reports the
 * origin of those schemes as "null" outside the browser.
 */
export function isTrustedSender(sender: SenderLike, extensionId: string, extensionBaseUrl: string): boolean {
  if (!extensionId || sender.id !== extensionId) return false;
  const prefix = extensionBaseUrl.endsWith("/") ? extensionBaseUrl : `${extensionBaseUrl}/`;
  if (typeof sender.url !== "string" || !sender.url.startsWith(prefix)) return false;
  // Chrome also reports the origin; Firefox may leave it out.
  return sender.origin === undefined || sender.origin === prefix.slice(0, -1);
}

/**
 * This extension's content script (the overlay button) in an http(s) page
 * of a tab. Web pages cannot message the extension at all (there is no
 * `externally_connectable`), so the id identifies the script; its requests
 * are still checked as untrusted, since the page around it is.
 */
export function isOwnContentScript<S extends SenderLike & { tab?: { id?: number } }>(
  sender: S,
  extensionId: string,
): sender is S & { url: string; tab: { id: number } } {
  if (!extensionId || sender.id !== extensionId) return false;
  const tabId = sender.tab?.id;
  if (tabId === undefined || !Number.isSafeInteger(tabId) || tabId < 0) return false;
  return typeof sender.url === "string" && /^https?:\/\//.test(sender.url);
}
