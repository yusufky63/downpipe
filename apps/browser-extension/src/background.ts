// Service worker (Chrome, Edge) or event page (Firefox). Every listener is
// registered synchronously at the top level, so the browser can wake the
// worker for any of them.
import { ext } from "./lib/browser";
import { decideCapture, linkFields, type DownloadItemLike } from "./lib/capture";
import { collectPageLinks } from "./lib/collect";
import { withSite } from "./lib/filters";
import { callHost } from "./lib/host";
import {
  MAX_PAGE_LINKS,
  collectionFromPage,
  collectionKey,
  staleCollectionKeys,
  unreadableCollection,
  type LinkCollection,
} from "./lib/links";
import {
  addActivity,
  parseActivity,
  parsePopupRequest,
  parseSettings,
  taskIdsOf,
  type Activity,
  type LinkResult,
  type PingReply,
  type PopupRequest,
  type Settings,
  type StatusReply,
  type TasksReply,
} from "./lib/messages";
import { notificationFor } from "./lib/notify";
import {
  downloadMessage,
  fileNameHint,
  newMessageId,
  pingMessage,
  tasksMessage,
  type TaskStatus,
} from "./lib/protocol";
import { isTrustedSender } from "./lib/sender";
import { bridgeDown, bridgeStatus, downloadOutcome, keptMessage } from "./lib/status";
import { linkLabel } from "./lib/url";

const MENU_ID = "downpipe-download-link";
const PAGE_MENU_ID = "downpipe-page-links";
const SELECTION_MENU_ID = "downpipe-selected-links";
const WEB_PAGES = ["http://*/*", "https://*/*"];
const LINK_TIMEOUT_MS = 10_000;
/** A captured download waits for this answer, so it is kept short. */
const CAPTURE_TIMEOUT_MS = 5_000;
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

// ---------- Context menus ----------

function createMenu(): void {
  void ext.contextMenus.removeAll().then(() => {
    ext.contextMenus.create({
      id: MENU_ID,
      title: "Download with Downpipe",
      contexts: ["link"],
      targetUrlPatterns: WEB_PAGES,
    });
    // `page` only shows where no more specific context applies, so this
    // does not crowd the link menu above.
    ext.contextMenus.create({
      id: PAGE_MENU_ID,
      title: "Download links on this page with Downpipe…",
      contexts: ["page", "frame"],
      documentUrlPatterns: WEB_PAGES,
    });
    ext.contextMenus.create({
      id: SELECTION_MENU_ID,
      title: "Download selected links with Downpipe…",
      contexts: ["selection"],
      documentUrlPatterns: WEB_PAGES,
    });
  });
}

ext.runtime.onInstalled.addListener(createMenu);
ext.runtime.onStartup.addListener(createMenu);

ext.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === MENU_ID) {
    void sendLink(info);
    return;
  }
  // Choosing the item grants `activeTab` for this tab, which is what lets
  // the collector run there.
  if ((info.menuItemId === PAGE_MENU_ID || info.menuItemId === SELECTION_MENU_ID) && tab) {
    void openLinks(tab, info.frameId, info.menuItemId === SELECTION_MENU_ID);
  }
});

async function sendLink(info: chrome.contextMenus.OnClickData): Promise<void> {
  const label = linkLabel(info.linkUrl ?? "");
  const fields = linkFields(info);
  const message = fields && downloadMessage(fields, newMessageId());
  if (!message) {
    await report({
      source: "link",
      label,
      outcome: "failed",
      message: "This link cannot be downloaded. Only http and https links work.",
    });
    return;
  }
  const result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
  const { outcome, message: text, fileName, taskId } = downloadOutcome(result);
  await report({ source: "link", label: fileName ?? label, outcome, message: text, taskId });
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

async function sendPageLink(url: string, pageUrl: string | undefined): Promise<LinkResult> {
  const label = linkLabel(url);
  const fields = linkFields({ linkUrl: url, pageUrl });
  const message = fields && downloadMessage(fields, newMessageId());
  if (!message) {
    return { outcome: "failed", message: "This link cannot be downloaded. Only http and https links work.", stop: false };
  }
  const result = await callHost(sendNative, message, LINK_TIMEOUT_MS);
  const outcome = downloadOutcome(result);
  // Into the popup's list, but without a badge or notification per link:
  // the links page shows the results itself.
  await report(
    { source: "link", label: outcome.fileName ?? label, outcome: outcome.outcome, message: outcome.message, taskId: outcome.taskId },
    { quiet: true },
  );
  return { ...outcome, stop: bridgeDown(result) };
}

// ---------- Capturing browser downloads ----------
// On only while the optional `downloads` permission is granted; the popup's
// toggle requests or removes it.

/** True when Downpipe took the download and the browser should drop it. */
async function handOff(item: DownloadItemLike): Promise<boolean> {
  const decision = decideCapture(item, ext.runtime.id, await settings());
  if (!decision.capture) return false;
  const message = downloadMessage(decision.fields, newMessageId());
  if (!message) return false;
  const label = fileNameHint(item.filename) ?? linkLabel(item.url);
  const result = await callHost(sendNative, message, CAPTURE_TIMEOUT_MS);
  const { outcome, message: text, fileName, taskId } = downloadOutcome(result);
  if (outcome === "failed") {
    await report({ source: "capture", label, outcome: "kept", message: keptMessage(result) });
    return false;
  }
  await report({ source: "capture", label: fileName ?? label, outcome, message: text, taskId });
  return true;
}

async function removeFromBrowser(id: number): Promise<void> {
  await ext.downloads.cancel(id).catch(() => undefined);
  await ext.downloads.erase({ id }).catch(() => undefined);
}

// Chrome and Edge: the browser has the server's file name, MIME type and
// usually the size by now, and holds the download until every listener has
// called `suggest`.
function onDeterminingFilename(
  item: chrome.downloads.DownloadItem,
  suggest: (suggestion?: chrome.downloads.FilenameSuggestion) => void,
): boolean {
  void handOff(item)
    .catch(() => false)
    .then(async (handed) => {
      if (handed) await removeFromBrowser(item.id);
    })
    .finally(() => {
      try {
        suggest();
      } catch {
        // Already canceled.
      }
    });
  return true;
}

// Firefox has no onDeterminingFilename: pause, ask Downpipe, then cancel or resume.
function onCreated(item: chrome.downloads.DownloadItem): void {
  void (async () => {
    // Filters first, so downloads they leave to the browser are never paused.
    if (!decideCapture(item, ext.runtime.id, await settings()).capture) return;
    await ext.downloads.pause(item.id).catch(() => undefined);
    const handed = await handOff(item).catch(() => false);
    if (handed) await removeFromBrowser(item.id);
    else await ext.downloads.resume(item.id).catch(() => undefined);
  })();
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
});
ext.permissions.onRemoved.addListener((p) => {
  if (p.permissions?.includes("downloads")) stopCapture();
});

// ---------- Settings ----------

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
  await ext.action.setTitle({ title: `Downpipe: ${activity.label}. ${activity.message}` });
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

type Reply = StatusReply | TasksReply | PingReply | Settings | LinkResult | { opened: boolean };

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
  }
}

ext.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Only this extension's own pages (popup, options, links page); it has no
  // content scripts and does not listen to other extensions (no
  // onMessageExternal). The injected link collector only returns a value
  // and never sends messages.
  if (!isTrustedSender(sender, ext.runtime.id, ext.runtime.getURL(""))) return false;
  const request = parsePopupRequest(message);
  if (!request) return false;
  answer(request).then(sendResponse, () => sendResponse(null));
  return true;
});
