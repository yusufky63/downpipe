import { t } from "../i18n";
import { ext } from "../lib/browser";
import { hostOf, matchingSite } from "../lib/filters";
import { WEB_ORIGINS, mediaLabels, type MediaCandidate } from "../lib/media";
import {
  isMediaReply,
  isSettings,
  overlayResultText,
  relativeTime,
  type Activity,
  type PageItem,
  type StatusReply,
  type TasksReply,
} from "../lib/messages";
import type { TaskStatus } from "../lib/protocol";
import { itemView, mayChange, mediaEmptyText, mediaRowView, pageRowView, statusView } from "../lib/view";
import { byId, extensionInfo, paintStatus, span, startPage } from "../ui/dom";

startPage();

const DOWNLOADS_PERMISSION: chrome.permissions.Permissions = { permissions: ["downloads"] };
const MEDIA_PERMISSIONS: chrome.permissions.Permissions = { permissions: ["webRequest"], origins: WEB_ORIGINS };
/** While detection is on, the page's media list is read again this often. */
const MEDIA_POLL_MS = 1_500;
/** While a listed download runs or waits for an answer, it is refreshed this often. */
const POLL_MS = 1_000;

const els = {
  version: byId("version"),
  status: {
    box: byId("status"),
    title: byId("status-title"),
    detail: byId("status-detail"),
    command: byId("status-command"),
    guide: byId<HTMLAnchorElement>("guide"),
  },
  recheck: byId<HTMLButtonElement>("recheck"),
  statusBlock: byId("status-block"),
  connected: byId("connected"),
  actions: byId("status-actions"),
  capture: byId<HTMLInputElement>("capture"),
  filters: byId<HTMLButtonElement>("filters"),
  siteAction: byId<HTMLButtonElement>("site-action"),
  siteNote: byId("site-note"),
  notify: byId<HTMLInputElement>("notify"),
  pageLinks: byId<HTMLButtonElement>("page-links"),
  activity: byId<HTMLUListElement>("activity"),
  empty: byId("activity-empty"),
  media: {
    block: byId("media-block"),
    toggle: byId<HTMLInputElement>("media"),
    list: byId<HTMLUListElement>("media-list"),
    empty: byId("media-empty"),
  },
};

const extension = extensionInfo();

/** What the list shows: the sent downloads and their last known state. */
let listed: Activity[] = [];
let live = new Map<string, TaskStatus>();
let pollTimer: ReturnType<typeof setTimeout> | undefined;
/** The tab the popup was opened over, when it is an http(s) page. */
let page: { tabId: number; host: string } | null = null;
let excludedSites: string[] = [];
/** The connected Downpipe takes streams; assumed until the status says otherwise. */
let streams = true;
/** The connected Downpipe takes video pages (YouTube); assumed likewise. */
let pages = true;
let lists = true;
let mediaItems: MediaCandidate[] = [];
/** The tab's title, which names the media rows. */
let mediaTitle: string | undefined;
/** The tab is a video page the site provider can download. */
let mediaPage: PageItem | undefined;
let mediaEnabled = false;
let mediaExcludedBy: string | undefined;
let mediaTimer: ReturnType<typeof setTimeout> | undefined;
/** What happened when a row was sent, by media id, while the popup is open. */
const mediaResults = new Map<string, { text: string; failed: boolean } | "sending">();
/** What the media list was last drawn from, to leave it alone (and focus in it) when nothing changed. */
let mediaDrawn = "";

function isStatusReply(value: unknown): value is StatusReply {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.bridge === "object" &&
    typeof v.capture === "boolean" &&
    typeof v.notify === "boolean" &&
    typeof v.startApp === "boolean" &&
    Array.isArray(v.excludedSites) &&
    v.excludedSites.every((s) => typeof s === "string") &&
    Array.isArray(v.activity) &&
    Array.isArray(v.tasks)
  );
}

function isTasksReply(value: unknown): value is TasksReply {
  if (typeof value !== "object" || value === null) return false;
  const tasks = (value as Record<string, unknown>).tasks;
  return tasks === null || Array.isArray(tasks);
}

