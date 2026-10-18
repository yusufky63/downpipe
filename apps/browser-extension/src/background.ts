// Service worker (Chrome, Edge) or event page (Firefox). Every listener is
// registered synchronously at the top level, so the browser can wake the
// worker for any of them.
import { activateBrowserLanguage, t } from "./i18n";
import { ext } from "./lib/browser";
import { parseBypassRequest } from "./lib/bypass";
import { linkFields } from "./lib/capture";
import { collectPageLinks } from "./lib/collect";
import { hostOf, matchingSite, normalizeSitePattern, withSite } from "./lib/filters";
import { captureHeld, captureStarted, claimIn, parseOffered, type HandoffDeps } from "./lib/handoff";
import { callHost } from "./lib/host";
import {
  MAX_PAGE_LINKS,
  collectionFromPage,
  collectionKey,
  staleCollectionKeys,
  unreadableCollection,
  type LinkCollection,
} from "./lib/links";
import { formatBytes } from "./lib/format";
import {
  WEB_ORIGINS,
  addMedia,
  classifyResponse,
  classifySource,
  cleanTitle,
  excludeMatchesFor,
  frameOf,
  kindLabel,
  mediaForVideo,
  mediaLabels,
  mediaName,
  pageHeaders,
  parseTabMedia,
  parseTabParts,
  playlistKey,
  readPlaylist,
  shouldReadPlaylist,
  tabMediaKey,
  tabPartsKey,
  titleFileName,
  type MediaCandidate,
  type PlaylistInfo,
  type TabMedia,
} from "./lib/media";
import {
  PAGE_ITEM_ID,
  addActivity,
  isPlace,
  parseActivity,
  parseOverlayRequest,
  parsePopupRequest,
  parseSettings,
  taskIdsOf,
  type Activity,
  type LinkResult,
  type MediaReply,
  type OverlayItem,
  type OverlayPrefs,
  type OverlayRequest,
  type PageItem,
  type PingReply,
  type PopupRequest,
  type Settings,
  type StatusReply,
  type TasksReply,
} from "./lib/messages";
import { notificationFor } from "./lib/notify";
import {
  downloadMessage,
  mediaMessage,
  newMessageId,
  pageMessage,
  pingMessage,
  tasksMessage,
  type TaskStatus,
} from "./lib/protocol";
import { isOwnContentScript, isTrustedSender } from "./lib/sender";
import { isSiteMedia, pageTitle, sitePage } from "./lib/sites";
import {
  bridgeDown,
  bridgeStatus,
  downloadOutcome,
  pageOutcome,
  streamOutcome,
  type HandOver,
} from "./lib/status";
import { linkLabel, referrerFor } from "./lib/url";

const MENU_ID = "downpipe-download-link";
const PAGE_MENU_ID = "downpipe-page-links";
const SELECTION_MENU_ID = "downpipe-selected-links";
const VIDEO_MENU_ID = "downpipe-video-page";
const TORRENT_MENU_ID = "downpipe-magnet-link";
/** Pages `lib/sites.ts` recognizes, as match patterns for the page menu. */
const VIDEO_PAGES = [
  "*://*.youtube.com/watch*",
  "*://*.youtube.com/shorts/*",
  "*://*.youtube.com/embed/*",
  "*://*.youtube.com/live/*",
  "*://*.youtube-nocookie.com/embed/*",
  "*://youtu.be/*",
];
const WEB_PAGES = ["http://*/*", "https://*/*"];
const LINK_TIMEOUT_MS = 10_000;
/** A captured download waits for this answer, so it is kept short. */
const CAPTURE_TIMEOUT_MS = 5_000;
/**
 * A later answer is still acted on: the host waits 10 seconds for the app,
 * so an acceptance can arrive after the browser went on with its copy.
 */
const CAPTURE_LATE_MS = 15_000;
const OFFERED_KEY = "offeredDownloads";
const STATUS_TIMEOUT_MS = 5_000;
const DOWNLOADS_PERMISSION: chrome.permissions.Permissions = { permissions: ["downloads"] };
const ACTIVITY_KEY = "activity";
const SETTINGS_KEY = "settings";
/** Successful hand-offs leave the notification center after this long. */
const NOTIFICATION_MS = 6_000;
// The badge API takes colors, not CSS variables: brand teal and danger.
const BADGE_OK = "#0b7a75";
const BADGE_FAIL = "#c62a0f";
const BADGE_MS = { ok: 4_000, fail: 10_000 };

const sendNative = (host: string, message: object): Promise<unknown> =>
  ext.runtime.sendNativeMessage(host, message);

// Before anything is worded: menus, notifications, the toolbar tooltip and
// the replies to the pages and the overlay.
activateBrowserLanguage();

// ---------- Context menus ----------

// Created on install and at every browser start, which is also when a new
// browser language takes effect, so the titles never need updating.
function createMenu(): void {
  void ext.contextMenus.removeAll().then(() => {
    ext.contextMenus.create({
      id: MENU_ID,
      title: t("menus.link"),
      contexts: ["link"],
      targetUrlPatterns: WEB_PAGES,
    });
    // `page` only shows where no more specific context applies, so this
    // does not crowd the link menu above.
    ext.contextMenus.create({
      id: PAGE_MENU_ID,
      title: t("menus.pageLinks"),
      contexts: ["page", "frame"],
      documentUrlPatterns: WEB_PAGES,
    });
    // On a video page the page itself is the download: Downpipe's site
    // provider (yt-dlp) gets its address and picks the video out.
    ext.contextMenus.create({
      id: VIDEO_MENU_ID,
      title: t("menus.video"),
      contexts: ["page", "frame"],
      documentUrlPatterns: VIDEO_PAGES,
    });
    // Magnet links are not web pages, so the link item above never shows on
    // them. A browser whose match patterns refuse the scheme leaves this
    // item out; the links page still lists magnet links.
    ext.contextMenus.create(
      {
        id: TORRENT_MENU_ID,
        title: t("menus.torrent"),
        contexts: ["link"],
        targetUrlPatterns: ["magnet:*"],
      },
      () => void ext.runtime.lastError,
    );
    ext.contextMenus.create({
      id: SELECTION_MENU_ID,
      title: t("menus.selection"),
      contexts: ["selection"],
      documentUrlPatterns: WEB_PAGES,
    });
  });
}

