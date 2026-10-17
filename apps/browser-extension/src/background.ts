// Service worker (Chrome, Edge) or event page (Firefox). Every listener is
// registered synchronously at the top level, so the browser can wake the
// worker for any of them.
import { ext } from "./lib/browser";
import { decideCapture, linkFields, type DownloadItemLike } from "./lib/capture";
import { callHost } from "./lib/host";
import {
  addActivity,
  parseActivity,
  parsePopupRequest,
  parseSettings,
  taskIdsOf,
  type Activity,
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
import { bridgeStatus, downloadOutcome, keptMessage } from "./lib/status";
import { linkLabel } from "./lib/url";

const MENU_ID = "downpipe-download-link";
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

// ---------- Link context menu ----------

function createMenu(): void {
  void ext.contextMenus.removeAll().then(() => {
    ext.contextMenus.create({
      id: MENU_ID,
      title: "Download with Downpipe",
      contexts: ["link"],
      targetUrlPatterns: ["http://*/*", "https://*/*"],
    });
  });
}

ext.runtime.onInstalled.addListener(createMenu);
ext.runtime.onStartup.addListener(createMenu);

ext.contextMenus.onClicked.addListener((info) => {
  if (info.menuItemId !== MENU_ID) return;
  void sendLink(info);
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

// ---------- Capturing browser downloads ----------
// On only while the optional `downloads` permission is granted; the popup's
// toggle requests or removes it.

/** True when Downpipe took the download and the browser should drop it. */
async function handOff(item: DownloadItemLike): Promise<boolean> {
  const decision = decideCapture(item, ext.runtime.id);
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

// Chrome and Edge: the browser has the server's file name and MIME type by
// now and holds the download until every listener has called `suggest`.
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
  if (!decideCapture(item, ext.runtime.id).capture) return;
  void (async () => {
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

// ---------- Activity, badge and notifications ----------

async function settings(): Promise<Settings> {
  const stored = await ext.storage.local.get(SETTINGS_KEY).catch((): Record<string, unknown> => ({}));
  return parseSettings(stored[SETTINGS_KEY]);
}

async function storedActivity(): Promise<Activity[]> {
  const stored = await ext.storage.session.get(ACTIVITY_KEY).catch((): Record<string, unknown> => ({}));
  return parseActivity(stored[ACTIVITY_KEY]);
}

// Reports are chained: two quick clicks would otherwise both read the old
// list and the second write would drop the first entry.
let reports: Promise<void> = Promise.resolve();
let badgeTimer: ReturnType<typeof setTimeout> | undefined;

function report(entry: Omit<Activity, "at">): Promise<void> {
  const activity: Activity = { ...entry, at: Date.now() };
  reports = reports.then(() => record(activity)).catch(() => undefined);
  return reports;
}

async function record(activity: Activity): Promise<void> {
  try {
    // Session storage: cleared when the browser closes, never written to disk.
    const stored = await ext.storage.session.get(ACTIVITY_KEY);
    await ext.storage.session.set({
      [ACTIVITY_KEY]: addActivity(parseActivity(stored[ACTIVITY_KEY]), activity),
    });
  } catch {
    // The badge below still tells the user what happened.
  }
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

// ---------- Popup ----------

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
  return { bridge: bridgeStatus(result), capture, notify: prefs.notify, activity, tasks: tasks ?? [] };
}

async function answer(request: PopupRequest): Promise<StatusReply | TasksReply | Settings> {
  switch (request.kind) {
    case "status":
      return status();
    case "tasks":
      return { tasks: await liveTasks(await storedActivity()) };
    case "set_notify": {
      const next: Settings = { ...(await settings()), notify: request.enabled };
      await ext.storage.local.set({ [SETTINGS_KEY]: next });
      return next;
    }
  }
}

ext.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Only this extension's own pages; it has no content scripts and does
  // not listen to other extensions (no onMessageExternal).
  if (!isTrustedSender(sender, ext.runtime.id, ext.runtime.getURL(""))) return false;
  const request = parsePopupRequest(message);
  if (!request) return false;
  answer(request).then(sendResponse, () => sendResponse(null));
  return true;
});