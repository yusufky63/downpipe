// Alt+click (or the key the options page sets) on a link: the browser
// downloads it once instead of Downpipe. Registered by the service worker
// only while capture is on and the key is not switched off. It reads the key
// from the settings and sends nothing until such a click; the page cannot
// trigger it, because made-up clicks are not trusted.
import { ext } from "../lib/browser";
import { bypassUrl, isBypassClick, parseBypassKey, type BypassKey } from "../lib/bypass";

const SETTINGS_KEY = "settings";

let key: BypassKey = "off";

function readKey(stored: unknown): BypassKey {
  const settings = typeof stored === "object" && stored !== null ? (stored as Record<string, unknown>) : {};
  return parseBypassKey(settings.bypassKey);
}

void ext.storage.local
  .get(SETTINGS_KEY)
  .then((stored) => {
    key = readKey(stored[SETTINGS_KEY]);
  })
  .catch(() => undefined);

ext.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes[SETTINGS_KEY]) key = readKey(changes[SETTINGS_KEY].newValue);
});

// The capture phase on the window: before the page's own handlers, so a
// page that routes its links in script does not swallow the click first.
window.addEventListener(
  "click",
  (event) => {
    if (!isBypassClick(event, key)) return;
    const target = event.target instanceof Element ? event.target : null;
    const link = target?.closest<HTMLAnchorElement | HTMLAreaElement>("a[href], area[href]");
    const url = bypassUrl(link?.href);
    if (!url) return;
    // The browser's own Alt+click download would come on top of this one.
    event.preventDefault();
    event.stopImmediatePropagation();
    void ext.runtime.sendMessage({ kind: "bypass_download", url }).catch(() => undefined);
  },
  true,
);