ext.runtime.onInstalled.addListener(createMenu);
ext.runtime.onStartup.addListener(createMenu);

ext.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === MENU_ID || info.menuItemId === TORRENT_MENU_ID) {
    void sendLink(info);
    return;
  }
  if (info.menuItemId === VIDEO_MENU_ID) {
    // The frame first: a video embedded in another site's page.
    const url = sitePage(info.frameUrl) ? info.frameUrl : info.pageUrl;
    const title = url === tab?.url ? pageTitle(cleanTitle(tab?.title)) : undefined;
    if (url) void sendVideoPage(url, title, "link");
    return;
  }
  // Choosing the item grants `activeTab` for this tab, which is what lets
  // the collector run there.
  if ((info.menuItemId === PAGE_MENU_ID || info.menuItemId === SELECTION_MENU_ID) && tab) {
    void openLinks(tab, info.frameId, info.menuItemId === SELECTION_MENU_ID);
  }
});

async function sendLink(info: chrome.contextMenus.OnClickData): Promise<void> {
  // A link to a video page goes to the site provider, not as a file.
  if (sitePage(info.linkUrl) && info.linkUrl) {
    await sendVideoPage(info.linkUrl, undefined, "link");
    return;
  }
  const label = linkLabel(info.linkUrl ?? "");
  const fields = linkFields(info);
  const message = fields && downloadMessage(fields, newMessageId());
  if (!message) {
    await report({
      source: "link",
      label,
      outcome: "failed",
      message: t("errors.code.invalid_url"),
    });
    return;
  }
  const result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
  const { outcome, message: text, fileName, taskId, awaitingConfirmation } = downloadOutcome(result);
  await report({ source: "link", label: fileName ?? label, outcome, message: text, taskId, awaitingConfirmation });
}

// ---------- Links on a page ----------
// No content scripts: the collector is injected once, on request, with the
// `activeTab` grant from the menu click or the popup, and only returns data.

async function runCollector(tabId: number, frameId: number | undefined, onlySelection: boolean): Promise<unknown> {
  const inject = async (target: chrome.scripting.InjectionTarget): Promise<unknown> => {
    const results = await ext.scripting.executeScript({
      target,
      func: collectPageLinks,
      args: [onlySelection, MAX_PAGE_LINKS],
    });
    return results[0]?.result;
  };
  // A cross-site frame is outside the `activeTab` grant; its page still is not.
  if (frameId) {
    const fromFrame = await inject({ tabId, frameIds: [frameId] }).catch(() => undefined);
    if (fromFrame !== undefined) return fromFrame;
  }
  return inject({ tabId }).catch(() => undefined);
}

async function openLinks(tab: chrome.tabs.Tab, frameId: number | undefined, onlySelection: boolean): Promise<void> {
  if (tab.id === undefined) return;
  const now = Date.now();
  const raw = await runCollector(tab.id, frameId, onlySelection);
  const collection: LinkCollection = collectionFromPage(raw, now) ?? unreadableCollection(tab.url, now);
  const id = crypto.randomUUID();
  const key = collectionKey(id);
  if (!key) return;
  try {
    // Session storage: in memory only, and readable by extension pages but
    // not by content scripts. The links page removes it once read.
    const stored = await ext.storage.session.get(null);
    const stale = staleCollectionKeys(stored, now);
    if (stale.length) await ext.storage.session.remove(stale);
    await ext.storage.session.set({ [key]: collection });
  } catch {
    // The page then says the links are no longer available.
  }
  const url = ext.runtime.getURL(`links.html?id=${id}`);
  // Next to the page it came from; anywhere if that window cannot take tabs.
  await ext.tabs
    .create({ url, index: tab.index + 1, openerTabId: tab.id, windowId: tab.windowId })
    .catch(() => ext.tabs.create({ url }))
    .catch(() => undefined);
}

/**
 * Hand a video page (YouTube) to Downpipe's site provider. Only the page's
 * address and title go over the bridge; yt-dlp in the app reads the page.
 */
async function sendVideoPage(
  url: string,
  title: string | undefined,
  source: Activity["source"],
  options: { quiet?: boolean } = {},
): Promise<LinkResult> {
  const page = sitePage(url);
  const message = page && pageMessage({ pageUrl: page.url, title }, newMessageId());
  if (!page || !message) {
    return { outcome: "failed", message: t("errors.notVideoPage"), stop: false };
  }
  const result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
  const outcome = pageOutcome(result);
  await report(
    {
      source,
      label: outcome.fileName ?? title ?? t("media.siteVideo", { site: page.site }),
      outcome: outcome.outcome,
      message: outcome.message,
      taskId: outcome.taskId,
      awaitingConfirmation: outcome.awaitingConfirmation,
    },
    options,
  );
  return { ...outcome, stop: bridgeDown(result) };
}

