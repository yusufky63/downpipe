// Links collected from a page: pick some, send them to Downpipe one by one
// through the service worker, see what happened to each.
import { runLimited } from "../lib/batch";
import { ext } from "../lib/browser";
import { hostOf } from "../lib/filters";
import {
  MAX_PAGE_LINKS,
  collectionKey,
  downloadLabel,
  extensionChips,
  filterLinks,
  linkView,
  parseCollection,
  summarize,
  withoutPages,
  type LinkCollection,
  type LinkOutcome,
  type LinkView,
} from "../lib/links";
import { isLinkResult, type LinkResult, type PingReply } from "../lib/messages";
import { statusView, type StatusView, type Tone } from "../lib/view";
import { byId, extensionInfo, paintStatus, span } from "../ui/dom";

/** Links handed over at once; each one starts a native host process. */
const CONCURRENCY = 3;

const els = {
  title: byId("title"),
  pageUrl: byId("page-url"),
  found: byId("found"),
  bridge: byId("bridge"),
  status: {
    box: byId("status"),
    title: byId("status-title"),
    detail: byId("status-detail"),
    command: byId("status-command"),
    guide: byId<HTMLAnchorElement>("guide"),
  },
  recheck: byId<HTMLButtonElement>("recheck"),
  notice: byId("notice"),
  picker: byId("picker"),
  query: byId<HTMLInputElement>("query"),
  hidePages: byId<HTMLInputElement>("hide-pages"),
  chips: byId("chips"),
  selectAll: byId<HTMLButtonElement>("select-all"),
  selectNone: byId<HTMLButtonElement>("select-none"),
  count: byId("count"),
  rows: byId<HTMLUListElement>("rows"),
  empty: byId("empty"),
  dock: byId("dock"),
  summary: byId("summary"),
  stop: byId<HTMLButtonElement>("stop"),
  download: byId<HTMLButtonElement>("download"),
};

type RowState = { state: "sending" } | { state: LinkOutcome; message: string };

const ROW_VIEW: Record<RowState["state"], { badge: string; tone: Tone }> = {
  sending: { badge: "Sending", tone: "primary" },
  added: { badge: "Added", tone: "positive" },
  duplicate: { badge: "In Downpipe", tone: "muted" },
  failed: { badge: "Failed", tone: "danger" },
};

let collection: LinkCollection | null = null;
let views: LinkView[] = [];
let visible: LinkView[] = [];
/** Chosen addresses. Kept across filter changes; only visible ones are sent. */
const selected = new Set<string>();
const results = new Map<string, RowState>();
const rowsByUrl = new Map<string, HTMLLIElement>();
let types = new Set<string>();
let sending = false;
let stopRequested = false;

// ---------- Reading the collection ----------

/**
 * The service worker left the links in session storage under the id in
 * this page's address; they are removed once read. A copy goes into this
 * tab's own sessionStorage so a reload still shows them.
 */
async function readCollection(): Promise<LinkCollection | null> {
  const key = collectionKey(new URLSearchParams(location.search).get("id") ?? "");
  if (!key) return null;
  try {
    const stored = await ext.storage.session.get(key);
    if (key in stored) {
      await ext.storage.session.remove(key).catch(() => undefined);
      const fresh = parseCollection(stored[key]);
      if (fresh) {
        try {
          sessionStorage.setItem(key, JSON.stringify(fresh));
        } catch {
          // Too big to keep: a reload then asks to collect again.
        }
      }
      return fresh;
    }
  } catch {
    // Fall through to the copy from an earlier load of this tab.
  }
  try {
    const kept = sessionStorage.getItem(key);
    return kept ? parseCollection(JSON.parse(kept)) : null;
  } catch {
    return null;
  }
}

// ---------- Rendering ----------

function showNotice(text: string): void {
  els.notice.textContent = text;
  els.notice.hidden = !text;
}

function chipButton(key: string, label: string, count: number | null, pressed: boolean): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "chip";
  button.dataset.key = key;
  button.setAttribute("aria-pressed", String(pressed));
  button.append(span("chip-label", label));
  if (count !== null) button.append(span("chip-count num", count.toLocaleString("en-US")));
  return button;
}

function renderChips(): void {
  const chips = extensionChips(withoutPages(views, els.hidePages.checked));
  // A type that "Hide web pages" just took away cannot stay chosen.
  types = new Set([...types].filter((key) => chips.some((c) => c.key === key)));
  const buttons = chips.map((c) => chipButton(c.key, c.label, c.count, types.has(c.key)));
  els.chips.replaceChildren(...(chips.length > 1 ? [chipButton("", "All types", null, types.size === 0), ...buttons] : []));
}