function renderStatus(reply: StatusReply | null): void {
  const view = statusView(reply?.bridge ?? null, extension, reply?.startApp ?? false);
  paintStatus(els.status, view);
  // Connected needs no box and no actions, only a word in the header: the
  // room goes to the media and download lists. The popup checks again each
  // time it opens.
  const connected = view.tone === "positive";
  els.statusBlock.hidden = connected;
  els.connected.hidden = !connected;
  els.connected.title = connected ? t("popup.connectedTitle", { title: view.title, detail: view.detail }) : "";
  els.actions.hidden = connected;
}

/**
 * "Pause capture on <host>" while capture is on. An exact entry can be
 * resumed here; a wildcard that covers the site is only named, since
 * removing it would affect other sites too.
 */
function renderSite(): void {
  const host = els.capture.checked ? page?.host : undefined;
  const by = host ? matchingSite([host], excludedSites) : undefined;
  els.siteAction.hidden = !host || (by !== undefined && by !== host);
  els.siteNote.hidden = !host || by === undefined || by === host;
  if (!host) return;
  const paused = by === host;
  // "this site" keeps the row on one line; the host is in the tooltip.
  els.siteAction.textContent = t(paused ? "popup.resumeSite" : "popup.pauseSite");
  els.siteAction.title = t(paused ? "popup.resumeSiteTitle" : "popup.pauseSiteTitle", { host });
  els.siteAction.dataset.paused = String(paused);
  els.siteNote.textContent = by ? t("popup.pausedBy", { site: by }) : "";
  els.siteNote.title = els.siteNote.textContent;
}

function activityItem(entry: Activity, now: number): HTMLLIElement {
  const view = itemView(entry, entry.taskId ? live.get(entry.taskId) : undefined);
  const li = document.createElement("li");
  li.className = "item";
  li.dataset.tone = view.tone;
  const label = span("item-label", view.label);
  label.title = view.label;
  const detail = span("item-detail num", view.detail);
  detail.title = view.technical ? `${view.detail}\n${view.technical}` : view.detail;
  li.append(label, span("badge", view.badge), detail);
  // A running download shows live numbers across the row; a finished one
  // also says when it was sent.
  if (view.progress === null) li.append(span("item-time num", relativeTime(entry.at, now)));
  else detail.classList.add("item-detail-wide");
  if (view.progress !== null) {
    const bar = document.createElement("div");
    bar.className = "bar";
    bar.setAttribute("role", "progressbar");
    bar.setAttribute("aria-label", t("popup.progress", { label: view.label }));
    bar.setAttribute("aria-valuemin", "0");
    bar.setAttribute("aria-valuemax", "100");
    bar.setAttribute("aria-valuenow", String(Math.floor(view.progress)));
    const fill = document.createElement("div");
    fill.className = "bar-fill";
    fill.style.width = `${view.progress}%`;
    bar.append(fill);
    li.append(bar);
  }
  return li;
}

function mediaRow(c: MediaCandidate, name: string | undefined): HTMLLIElement {
  const view = mediaRowView(c, streams, name);
  const result = mediaResults.get(c.id);
  const li = document.createElement("li");
  li.className = "item media-item";
  li.dataset.tone = result && result !== "sending" ? (result.failed ? "danger" : "positive") : "muted";
  const label = span("item-label", view.label);
  label.title = c.url;
  const detail = span("item-detail num", result && result !== "sending" ? result.text : view.detail);
  detail.title = detail.textContent ?? "";
  const send = document.createElement("button");
  send.type = "button";
  send.className = "btn btn-small";
  send.textContent = t(result === "sending" ? "common.sending" : "common.download");
  send.disabled = !view.sendable || result === "sending";
  send.setAttribute("aria-label", t("popup.downloadItem", { label: view.label }));
  send.addEventListener("click", () => void sendMedia(c.id));
  li.append(label, send, detail);
  return li;
}

/** Results of sending the page row, in `mediaResults` next to media ids. */
const PAGE_RESULT = "page";