/**
 * The video page a tab or frame shows, if the site provider takes it: the
 * tab's own page first, else the frame's (a video embedded in another
 * site). The title only comes with the tab's page.
 */
async function videoPageOf(tabId: number, frameUrl?: string): Promise<{ url: string; item: PageItem } | null> {
  const tab = await ext.tabs.get(tabId).catch(() => undefined);
  const fromTab = sitePage(tab?.url);
  if (fromTab && tab?.url) {
    return { url: tab.url, item: { site: fromTab.site, title: pageTitle(cleanTitle(tab.title)) } };
  }
  const fromFrame = sitePage(frameUrl);
  return fromFrame && frameUrl ? { url: frameUrl, item: { site: fromFrame.site } } : null;
}

async function sendPageLink(url: string, pageUrl: string | undefined): Promise<LinkResult> {
  if (sitePage(url)) return sendVideoPage(url, undefined, "link", { quiet: true });
  const label = linkLabel(url);
  const fields = linkFields({ linkUrl: url, pageUrl });
  const message = fields && downloadMessage(fields, newMessageId());
  if (!message) {
    return { outcome: "failed", message: t("errors.code.invalid_url"), stop: false };
  }
  const result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
  const outcome = downloadOutcome(result);
  // Into the popup's list, but without a badge or notification per link:
  // the links page shows the results itself.
  await report(
    {
      source: "link",
      label: outcome.fileName ?? label,
      outcome: outcome.outcome,
      message: outcome.message,
      taskId: outcome.taskId,
      awaitingConfirmation: outcome.awaitingConfirmation,
    },
    { quiet: true },
  );
  return { ...outcome, stop: bridgeDown(result) };
}

// ---------- Capturing browser downloads ----------
// On only while the optional `downloads` permission is granted; the popup's
// toggle requests or removes it.

// Offered ids, chained like the settings so two events cannot both claim one.
let offeredWrites: Promise<unknown> = Promise.resolve();

function claimDownload(id: number): Promise<boolean> {
  const next = offeredWrites.then(async () => {
    const stored = await ext.storage.session.get(OFFERED_KEY);
    const list = claimIn(parseOffered(stored[OFFERED_KEY]), id);
    if (!list) return false;
    await ext.storage.session.set({ [OFFERED_KEY]: list });
    return true;
  });
  offeredWrites = next.catch(() => undefined);
  return next;
}

function handoffDeps(): HandoffDeps {
  return {
    downloads: ext.downloads,
    extensionId: ext.runtime.id,
    filters: settings,
    // The race with the browser's patience is lib/handoff.ts's; this only
    // bounds a bridge that never answers at all.
    send: (message) => callHost(sendNative, message, CAPTURE_TIMEOUT_MS + CAPTURE_LATE_MS),
    report: (entry) => report(entry),
    claim: claimDownload,
    answerMs: CAPTURE_TIMEOUT_MS,
    lateMs: CAPTURE_LATE_MS,
  };
}

// Chrome and Edge: the browser has the server's file name, MIME type and
// usually the size by now, and holds the download until every listener has
// called `suggest`.
function onDeterminingFilename(
  item: chrome.downloads.DownloadItem,
  suggest: (suggestion?: chrome.downloads.FilenameSuggestion) => void,
): boolean {
  void captureHeld(item, handoffDeps(), () => suggest());
  return true;
}

// Firefox has no onDeterminingFilename: pause, ask Downpipe, then cancel or resume.
function onCreated(item: chrome.downloads.DownloadItem): void {
  void captureStarted(item, handoffDeps());
}

function startCapture(): void {
  const downloads = ext.downloads;
  // Undefined until the permission is granted.
  if (!downloads) return;
  if (downloads.onDeterminingFilename) {
    if (!downloads.onDeterminingFilename.hasListener(onDeterminingFilename)) {
      downloads.onDeterminingFilename.addListener(onDeterminingFilename);
    }
  } else if (!downloads.onCreated.hasListener(onCreated)) {
    downloads.onCreated.addListener(onCreated);
  }
}

function stopCapture(): void {
  const downloads = ext.downloads;
  if (!downloads) return;
  downloads.onDeterminingFilename?.removeListener(onDeterminingFilename);
  downloads.onCreated.removeListener(onCreated);
}

startCapture();
ext.permissions.onAdded.addListener((p) => {
  if (p.permissions?.includes("downloads")) startCapture();
  void syncBypass();
});
ext.permissions.onRemoved.addListener((p) => {
  if (p.permissions?.includes("downloads")) stopCapture();
  void syncBypass();
});

// ---------- Letting the browser download a link itself ----------
// Alt+click (or the key set in the options) on a link: the browser downloads
// it once, as the extension's own download, which capture leaves alone. The
// content script that spots the click needs access to the sites (the same
// optional access media detection asks for), and runs only while capture is
// on, since otherwise the browser downloads everything itself anyway.

const BYPASS_SCRIPT_ID = "downpipe-bypass";

async function syncBypass(): Promise<void> {
  const [capture, sites, prefs] = await Promise.all([
    ext.permissions.contains(DOWNLOADS_PERMISSION).catch(() => false),
    ext.permissions.contains({ origins: WEB_ORIGINS }).catch(() => false),
    cachedSettings(),
  ]);
  const registered = await ext.scripting
    .getRegisteredContentScripts({ ids: [BYPASS_SCRIPT_ID] })
    .catch((): chrome.scripting.RegisteredContentScript[] => []);
  const wanted = capture && sites && prefs.bypassKey !== "off";
  if (!wanted) {
    if (registered.length) await ext.scripting.unregisterContentScripts({ ids: [BYPASS_SCRIPT_ID] }).catch(() => undefined);
    return;
  }
  if (registered.length) return;
  const script: chrome.scripting.RegisteredContentScript = {
    id: BYPASS_SCRIPT_ID,
    js: ["bypass.js"],
    matches: WEB_ORIGINS,
    allFrames: true,
    // Before the page's scripts, so its own click handlers come second.
    runAt: "document_start",
  };
  await ext.scripting
    .registerContentScripts([{ ...script, persistAcrossSessions: true }])
    .catch(() => ext.scripting.registerContentScripts([script]))
    .catch(() => undefined);
}

