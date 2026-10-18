import { describe, expect, it } from "vitest";
import type { DownloadItemLike } from "./capture";
import { CAPTURE_EVERYTHING, defaultFilters } from "./filters";
import {
  MAX_OFFERED,
  captureHeld,
  captureStarted,
  claimIn,
  offer,
  parseOffered,
  type DownloadsLike,
  type HandoffDeps,
} from "./handoff";
import type { HostResult } from "./host";
import type { Activity } from "./messages";

const ACCEPTED: HostResult = { ok: true, response: { type: "accepted", taskId: "t1", fileName: "big.zip" } };
const DUPLICATE: HostResult = { ok: true, response: { type: "error", code: "duplicate", message: "x", taskId: "t0" } };
const CLOSED: HostResult = { ok: true, response: { type: "error", code: "app_not_running", message: "x" } };

/** The browser's download list, as far as a hand-over sees it. */
class FakeDownloads implements DownloadsLike {
  calls: string[] = [];
  states = new Map<number, string>();

  constructor(entries: [number, string][] = []) {
    for (const [id, state] of entries) this.states.set(id, state);
  }

  cancel(id: number): Promise<void> {
    this.calls.push(`cancel ${id}`);
    this.states.set(id, "interrupted");
    return Promise.resolve();
  }

  erase(query: { id: number }): Promise<unknown> {
    this.calls.push(`erase ${query.id}`);
    this.states.delete(query.id);
    return Promise.resolve([query.id]);
  }

  pause(id: number): Promise<void> {
    this.calls.push(`pause ${id}`);
    // A finished download cannot be paused.
    if (this.states.get(id) === "complete") return Promise.reject(new Error("not in progress"));
    this.states.set(id, "interrupted");
    return Promise.resolve();
  }

  resume(id: number): Promise<void> {
    this.calls.push(`resume ${id}`);
    this.states.set(id, "in_progress");
    return Promise.resolve();
  }

