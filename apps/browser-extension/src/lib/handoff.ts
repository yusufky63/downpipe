// Handing a browser download to Downpipe without ever downloading it twice.
//
// The browser's copy is removed only once the app has the download, and
// only while the browser's copy is unfinished; when the app does not take
// it, the browser's copy carries on as if the extension were not there.
// Each browser download is offered once: the service worker can be
// restarted in the middle of a hand-over, and the ledger of offered ids
// (session storage, so it ends with the browser) keeps a second event for
// the same download from sending it again. Nothing is queued for later, so
// a browser or app restart never sends anything a second time.
//
// The service worker wires this to `downloads` and the native host; the
// tests drive it with fakes of both.
import { decideCapture, type DownloadItemLike } from "./capture";
import type { CaptureFilters } from "./filters";
import type { HostResult } from "./host";
import type { Activity } from "./messages";
import { downloadMessage, fileNameHint, newMessageId, type HostMessage } from "./protocol";
import { downloadOutcome, keptMessage } from "./status";
import { linkLabel } from "./url";

/** The parts of `chrome.downloads` a hand-over uses. */
export interface DownloadsLike {
  cancel(id: number): Promise<void>;
  erase(query: { id: number }): Promise<unknown>;
  pause(id: number): Promise<void>;
  resume(id: number): Promise<void>;
  search(query: { id: number }): Promise<{ state?: string }[]>;
}

export interface HandoffDeps {
  downloads: DownloadsLike;
  extensionId: string;
  filters(): Promise<CaptureFilters>;
  /**
   * Sends a message to the native host and resolves with its answer however
   * long that takes; the host gives up on the app after 10 seconds itself.
   */
  send(message: HostMessage): Promise<HostResult>;
  report(entry: Omit<Activity, "at">): Promise<void>;
  /** False when this browser download was already offered once. */
  claim(id: number): Promise<boolean>;
  /** How long the browser waits for the app before it goes on by itself. */
  answerMs: number;
  /** After that, how long a late answer is still acted on. */
  lateMs: number;
  /** Timers, replaceable in tests. */
  wait?: (ms: number) => Promise<void>;
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const TIMED_OUT: HostResult = { ok: false, failure: "timeout", detail: "No answer in time" };

/** Taken: the app has it, or already had it. */
function taken(result: HostResult): boolean {
  const { outcome } = downloadOutcome(result);
  return outcome === "added" || outcome === "duplicate";
}

/**
 * Cancels and erases the browser's copy, unless it already finished: an
 * erased finished download would leave its file on disk with no trace in
 * the browser, beside the app's copy.
 */
export async function removeBrowserCopy(downloads: DownloadsLike, id: number): Promise<boolean> {
  const [item] = await downloads.search({ id }).catch((): { state?: string }[] => []);
  if (item?.state === "complete") return false;
  await downloads.cancel(id).catch(() => undefined);
  await downloads.erase({ id }).catch(() => undefined);
  return true;
}

async function reportTaken(deps: HandoffDeps, item: DownloadItemLike, result: HostResult): Promise<void> {
  const { outcome, message, fileName, taskId, awaitingConfirmation } = downloadOutcome(result);
  const label = fileName ?? fileNameHint(item.filename) ?? linkLabel(item.url);
  // Reports never decide the outcome: a failed write must not keep a copy in both places.
  await deps.report({ source: "capture", label, outcome, message, taskId, awaitingConfirmation }).catch(() => undefined);
}

async function reportKept(deps: HandoffDeps, item: DownloadItemLike, result: HostResult): Promise<void> {
  const label = fileNameHint(item.filename) ?? linkLabel(item.url);
  await deps.report({ source: "capture", label, outcome: "kept", message: keptMessage(result) }).catch(() => undefined);
}

/**
 * Offers one browser download to the app. True when the app took it and the
 * caller should remove the browser's copy now. When the app answers only
 * after `answerMs`, the browser has gone on with its copy; an acceptance
 * that still arrives within `lateMs` removes that copy here (`late`
 * resolves once that is settled).
 */
export async function offer(
  item: DownloadItemLike,
  deps: HandoffDeps,
): Promise<{ handed: boolean; late: Promise<void> }> {
  const none = { handed: false, late: Promise.resolve() };
  const decision = decideCapture(item, deps.extensionId, await deps.filters());
  if (!decision.capture) return none;
  if (!(await deps.claim(item.id).catch(() => true))) return none;
  const message = downloadMessage(decision.fields, newMessageId());
  if (!message) return none;
  const wait = deps.wait ?? sleep;
  const answer = deps.send(message);
  const first = await Promise.race([answer, wait(deps.answerMs).then(() => null)]);
  if (first) {
    if (taken(first)) {
      await reportTaken(deps, item, first);
      return { handed: true, late: Promise.resolve() };
    }
    await reportKept(deps, item, first);
    return none;
  }
  // Too slow for the browser to wait: it keeps downloading, and the answer
  // decides afterwards which copy stays.
  const late = (async () => {
    const result = (await Promise.race([answer, wait(deps.lateMs).then(() => null)])) ?? TIMED_OUT;
    if (taken(result) && (await removeBrowserCopy(deps.downloads, item.id))) await reportTaken(deps, item, result);
    else await reportKept(deps, item, taken(result) ? TIMED_OUT : result);
  })();
  return { handed: false, late };
}

/**
 * Chrome and Edge: `onDeterminingFilename` holds the download until
 * `suggest` is called, so the app is asked first and the browser's copy
 * never starts writing its file when the app takes it.
 */
export async function captureHeld(item: DownloadItemLike, deps: HandoffDeps, suggest: () => void): Promise<void> {
  let released = false;
  const release = () => {
    if (released) return;
    released = true;
    try {
      suggest();
    } catch {
      // The download is gone already.
    }
  };
  try {
    const { handed, late } = await offer(item, deps).catch(() => ({ handed: false, late: Promise.resolve() }));
    if (handed) await removeBrowserCopy(deps.downloads, item.id);
    // Released before a late answer, which may then remove the copy.
    release();
    await late;
  } finally {
    release();
  }
}

/**
 * Firefox: no event holds a download, so one the filters let through is
 * paused while the app is asked, and resumed when the app does not take it.
 * A download that finished before the pause (a small file on a fast line)
 * is left alone: offering it would download it a second time.
 */
export async function captureStarted(item: DownloadItemLike, deps: HandoffDeps): Promise<void> {
  if (!decideCapture(item, deps.extensionId, await deps.filters()).capture) return;
  await deps.downloads.pause(item.id).catch(() => undefined);
  const [now] = await deps.downloads.search({ id: item.id }).catch((): { state?: string }[] => []);
  if (!now || now.state === "complete") return;
  const { handed, late } = await offer(item, deps).catch(() => ({
    handed: false,
    late: Promise.resolve(),
  }));
  if (handed && (await removeBrowserCopy(deps.downloads, item.id))) return;
  await deps.downloads.resume(item.id).catch(() => undefined);
  await late;
}

/** At most this many offered ids are remembered; downloads are offered within seconds. */
export const MAX_OFFERED = 200;

/** The ledger's next list, or null when `id` is already in it. */
export function claimIn(offered: readonly number[], id: number): number[] | null {
  if (offered.includes(id)) return null;
  return [...offered, id].slice(-MAX_OFFERED);
}

/** Session storage's value, whatever was stored. */
export function parseOffered(value: unknown): number[] {
  return Array.isArray(value) ? value.filter((n): n is number => Number.isSafeInteger(n) && n >= 0).slice(-MAX_OFFERED) : [];
}