/** The browser downloads `url` itself; checked again here, since a page is around the script. */
async function bypassDownload(url: string): Promise<{ ok: boolean }> {
  const prefs = await settings();
  if (prefs.bypassKey === "off" || !ext.downloads) return { ok: false };
  const id = await ext.downloads.download({ url }).catch(() => undefined);
  return { ok: id !== undefined };
}

// ---------- Media detection ----------
// On only while the optional `webRequest` permission and access to web
// pages are granted; the popup's switch requests or removes both. The
// listeners only read response headers: nothing is blocked or changed, and
// nothing leaves the browser until the user picks a download. The one
// request of its own is reading each new HLS playlist once, without
// cookies, to tell a master from its quality playlists (lib/media.ts).

const MEDIA_PERMISSIONS: chrome.permissions.Permissions = { permissions: ["webRequest"], origins: WEB_ORIGINS };
const OVERLAY_SCRIPT_ID = "downpipe-overlay";
// Manifests come from XHR or fetch, files from media elements; Firefox files
// some player requests under "other".
const MEDIA_REQUEST_TYPES: `${chrome.webRequest.ResourceType}`[] = ["media", "xmlhttprequest", "other"];

/** `Referer` and `Origin` of recent requests, by request id, until their response starts. */
const sentHeaders = new Map<string, Pick<MediaCandidate, "referrer" | "origin">>();
/** Requests whose response never starts (blocked, failed) must not pile up. */
const MAX_SENT_HEADERS = 500;

// What the page's own request sent, so Downpipe can ask the server the same
// way. Only these two headers are kept; cookies and the rest never are.
function onMediaRequest(details: chrome.webRequest.OnSendHeadersDetails): void {
  if (details.tabId < 0) return;
  const headers = pageHeaders(details.requestHeaders);
  if (!headers.referrer && !headers.origin) return;
  sentHeaders.set(details.requestId, headers);
  if (sentHeaders.size > MAX_SENT_HEADERS) {
    const oldest = sentHeaders.keys().next().value;
    if (oldest !== undefined) sentHeaders.delete(oldest);
  }
}

/** HLS playlists being read or added, by tab and address. */
const playlistsInHand = new Set<string>();
/**
 * Playlist reads at once. A player fetches a master and several quality
 * playlists within a moment, a feed several of each; the rest wait their
 * turn, since a master read late still hides its parts.
 */
const MAX_PLAYLIST_READS = 4;
/** Beyond this many waiting, a playlist is listed unread. */
const MAX_PLAYLIST_QUEUE = 40;
let playlistReads = 0;
const playlistQueue: (() => void)[] = [];

async function inReadSlot<T>(run: () => Promise<T>): Promise<T | undefined> {
  if (playlistReads < MAX_PLAYLIST_READS) {
    playlistReads += 1;
  } else {
    if (playlistQueue.length >= MAX_PLAYLIST_QUEUE) return undefined;
    // The slot is handed over by the read that ends.
    await new Promise<void>((resolve) => playlistQueue.push(resolve));
  }
  try {
    return await run();
  } finally {
    const next = playlistQueue.shift();
    if (next) next();
    else playlistReads -= 1;
  }
}

/**
 * What a new HLS playlist is: undefined when it was not read (already
 * listed or known by its turn, which `addMedia` then leaves out, or too
 * many waiting), null when reading failed.
 */
function examinePlaylist(tabId: number, url: string): Promise<PlaylistInfo | null | undefined> {
  return inReadSlot(async () => {
    // Checked when its turn comes: the master may have named it meanwhile.
    if (!shouldReadPlaylist(await tabState(tabId), url)) return undefined;
    // The extension's own request has no tab, so the listeners here skip it.
    return readPlaylist((input, init) => fetch(input, init), playlistKey(url));
  });
}

function onMediaResponse(details: chrome.webRequest.OnResponseStartedDetails): void {
  const sent = sentHeaders.get(details.requestId);
  sentHeaders.delete(details.requestId);
  void cachedSettings()
    .then(async (prefs) => {
      const found = classifyResponse(details, prefs);
      if (!found) return;
      const frameUrl = frameOf(details as { documentUrl?: string; initiator?: string });
      // IDM's site exceptions: the excluded sites leave detection alone too.
      if (matchingSite([hostOf(found.url), hostOf(frameUrl), hostOf(sent?.referrer)], prefs.excludedSites)) return;
      const entry = { ...found, frameUrl, ...sent };
      if (found.kind !== "hls") {
        await changeTabMedia(details.tabId, (tab) => addMedia(tab, entry, crypto.randomUUID(), Date.now()));
        return;
      }
      // Players reload live playlists every few seconds; the first response
      // in hand decides, and the rest are repeats.
      const key = `${details.tabId} ${playlistKey(found.url)}`;
      if (playlistsInHand.has(key)) return;
      playlistsInHand.add(key);
      const page = tabPages.get(details.tabId);
      try {
        const playlist = await examinePlaylist(details.tabId, found.url);
        // The tab loaded another page while the playlist was read.
        if (tabPages.get(details.tabId) !== page) return;
        await changeTabMedia(details.tabId, (tab) => addMedia(tab, entry, crypto.randomUUID(), Date.now(), playlist));
      } finally {
        playlistsInHand.delete(key);
      }
    })
    .catch(() => undefined);
}