  search(query: { id: number }): Promise<{ state?: string }[]> {
    const state = this.states.get(query.id);
    return Promise.resolve(state ? [{ state }] : []);
  }
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve: (value: T) => void = () => undefined;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

const ANSWER_MS = 5;
const LATE_MS = 15;

function setup(answers: (HostResult | Promise<HostResult>)[], downloads = new FakeDownloads([[7, "in_progress"]])) {
  const sent: unknown[] = [];
  const reports: Omit<Activity, "at">[] = [];
  const offered: number[] = [];
  // The browser stops waiting when a test calls timeOut(); a late answer
  // never times out by itself, so each test decides what the app says, and when.
  const patience = deferred<undefined>();
  const deps: HandoffDeps = {
    downloads,
    extensionId: "ext",
    filters: () => Promise.resolve(CAPTURE_EVERYTHING),
    send: (message) => {
      sent.push(message);
      return Promise.resolve(answers.shift() ?? CLOSED);
    },
    report: (entry) => {
      reports.push(entry);
      return Promise.resolve();
    },
    claim: (id) => {
      const next = claimIn(offered, id);
      if (!next) return Promise.resolve(false);
      offered.splice(0, offered.length, ...next);
      return Promise.resolve(true);
    },
    answerMs: ANSWER_MS,
    lateMs: LATE_MS,
    wait: (ms) => (ms === ANSWER_MS ? patience.promise : new Promise<undefined>(() => undefined)),
  };
  return { deps, downloads, sent, reports, timeOut: () => patience.resolve(undefined) };
}

const item = (extra: Partial<DownloadItemLike> = {}): DownloadItemLike => ({
  id: 7,
  url: "https://example.org/big.zip",
  filename: "C:\\Users\\u\\Downloads\\big.zip",
  state: "in_progress",
  ...extra,
});

describe("Chrome and Edge (held downloads)", () => {
  it("cancels and erases the browser's copy once the app accepts, before releasing it", async () => {
    const { deps, downloads, reports } = setup([ACCEPTED]);
    const order: string[] = [];
    await captureHeld(item(), deps, () => order.push(...downloads.calls, "suggest"));
    expect(order).toEqual(["cancel 7", "erase 7", "suggest"]);
    expect(downloads.states.has(7)).toBe(false);
    expect(reports).toMatchObject([{ source: "capture", outcome: "added", label: "big.zip", taskId: "t1" }]);
  });

  it("drops the browser's copy of a link the app already has", async () => {
    const { deps, downloads, reports } = setup([DUPLICATE]);
    await captureHeld(item(), deps, () => undefined);
    expect(downloads.calls).toEqual(["cancel 7", "erase 7"]);
    expect(reports[0]?.outcome).toBe("duplicate");
  });

  it("leaves the browser's copy alone when the app does not take it", async () => {
    const { deps, downloads, reports } = setup([CLOSED]);
    let released = 0;
    await captureHeld(item(), deps, () => (released += 1));
    expect(downloads.calls).toEqual([]);
    expect(released).toBeGreaterThan(0);
    expect(reports).toMatchObject([{ outcome: "kept", message: "Downpipe was not running, so the browser downloaded it." }]);
  });

  it("removes the browser's copy when the app accepts after the browser went on", async () => {
    const answer = deferred<HostResult>();
    const { deps, downloads, reports, timeOut } = setup([answer.promise]);
    const events: string[] = [];
    const done = captureHeld(item(), deps, () => events.push("suggest"));
    timeOut();
    await new Promise((r) => setTimeout(r, 0));
    // Released on time, nothing removed yet.
    expect(events).toEqual(["suggest"]);
    expect(downloads.calls).toEqual([]);
    answer.resolve(ACCEPTED);
    await done;
    expect(downloads.calls).toEqual(["cancel 7", "erase 7"]);
    expect(reports).toMatchObject([{ outcome: "added" }]);
  });

  it("keeps a copy the browser finished before a late acceptance", async () => {
    const answer = deferred<HostResult>();
    const { deps, downloads, reports, timeOut } = setup([answer.promise]);
    const done = captureHeld(item(), deps, () => undefined);
    timeOut();
    await new Promise((r) => setTimeout(r, 0));
    downloads.states.set(7, "complete");
    answer.resolve(ACCEPTED);
    await done;
    // Erasing it would hide a finished file the user has on disk.
    expect(downloads.calls).toEqual([]);
    expect(reports).toMatchObject([{ outcome: "kept" }]);
  });

  it("says the browser kept it when a late answer is a refusal", async () => {
    const answer = deferred<HostResult>();
    const { deps, downloads, reports, timeOut } = setup([answer.promise]);
    const done = captureHeld(item(), deps, () => undefined);
    timeOut();
    answer.resolve(CLOSED);
    await done;
    expect(downloads.calls).toEqual([]);
    expect(reports).toMatchObject([{ outcome: "kept" }]);
  });

  it("does not keep both copies when the popup's list cannot be written", async () => {
    const { deps, downloads } = setup([ACCEPTED]);
    deps.report = () => Promise.reject(new Error("storage is full"));
    await captureHeld(item(), deps, () => undefined);
    expect(downloads.calls).toEqual(["cancel 7", "erase 7"]);
  });
});

describe("Firefox (started downloads)", () => {
  it("pauses, hands over and erases the browser's copy", async () => {
    const { deps, downloads, sent } = setup([ACCEPTED]);
    await captureStarted(item(), deps);
    expect(sent).toHaveLength(1);
    expect(downloads.calls).toEqual(["pause 7", "cancel 7", "erase 7"]);
  });

  it("resumes the browser's copy when the app does not take it", async () => {
    const { deps, downloads } = setup([CLOSED]);
    await captureStarted(item(), deps);
    expect(downloads.calls).toEqual(["pause 7", "resume 7"]);
    expect(downloads.states.get(7)).toBe("in_progress");
  });

  it("never offers a download that finished before it could be paused", async () => {
    const downloads = new FakeDownloads([[7, "complete"]]);
    const { deps, sent } = setup([ACCEPTED], downloads);
    await captureStarted(item(), deps);
    expect(sent).toEqual([]);
    expect(downloads.calls).toEqual(["pause 7"]);
  });

  it("never pauses a download the filters leave to the browser", async () => {
    const { deps, downloads, sent } = setup([ACCEPTED]);
    deps.filters = () => Promise.resolve(defaultFilters());
    await captureStarted(item({ url: "https://example.org/page.html", filename: "page.html" }), deps);
    expect(sent).toEqual([]);
    expect(downloads.calls).toEqual([]);
  });

  it("removes a copy it already resumed when the app accepts late", async () => {
    const answer = deferred<HostResult>();
    const { deps, downloads, timeOut } = setup([answer.promise]);
    const done = captureStarted(item(), deps);
    timeOut();
    await new Promise((r) => setTimeout(r, 0));
    expect(downloads.calls).toEqual(["pause 7", "resume 7"]);
    answer.resolve(ACCEPTED);
    await done;
    expect(downloads.calls).toEqual(["pause 7", "resume 7", "cancel 7", "erase 7"]);
  });
});

describe("one offer per download", () => {
  it("never sends the same browser download twice", async () => {
    const { deps, sent } = setup([CLOSED, ACCEPTED]);
    await offer(item(), deps);
    // A second event for the same id, for example after the service worker restarted.
    const again = await offer(item(), deps);
    expect(again.handed).toBe(false);
    expect(sent).toHaveLength(1);
  });

  it("leaves downloads the browser brings back after a restart alone", async () => {
    // After a restart the browser lists unfinished downloads as interrupted;
    // the ledger is empty again, and they still are not offered.
    const { deps, sent } = setup([ACCEPTED], new FakeDownloads([[7, "interrupted"]]));
    await captureStarted(item({ state: "interrupted" }), deps);
    await captureHeld(item({ state: "interrupted" }), deps, () => undefined);
    expect(sent).toEqual([]);
  });

  it("keeps a bounded list of offered ids", () => {
    expect(claimIn([1, 2], 2)).toBeNull();
    expect(claimIn([1, 2], 3)).toEqual([1, 2, 3]);
    const full = Array.from({ length: MAX_OFFERED }, (_, i) => i);
    expect(claimIn(full, 9999)).toHaveLength(MAX_OFFERED);
    expect(claimIn(full, 9999)?.at(-1)).toBe(9999);
    expect(parseOffered([1, "2", -1, 2.5, 3])).toEqual([1, 3]);
    expect(parseOffered(undefined)).toEqual([]);
  });
});