/** The video page itself (YouTube), first in the list. */
function pageRow(page: PageItem): HTMLLIElement {
  const view = pageRowView(page, pages, lists);
  const result = mediaResults.get(PAGE_RESULT);
  const li = document.createElement("li");
  li.className = "item media-item";
  li.dataset.tone = result && result !== "sending" ? (result.failed ? "danger" : "positive") : "muted";
  const label = span("item-label", view.label);
  label.title = view.label;
  const detail = span("item-detail num", result && result !== "sending" ? result.text : view.detail);
  detail.title = detail.textContent ?? "";
  const send = document.createElement("button");
  send.type = "button";
  send.className = "btn btn-small";
  send.textContent = t(result === "sending" ? "common.sending" : "common.download");
  send.disabled = !view.sendable || result === "sending";
  send.setAttribute("aria-label", t("popup.downloadItem", { label: view.label }));
  send.addEventListener("click", () => void sendPage());
  li.append(label, send, detail);
  return li;
}

function renderMedia(): void {
  const drawn = JSON.stringify([
    mediaEnabled,
    mediaExcludedBy,
    streams,
    pages,
    lists,
    mediaPage,
    mediaTitle,
    mediaItems.map((c) => [c.id, c.qualities]),
    [...mediaResults],
  ]);
  els.media.toggle.checked = mediaEnabled;
  if (drawn === mediaDrawn) return;
  mediaDrawn = drawn;
  const names = mediaLabels(mediaItems, mediaTitle);
  const rows = mediaItems.map((c, i) => mediaRow(c, names[i]));
  els.media.list.replaceChildren(...(mediaPage ? [pageRow(mediaPage), ...rows] : rows));
  els.media.empty.textContent = mediaEmptyText(mediaEnabled, mediaExcludedBy);
  // On a video page the page row is the download; "nothing yet" would be wrong.
  els.media.empty.hidden = mediaItems.length > 0 || mediaPage !== undefined;
}

async function refreshMedia(): Promise<void> {
  clearTimeout(mediaTimer);
  if (!page) return;
  const reply: unknown = await ext.runtime.sendMessage({ kind: "media_list", tabId: page.tabId }).catch(() => null);
  if (!isMediaReply(reply)) return;
  mediaEnabled = reply.enabled;
  mediaExcludedBy = reply.excludedBy;
  mediaItems = reply.items;
  mediaTitle = reply.title;
  mediaPage = reply.page;
  renderMedia();
  if (mediaEnabled) mediaTimer = setTimeout(() => void refreshMedia(), MEDIA_POLL_MS);
}

async function sendMedia(mediaId: string): Promise<void> {
  if (!page) return;
  mediaResults.set(mediaId, "sending");
  renderMedia();
  const reply: unknown = await ext.runtime
    .sendMessage({ kind: "send_media", tabId: page.tabId, mediaId })
    .catch(() => null);
  mediaResults.set(mediaId, overlayResultText(reply));
  renderMedia();
  // The Downloads list below now has it.
  void refresh();
}

async function sendPage(): Promise<void> {
  if (!page) return;
  mediaResults.set(PAGE_RESULT, "sending");
  renderMedia();
  const reply: unknown = await ext.runtime.sendMessage({ kind: "send_page", tabId: page.tabId }).catch(() => null);
  mediaResults.set(PAGE_RESULT, overlayResultText(reply));
  renderMedia();
  void refresh();
}

function renderList(): void {
  const now = Date.now();
  els.activity.replaceChildren(...listed.map((entry) => activityItem(entry, now)));
  els.empty.hidden = listed.length > 0;
}

function applyTasks(tasks: TaskStatus[]): void {
  live = new Map(tasks.map((t) => [t.taskId, t]));
  renderList();
  // Keep refreshing only while something the popup shows can still change.
  clearTimeout(pollTimer);
  if (tasks.some(mayChange)) pollTimer = setTimeout(() => void poll(), POLL_MS);
}

async function poll(): Promise<void> {
  const reply: unknown = await ext.runtime.sendMessage({ kind: "tasks" }).catch(() => null);
  if (isTasksReply(reply) && reply.tasks) applyTasks(reply.tasks);
}