// A new document in the tab: what the old one played is gone.
function onPageRequest(details: chrome.webRequest.OnBeforeRequestDetails): undefined {
  if (details.tabId >= 0) void forgetTabMedia(details.tabId);
  // Observing only: the request goes ahead unchanged.
  return undefined;
}

function startMediaDetection(): void {
  const webRequest = ext.webRequest;
  // Undefined until the permission is granted. Registered synchronously at
  // the top level, so a response can wake the service worker.
  if (!webRequest) return;
  if (!webRequest.onSendHeaders.hasListener(onMediaRequest)) {
    const filter = { urls: WEB_ORIGINS, types: MEDIA_REQUEST_TYPES };
    try {
      // Chrome shows Referer and Origin to listeners only with `extraHeaders`.
      webRequest.onSendHeaders.addListener(onMediaRequest, filter, ["requestHeaders", "extraHeaders"]);
    } catch {
      // Firefox shows them without it and refuses the option.
      webRequest.onSendHeaders.addListener(onMediaRequest, filter, ["requestHeaders"]);
    }
  }
  if (!webRequest.onResponseStarted.hasListener(onMediaResponse)) {
    webRequest.onResponseStarted.addListener(
      onMediaResponse,
      { urls: WEB_ORIGINS, types: MEDIA_REQUEST_TYPES },
      ["responseHeaders"],
    );
  }
  if (!webRequest.onBeforeRequest.hasListener(onPageRequest)) {
    webRequest.onBeforeRequest.addListener(onPageRequest, { urls: WEB_ORIGINS, types: ["main_frame"] });
  }
}

function stopMediaDetection(): void {
  ext.webRequest?.onSendHeaders.removeListener(onMediaRequest);
  sentHeaders.clear();
  playlistsInHand.clear();
  ext.webRequest?.onResponseStarted.removeListener(onMediaResponse);
  ext.webRequest?.onBeforeRequest.removeListener(onPageRequest);
}

async function mediaEnabled(): Promise<boolean> {
  return ext.permissions.contains(MEDIA_PERMISSIONS).catch(() => false);
}

/**
 * The overlay button on playing videos runs only while detection is on and
 * the button is not switched off in the options, and never on excluded sites.
 */
async function syncOverlay(): Promise<void> {
  const [enabled, prefs] = await Promise.all([mediaEnabled(), cachedSettings()]);
  const registered = await ext.scripting
    .getRegisteredContentScripts({ ids: [OVERLAY_SCRIPT_ID] })
    .catch((): chrome.scripting.RegisteredContentScript[] => []);
  // The button's own list joins the excluded sites here only: detection and
  // capture go on there.
  const excludeMatches = excludeMatchesFor([...new Set([...prefs.excludedSites, ...prefs.overlaySites])]);
  const current = registered[0];
  if (!enabled || !prefs.mediaOverlay) {
    if (current) await ext.scripting.unregisterContentScripts({ ids: [OVERLAY_SCRIPT_ID] }).catch(() => undefined);
    return;
  }
  if (current && JSON.stringify(current.excludeMatches ?? []) === JSON.stringify(excludeMatches)) return;
  if (current) await ext.scripting.unregisterContentScripts({ ids: [OVERLAY_SCRIPT_ID] }).catch(() => undefined);
  const script: chrome.scripting.RegisteredContentScript = {
    id: OVERLAY_SCRIPT_ID,
    js: ["overlay.js"],
    matches: WEB_ORIGINS,
    allFrames: true,
    runAt: "document_idle",
    ...(excludeMatches.length ? { excludeMatches } : {}),
  };
  // Chrome keeps it across restarts with this; a browser that refuses the
  // field gets it registered again by this function at every start.
  await ext.scripting
    .registerContentScripts([{ ...script, persistAcrossSessions: true }])
    .catch(() => ext.scripting.registerContentScripts([script]))
    .catch(() => undefined);
}

// Like settings and reports, list changes are chained so two responses
// arriving together cannot overwrite each other's entry.
let mediaWrites: Promise<unknown> = Promise.resolve();

function changeTabMedia(tabId: number, change: (tab: TabMedia) => TabMedia | null): Promise<TabMedia> {
  const key = tabMediaKey(tabId);
  const partsKey = tabPartsKey(tabId);
  const next = mediaWrites.then(async () => {
    // Session storage: in memory only, cleared when the browser closes.
    const stored = await ext.storage.session.get([key, partsKey]).catch((): Record<string, unknown> => ({}));
    const tab: TabMedia = { items: parseTabMedia(stored[key]), parts: parseTabParts(stored[partsKey]) };
    const changed = change(tab);
    if (!changed) return tab;
    await ext.storage.session.set(
      changed.parts === tab.parts ? { [key]: changed.items } : { [key]: changed.items, [partsKey]: changed.parts },
    );
    return changed;
  });
  mediaWrites = next.catch(() => undefined);
  return next;
}

function tabState(tabId: number): Promise<TabMedia> {
  return changeTabMedia(tabId, () => null);
}

async function tabMedia(tabId: number): Promise<MediaCandidate[]> {
  return (await tabState(tabId)).items;
}