function paintRow(li: HTMLLIElement, url: string): void {
  const result = results.get(url);
  const badge = li.querySelector<HTMLElement>(".row-result");
  if (!badge) return;
  if (!result) {
    delete li.dataset.tone;
    badge.hidden = true;
    const note = li.querySelector<HTMLElement>(".row-note");
    if (note) note.hidden = true;
    return;
  }
  const view = ROW_VIEW[result.state];
  li.dataset.tone = view.tone;
  badge.hidden = false;
  badge.textContent = view.badge;
  badge.title = "message" in result ? result.message : "";
  // The reason a link failed, under its address; a tooltip alone would be
  // out of reach on touch screens.
  const note = li.querySelector<HTMLElement>(".row-note");
  if (note) {
    note.textContent = result.state === "failed" ? result.message : "";
    note.hidden = result.state !== "failed";
  }
}

function row(view: LinkView, index: number): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "row";
  const label = document.createElement("label");
  label.className = "row-label";
  const check = document.createElement("input");
  check.type = "checkbox";
  check.className = "check";
  check.id = `link-${index}`;
  check.checked = selected.has(view.url);
  check.disabled = sending;
  check.dataset.url = view.url;
  const title = view.text || view.name;
  const text = span("row-text", title);
  text.title = title;
  const url = span("row-url", view.url);
  url.title = view.url;
  const tags = document.createElement("span");
  tags.className = "row-tags";
  const result = span("badge row-result", "");
  result.hidden = true;
  tags.append(result);
  if (view.ext) tags.append(span("badge row-ext", view.ext));
  const note = span("row-note", "");
  note.hidden = true;
  label.append(check, text, tags, url, note);
  li.append(label);
  paintRow(li, view.url);
  return li;
}

function renderCount(): void {
  const chosen = visible.filter((v) => selected.has(v.url)).length;
  els.count.textContent = `${chosen.toLocaleString("en-US")} of ${visible.length.toLocaleString("en-US")} selected`;
  if (!sending) {
    els.download.textContent = downloadLabel(chosen);
    els.download.disabled = chosen === 0;
  }
}

function renderList(): void {
  renderChips();
  visible = filterLinks(views, { query: els.query.value, hidePages: els.hidePages.checked, types });
  rowsByUrl.clear();
  const items = visible.map((view, i) => {
    const li = row(view, i);
    rowsByUrl.set(view.url, li);
    return li;
  });
  els.rows.replaceChildren(...items);
  els.empty.hidden = visible.length > 0;
  const onlyPages = els.hidePages.checked && views.length > 0 && views.every((v) => v.page);
  els.empty.textContent = onlyPages
    ? "Every link here is a web page. Turn off Hide web pages to see them."
    : "No links match the filter.";
  renderCount();
}

function renderIntro(c: LinkCollection): void {
  const host = hostOf(c.pageUrl);
  els.title.textContent = c.title || (host ? `Links on ${host}` : "Links on this page");
  els.pageUrl.textContent = c.pageUrl;
  els.pageUrl.title = c.pageUrl;
  els.pageUrl.hidden = !c.pageUrl;
  const n = c.links.length;
  els.found.textContent =
    n === 0
      ? ""
      : `${n.toLocaleString("en-US")} ${n === 1 ? "link" : "links"} found.` +
        (c.truncated ? ` Only the first ${MAX_PAGE_LINKS.toLocaleString("en-US")} are listed.` : "");
  els.found.hidden = n === 0;
}

// ---------- The bridge ----------

async function checkBridge(): Promise<StatusView> {
  els.recheck.disabled = true;
  try {
    const reply: unknown = await ext.runtime.sendMessage({ kind: "ping" }).catch(() => null);
    const bridge =
      typeof reply === "object" && reply !== null && typeof (reply as PingReply).bridge === "object"
        ? (reply as PingReply).bridge
        : ({ state: "error", message: "The extension did not answer." } as const);
    const view = statusView(bridge, extensionInfo());
    paintStatus(els.status, view);
    // Nothing to say while connected.
    els.bridge.hidden = view.tone === "positive";
    return view;
  } finally {
    els.recheck.disabled = false;
  }
}

// ---------- Sending ----------

function setSending(on: boolean): void {
  sending = on;
  els.stop.hidden = !on;
  els.stop.disabled = false;
  for (const el of [els.query, els.hidePages, els.selectAll, els.selectNone]) el.disabled = on;
  for (const el of els.chips.querySelectorAll<HTMLButtonElement>("button")) el.disabled = on;
  for (const el of els.rows.querySelectorAll<HTMLInputElement>(".check")) el.disabled = on;
  if (on) els.download.disabled = true;
  else renderCount();
}

function setResult(url: string, state: RowState): void {
  results.set(url, state);
  const li = rowsByUrl.get(url);
  if (li) paintRow(li, url);
}

async function sendOne(view: LinkView, pageUrl: string): Promise<LinkResult> {
  const request = pageUrl
    ? { kind: "send_link", url: view.url, pageUrl }
    : { kind: "send_link", url: view.url };
  const reply: unknown = await ext.runtime.sendMessage(request).catch(() => null);
  return isLinkResult(reply)
    ? reply
    : { outcome: "failed", message: "The extension did not answer.", stop: true };
}