async function refresh(): Promise<void> {
  els.recheck.disabled = true;
  try {
    const reply: unknown = await ext.runtime.sendMessage({ kind: "status" });
    const status = isStatusReply(reply) ? reply : null;
    if (status) {
      streams = status.bridge.state !== "connected" || status.bridge.media;
      pages = status.bridge.state !== "connected" || status.bridge.pages;
      lists = status.bridge.state !== "connected" || status.bridge.lists;
      renderMedia();
    }
    renderStatus(
      status ?? {
        bridge: { state: "error", message: t("common.extensionNoAnswer") },
        capture: false,
        notify: true,
        startApp: false,
        excludedSites: [],
        activity: [],
        tasks: [],
      },
    );
    if (status) {
      els.capture.checked = status.capture;
      els.notify.checked = status.notify;
      excludedSites = status.excludedSites;
      renderSite();
      listed = status.activity;
      applyTasks(status.tasks);
    }
  } finally {
    els.recheck.disabled = false;
  }
}

/** Opening the popup grants `activeTab`, which is what makes the tab's URL readable here. */
async function findPage(): Promise<void> {
  const [tab] = await ext.tabs.query({ active: true, currentWindow: true }).catch((): chrome.tabs.Tab[] => []);
  const host = hostOf(tab?.url);
  page = tab?.id !== undefined && host ? { tabId: tab.id, host } : null;
  els.pageLinks.hidden = page === null;
  els.media.block.hidden = page === null;
  renderSite();
  await refreshMedia();
}

// Detection is the optional `webRequest` permission plus access to web pages;
// the request has to come straight from this click.
els.media.toggle.addEventListener("change", () => {
  const wanted = els.media.toggle.checked;
  els.media.toggle.disabled = true;
  const change = wanted ? ext.permissions.request(MEDIA_PERMISSIONS) : ext.permissions.remove(MEDIA_PERMISSIONS);
  void change
    .catch(() => false)
    .then(() => refreshMedia())
    .finally(() => {
      els.media.toggle.disabled = false;
    });
});

// The toggle is the optional `downloads` permission itself: granted means
// capture is on. The request has to come straight from this click.
els.capture.addEventListener("change", () => {
  const wanted = els.capture.checked;
  els.capture.disabled = true;
  const change = wanted ? ext.permissions.request(DOWNLOADS_PERMISSION) : ext.permissions.remove(DOWNLOADS_PERMISSION);
  void change
    .catch(() => false)
    .then(() => ext.permissions.contains(DOWNLOADS_PERMISSION))
    .then((granted) => {
      els.capture.checked = granted;
      renderSite();
    })
    .finally(() => {
      els.capture.disabled = false;
    });
});

els.notify.addEventListener("change", () => {
  els.notify.disabled = true;
  void ext.runtime
    .sendMessage({ kind: "set_notify", enabled: els.notify.checked })
    .catch(() => undefined)
    .finally(() => {
      els.notify.disabled = false;
    });
});

els.siteAction.addEventListener("click", () => {
  if (!page) return;
  const paused = els.siteAction.dataset.paused !== "true";
  els.siteAction.disabled = true;
  void ext.runtime
    .sendMessage({ kind: "set_site_paused", site: page.host, paused })
    .catch(() => null)
    .then((reply: unknown) => {
      if (isSettings(reply)) excludedSites = reply.excludedSites;
      renderSite();
    })
    .finally(() => {
      els.siteAction.disabled = false;
    });
});

els.filters.addEventListener("click", () => {
  void ext.runtime
    .openOptionsPage()
    .catch(() => undefined)
    .then(() => window.close());
});

els.pageLinks.addEventListener("click", () => {
  if (!page) return;
  els.pageLinks.disabled = true;
  // The service worker reads the page and opens the links page; the popup
  // closes when that tab takes the focus.
  void ext.runtime
    .sendMessage({ kind: "collect_links", tabId: page.tabId })
    .catch(() => undefined)
    .then(() => window.close());
});

els.recheck.addEventListener("click", () => {
  renderStatus(null);
  void refresh();
});

els.version.textContent = `v${ext.runtime.getManifest().version}`;
void findPage();
void refresh();