/** The tab's title, which names the rows and the downloads. */
async function tabTitle(tabId: number): Promise<string | undefined> {
  // Readable because detection holds access to every site.
  return cleanTitle((await ext.tabs.get(tabId).catch(() => undefined))?.title);
}

/** How often each tab's list was forgotten, so a playlist read for one page is not added to the next. */
const tabPages = new Map<number, number>();

function forgetTabMedia(tabId: number): Promise<void> {
  tabPages.set(tabId, (tabPages.get(tabId) ?? 0) + 1);
  const next = mediaWrites.then(() => ext.storage.session.remove([tabMediaKey(tabId), tabPartsKey(tabId)]));
  mediaWrites = next.catch(() => undefined);
  return next.catch(() => undefined);
}

async function forgetAllMedia(): Promise<void> {
  const stored = await ext.storage.session.get(null).catch((): Record<string, unknown> => ({}));
  const keys = Object.keys(stored).filter((k) => k.startsWith("media:"));
  if (keys.length) await ext.storage.session.remove(keys).catch(() => undefined);
}

/** Hand one detected stream or file to Downpipe, named after the tab's page. */
async function sendMedia(tabId: number, mediaId: string): Promise<LinkResult> {
  const item = (await tabMedia(tabId)).find((c) => c.id === mediaId);
  if (!item) {
    return { outcome: "failed", message: t("errors.mediaGone"), stop: false };
  }
  // Readable because detection holds access to every site.
  const tab = await ext.tabs.get(tabId).catch(() => undefined);
  const pageUrl = tab?.url;
  const title = cleanTitle(tab?.title);
  // What the player's own request sent; otherwise what the browser would
  // send from the frame that requested it (players often sit in a frame
  // of another site).
  const referrer = item.referrer ?? referrerFor(item.frameUrl ?? pageUrl, item.url);
  const id = newMessageId();
  let result;
  let outcome: HandOver;
  if (item.kind === "direct") {
    const fileName = titleFileName(title, item);
    const message = downloadMessage({ resourceUrl: item.url, pageUrl, referrer, mimeType: item.mime, fileName }, id);
    if (!message) return { outcome: "failed", message: t("errors.mediaInvalid"), stop: false };
    result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
    outcome = downloadOutcome(result);
  } else {
    const message = mediaMessage(
      { manifestUrl: item.url, kind: item.kind, pageUrl, referrer, title, origin: item.origin },
      id,
    );
    if (!message) return { outcome: "failed", message: t("errors.streamInvalid"), stop: false };
    result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
    outcome = streamOutcome(result);
  }
  await report({
    source: "media",
    label: outcome.fileName ?? title ?? mediaName(item.url),
    outcome: outcome.outcome,
    message: outcome.message,
    taskId: outcome.taskId,
    awaitingConfirmation: outcome.awaitingConfirmation,
  });
  return { ...outcome, stop: bridgeDown(result) };
}

function overlayItem(c: MediaCandidate, label: string): OverlayItem {
  const size = c.size !== undefined ? ` · ${formatBytes(c.size)}` : "";
  return { mediaId: c.id, label, detail: `${kindLabel(c)}${size}` };
}

function pageOverlayItem(page: PageItem): OverlayItem {
  return {
    mediaId: PAGE_ITEM_ID,
    label: page.title ?? t("media.thisSiteVideo", { site: page.site }),
    detail: t("media.siteVideoQuality", { site: page.site }),
  };
}

const OVERLAY_PLACES_KEY = "overlayPlaces";
/** Sites whose dragged button position is remembered; the oldest go first. */
const MAX_OVERLAY_PLACES = 100;

type Places = Record<string, { x: number; y: number }>;

async function overlayPlaces(): Promise<Places> {
  const stored = await ext.storage.local.get(OVERLAY_PLACES_KEY).catch((): Record<string, unknown> => ({}));
  const value: unknown = stored[OVERLAY_PLACES_KEY];
  if (typeof value !== "object" || value === null) return {};
  return Object.fromEntries(Object.entries(value).filter(([, place]) => isPlace(place))) as Places;
}

let placeWrites: Promise<unknown> = Promise.resolve();

function rememberPlace(host: string, place: { x: number; y: number }): Promise<void> {
  const next = placeWrites.then(async () => {
    // Insertion order is age order: the newest is written last.
    const kept = Object.entries(await overlayPlaces())
      .filter(([site]) => site !== host)
      .slice(-(MAX_OVERLAY_PLACES - 1));
    await ext.storage.local.set({ [OVERLAY_PLACES_KEY]: Object.fromEntries([...kept, [host, place]]) });
  });
  placeWrites = next.catch(() => undefined);
  return next;
}

/**
 * The site the button belongs to: the tab's, so a player embedded from
 * another site is hidden with the page it is on. The tab's address needs the
 * host permission, which detection has; the frame's is the fallback.
 */
function overlaySite(tabUrl: string | undefined, frameUrl: string | undefined): string | null {
  return hostOf(tabUrl) ?? hostOf(frameUrl);
}