async function sendChosen(): Promise<void> {
  if (!collection || sending) return;
  const pageUrl = collection.pageUrl;
  const chosen = visible.filter((v) => selected.has(v.url));
  if (!chosen.length) return;
  setSending(true);
  stopRequested = false;
  els.summary.dataset.tone = "muted";
  els.summary.textContent = "";
  els.download.textContent = "Checking Downpipe…";
  // One check first: with the app closed nothing is sent, instead of the
  // first few links failing.
  const bridge = await checkBridge();
  if (bridge.tone !== "positive") {
    setSending(false);
    els.summary.dataset.tone = "danger";
    els.summary.textContent = `Nothing sent: ${bridge.title}.`;
    els.bridge.scrollIntoView({ block: "nearest" });
    return;
  }
  const outcomes: LinkOutcome[] = [];
  let bridgeProblem: string | null = null;
  const progress = () => {
    els.download.textContent = `Sending ${outcomes.length.toLocaleString("en-US")} of ${chosen.length.toLocaleString("en-US")}…`;
  };
  progress();
  await runLimited(
    chosen,
    CONCURRENCY,
    async (view) => {
      setResult(view.url, { state: "sending" });
      const result = await sendOne(view, pageUrl);
      outcomes.push(result.outcome);
      setResult(view.url, { state: result.outcome, message: result.message });
      // Sent once is enough: a second press only sends what is left.
      if (result.outcome !== "failed") selected.delete(view.url);
      if (result.stop) bridgeProblem ??= result.message;
      progress();
    },
    () => stopRequested || bridgeProblem !== null,
  );
  setSending(false);
  for (const view of chosen) {
    const li = rowsByUrl.get(view.url);
    if (li) {
      paintRow(li, view.url);
      const check = li.querySelector<HTMLInputElement>(".check");
      if (check) check.checked = selected.has(view.url);
    }
  }
  renderCount();
  const summary = summarize(outcomes);
  if (bridgeProblem) {
    els.summary.dataset.tone = "danger";
    els.summary.textContent = `Stopped: ${bridgeProblem}`;
    await checkBridge();
  } else if (stopRequested) {
    els.summary.dataset.tone = "muted";
    els.summary.textContent = `Stopped. ${summary}`;
  } else {
    const failed = outcomes.filter((o) => o === "failed").length;
    els.summary.dataset.tone = failed === 0 ? "positive" : failed === outcomes.length ? "danger" : "muted";
    els.summary.textContent = summary;
  }
}

// ---------- Events ----------

els.rows.addEventListener("change", (event) => {
  const check = event.target;
  if (!(check instanceof HTMLInputElement) || !check.dataset.url) return;
  if (check.checked) selected.add(check.dataset.url);
  else selected.delete(check.dataset.url);
  renderCount();
});

els.chips.addEventListener("click", (event) => {
  const button = (event.target as Element).closest<HTMLButtonElement>(".chip");
  if (!button || sending) return;
  const key = button.dataset.key ?? "";
  if (!key) types = new Set();
  else if (types.has(key)) types.delete(key);
  else types.add(key);
  renderList();
});

els.query.addEventListener("input", renderList);
els.hidePages.addEventListener("change", renderList);

els.selectAll.addEventListener("click", () => {
  for (const view of visible) selected.add(view.url);
  renderList();
});
els.selectNone.addEventListener("click", () => {
  for (const view of visible) selected.delete(view.url);
  renderList();
});

els.download.addEventListener("click", () => void sendChosen());
els.stop.addEventListener("click", () => {
  stopRequested = true;
  els.stop.disabled = true;
});
els.recheck.addEventListener("click", () => void checkBridge());

window.addEventListener("beforeunload", (event) => {
  if (sending) event.preventDefault();
});

async function start(): Promise<void> {
  collection = await readCollection();
  void checkBridge();
  if (!collection) {
    showNotice(
      "These links are no longer available. Go back to the page and choose Download links on this page with Downpipe again.",
    );
    return;
  }
  renderIntro(collection);
  if (collection.unreadable) {
    showNotice(
      "The browser does not let extensions read this page. Its own pages, extension stores and some built-in viewers cannot be read.",
    );
    return;
  }
  if (!collection.links.length) {
    showNotice("No http or https links were found on this page.");
    return;
  }
  views = collection.links.map(linkView);
  // Files start chosen, web pages do not, as in IDM's link list.
  for (const view of views) if (!view.page) selected.add(view.url);
  // Addresses without an extension count as pages; when that is all there
  // is (`/download/123` style links), hiding them would show an empty list.
  if (selected.size === 0) els.hidePages.checked = false;
  els.picker.hidden = false;
  els.dock.hidden = false;
  renderList();
}

void start();