async function overlayAnswer(
  request: OverlayRequest,
  tabId: number,
  frameUrl: string | undefined,
  tabUrl?: string,
): Promise<{ items: OverlayItem[] } | LinkResult | OverlayPrefs | { ok: boolean }> {
  switch (request.kind) {
    case "overlay_prefs": {
      const site = overlaySite(tabUrl, frameUrl);
      const [prefs, places] = await Promise.all([cachedSettings(), overlayPlaces()]);
      const place = site ? places[site] : undefined;
      return {
        hoverOnly: prefs.overlayHoverOnly,
        hidden: site !== null && matchingSite([site], prefs.overlaySites) !== undefined,
        ...(place ? { place } : {}),
      };
    }
    case "overlay_hide_site": {
      const site = overlaySite(tabUrl, frameUrl);
      const pattern = site ? normalizeSitePattern(site) : null;
      if (!pattern) return { ok: false };
      await changeSettings((s) => ({ ...s, overlaySites: withSite(s.overlaySites, pattern, true) }));
      return { ok: true };
    }
    case "overlay_place": {
      const site = overlaySite(tabUrl, frameUrl);
      if (!site) return { ok: false };
      await rememberPlace(site, { x: request.x, y: request.y });
      return { ok: true };
    }
    case "overlay_list": {
      // The element's own source, when it is a plain http(s) file or playlist.
      const src = request.src;
      const found = src ? classifySource(src) : null;
      const { items } =
        src && found && !isSiteMedia(src)
          ? await changeTabMedia(tabId, (tab) =>
              addMedia(tab, { ...found, url: src, frameUrl }, crypto.randomUUID(), Date.now()),
            )
          : await tabState(tabId);
      const page = await videoPageOf(tabId, frameUrl);
      // On a page with several videos (an X timeline), only this one's.
      const media = mediaForVideo(
        items.filter((c) => !isSiteMedia(c.url)),
        request.poster,
      );
      const labels = mediaLabels(media, await tabTitle(tabId));
      const listed = media.map((c, i) => overlayItem(c, labels[i] ?? mediaName(c.url)));
      return { items: page ? [pageOverlayItem(page.item), ...listed] : listed };
    }
    case "overlay_send":
      return sendMedia(tabId, request.mediaId);
    case "overlay_page": {
      const page = await videoPageOf(tabId, frameUrl);
      if (!page) return { outcome: "failed", message: t("errors.unknownVideoPage"), stop: false };
      return sendVideoPage(page.url, page.item.title, "media");
    }
  }
}

startMediaDetection();
void syncOverlay();
ext.permissions.onAdded.addListener((p) => {
  if (p.permissions?.includes("webRequest") || p.origins?.length) {
    startMediaDetection();
    void syncOverlay();
  }
});
ext.permissions.onRemoved.addListener((p) => {
  if (!p.permissions?.includes("webRequest") && !p.origins?.length) return;
  void mediaEnabled().then((on) => {
    if (on) return;
    stopMediaDetection();
    void syncOverlay();
    void forgetAllMedia();
  });
});
ext.tabs.onRemoved.addListener((tabId) => void forgetTabMedia(tabId));

// ---------- Settings ----------

// Media detection consults the settings for every response it classifies;
// they are read once and kept until they change.
let settingsCache: Promise<Settings> | null = null;

function cachedSettings(): Promise<Settings> {
  settingsCache ??= settings();
  return settingsCache;
}

ext.storage.onChanged.addListener((changes, area) => {
  if (area !== "local" || !changes[SETTINGS_KEY]) return;
  settingsCache = null;
  // The overlay follows its switch and the excluded sites, the click script its key.
  void syncOverlay();
  void syncBypass();
});
// After the settings cache above exists, which it reads.
void syncBypass();

async function settings(): Promise<Settings> {
  const stored = await ext.storage.local.get(SETTINGS_KEY).catch((): Record<string, unknown> => ({}));
  return parseSettings(stored[SETTINGS_KEY]);
}

// Changes are chained for the same reason as reports below: the popup and
// the options page could otherwise both read the old settings and the
// second write would undo the first.
let settingsWrites: Promise<unknown> = Promise.resolve();

function changeSettings(change: (current: Settings) => Settings): Promise<Settings> {
  const next = settingsWrites.then(async () => {
    const updated = change(await settings());
    await ext.storage.local.set({ [SETTINGS_KEY]: updated });
    return updated;
  });
  settingsWrites = next.catch(() => undefined);
  return next;
}

// ---------- Activity, badge and notifications ----------

async function storedActivity(): Promise<Activity[]> {
  const stored = await ext.storage.session.get(ACTIVITY_KEY).catch((): Record<string, unknown> => ({}));
  return parseActivity(stored[ACTIVITY_KEY]);
}

// Reports are chained: two quick clicks would otherwise both read the old
// list and the second write would drop the first entry.
let reports: Promise<void> = Promise.resolve();
let badgeTimer: ReturnType<typeof setTimeout> | undefined;

function report(entry: Omit<Activity, "at">, options: { quiet?: boolean } = {}): Promise<void> {
  const activity: Activity = { ...entry, at: Date.now() };
  reports = reports.then(() => record(activity, options.quiet === true)).catch(() => undefined);
  return reports;
}

async function record(activity: Activity, quiet: boolean): Promise<void> {
  try {
    // Session storage: cleared when the browser closes, never written to disk.
    const stored = await ext.storage.session.get(ACTIVITY_KEY);
    await ext.storage.session.set({
      [ACTIVITY_KEY]: addActivity(parseActivity(stored[ACTIVITY_KEY]), activity),
    });
  } catch {
    // The badge below still tells the user what happened.
  }
  if (quiet) return;
  const ok = activity.outcome === "added" || activity.outcome === "duplicate";
  await ext.action.setBadgeBackgroundColor({ color: ok ? BADGE_OK : BADGE_FAIL });
  await ext.action.setBadgeText({ text: ok ? "+" : "!" });
  await ext.action.setTitle({ title: t("notifications.toolbarTitle", { label: activity.label, message: activity.message }) });
  // One timer for the latest report, so an older one cannot clear a newer badge.
  clearTimeout(badgeTimer);
  badgeTimer = setTimeout(
    () => {
      void ext.action.setBadgeText({ text: "" });
      void ext.action.setTitle({ title: "Downpipe" });
    },
    ok ? BADGE_MS.ok : BADGE_MS.fail,
  );
  if ((await settings()).notify) await notify(activity);
}

async function notify(activity: Activity): Promise<void> {
  const text = notificationFor(activity);
  const id = `downpipe-${activity.at}`;
  try {
    await ext.notifications.create(id, {
      type: "basic",
      iconUrl: ext.runtime.getURL("icons/icon-128.png"),
      title: text.title,
      message: text.message,
    });
  } catch {
    // Notifications blocked by the system: the badge and popup still say it.
    return;
  }
  if (!text.sticky) setTimeout(() => void ext.notifications.clear(id), NOTIFICATION_MS);
}

// ---------- Extension pages ----------

/** Live state of the downloads in the popup's list; null if the app could not be asked. */
async function liveTasks(activity: readonly Activity[]): Promise<TaskStatus[] | null> {
  const message = tasksMessage(taskIdsOf(activity), newMessageId());
  if (!message) return [];
  const result = await callHost(sendNative, message, STATUS_TIMEOUT_MS);
  return result.ok && result.response.type === "tasks" ? result.response.tasks : null;
}

async function status(): Promise<StatusReply> {
  const activity = await storedActivity();
  // The ping and the progress query each start a host process; run them side by side.
  const [result, capture, prefs, tasks] = await Promise.all([
    callHost(sendNative, pingMessage(newMessageId()), STATUS_TIMEOUT_MS),
    ext.permissions.contains(DOWNLOADS_PERMISSION),
    settings(),
    liveTasks(activity),
  ]);
  return {
    bridge: bridgeStatus(result),
    capture,
    notify: prefs.notify,
    excludedSites: prefs.excludedSites,
    activity,
    tasks: tasks ?? [],
  };
}

async function collectFromPopup(tabId: number): Promise<{ opened: boolean }> {
  const tab = await ext.tabs.get(tabId).catch(() => undefined);
  if (!tab) return { opened: false };
  await openLinks(tab, undefined, false);
  return { opened: true };
}

type Reply = StatusReply | TasksReply | PingReply | Settings | LinkResult | MediaReply | { opened: boolean };

async function answer(request: PopupRequest): Promise<Reply> {
  switch (request.kind) {
    case "status":
      return status();
    case "tasks":
      return { tasks: await liveTasks(await storedActivity()) };
    case "ping":
      return { bridge: bridgeStatus(await callHost(sendNative, pingMessage(newMessageId()), STATUS_TIMEOUT_MS)) };
    case "get_settings":
      return settings();
    case "set_notify":
      return changeSettings((s) => ({ ...s, notify: request.enabled }));
    case "save_filters":
      return changeSettings((s) => ({ ...s, ...request.filters }));
    case "set_site_paused":
      return changeSettings((s) => ({ ...s, excludedSites: withSite(s.excludedSites, request.site, request.paused) }));
    case "collect_links":
      return collectFromPopup(request.tabId);
    case "send_link":
      return sendPageLink(request.url, request.pageUrl);
    case "media_list": {
      // A video page is offered whether detection is on or not: opening the
      // popup grants `activeTab`, which is enough to read the tab's address.
      const page = (await videoPageOf(request.tabId))?.item;
      const withPage = (reply: MediaReply): MediaReply => (page ? { ...reply, page } : reply);
      const enabled = await mediaEnabled();
      if (!enabled) return withPage({ enabled, items: [] });
      const tab = await ext.tabs.get(request.tabId).catch(() => undefined);
      const excludedBy = matchingSite([hostOf(tab?.url)], (await cachedSettings()).excludedSites);
      if (excludedBy) return withPage({ enabled, items: [], excludedBy });
      const items = (await tabMedia(request.tabId)).filter((c) => !isSiteMedia(c.url));
      return withPage({ enabled, items, title: cleanTitle(tab?.title) });
    }
    case "save_media":
      return changeSettings((s) => ({ ...s, ...request.media }));
    case "set_bypass_key":
      return changeSettings((s) => ({ ...s, bypassKey: request.key }));
    case "send_media":
      return sendMedia(request.tabId, request.mediaId);
    case "send_page": {
      const page = await videoPageOf(request.tabId);
      if (!page) return { outcome: "failed", message: t("errors.unknownVideoPage"), stop: false };
      return sendVideoPage(page.url, page.item.title, "media");
    }
  }
}

ext.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // This extension's own pages (popup, options, links page) may ask
  // anything above. It does not listen to other extensions (no
  // onMessageExternal), and the injected link collector only returns a
  // value and never sends messages.
  if (isTrustedSender(sender, ext.runtime.id, ext.runtime.getURL(""))) {
    const request = parsePopupRequest(message);
    if (!request) return false;
    answer(request).then(sendResponse, () => sendResponse(null));
    return true;
  }
  // Content scripts inside web pages. The click script asks for one browser
  // download of a link; the overlay button has three requests, always about
  // the sender's own tab, which the browser reports and the page cannot choose.
  if (isOwnContentScript(sender, ext.runtime.id)) {
    const bypass = parseBypassRequest(message);
    if (bypass) {
      bypassDownload(bypass.url).then(sendResponse, () => sendResponse({ ok: false }));
      return true;
    }
    const request = parseOverlayRequest(message);
    if (!request) return false;
    overlayAnswer(request, sender.tab.id, frameOf({ documentUrl: sender.url }), sender.tab.url).then(sendResponse, () =>
      sendResponse(null),
    );
    return true;
  }
  return false;
});
