// The Download button over playing videos. The service worker registers
// this content script only while media detection is on. It sends nothing by
// itself: a click by the user asks the service worker for this tab's media
// (detected from the page's network requests), and sending one is a second
// request, again only after a click. The page cannot see into the button
// (a closed shadow root) and synthetic clicks from page scripts are ignored.
import { ext } from "../lib/browser";
import { PAGE_ITEM_ID, isOverlayItems, overlayResultText, type OverlayItem } from "../lib/messages";

/** Thumbnails and decorative loops stay without a button. */
const MIN_WIDTH = 200;
const MIN_HEIGHT = 112;
/** The button stays this long after the pointer left the video. */
const LINGER_MS = 1500;
/** And this long after a video starts playing, so it is noticed. */
const AFTER_PLAY_MS = 4000;
const NOTE_MS = 4000;
/** How often the pointer may trigger a look for a video under it. */
const PROBE_MS = 250;

const STYLE = `
:host { all: initial; }
.wrap {
  position: fixed; top: 0; left: 0; display: grid; justify-items: end; gap: 6px;
  max-width: 320px; pointer-events: auto; transform: translateX(-100%);
  font: 500 13px/1.4 "DM Sans", system-ui, sans-serif; color: #0a0b0d;
  -webkit-font-smoothing: antialiased;
}
button { font: inherit; cursor: pointer; }
.main {
  display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 12px;
  border: 1px solid #075e5a; border-radius: 6px; background: #0b7a75; color: #fff;
  white-space: nowrap;
}
.main:hover { background: #075e5a; }
.main:focus-visible, .item:focus-visible { outline: 2px solid #2bb3a6; outline-offset: 2px; }
.main:disabled { cursor: progress; opacity: 0.8; }
.menu, .note {
  min-width: 220px; max-width: 320px; border: 1px solid #dee1e7; border-radius: 8px;
  /* The desktop app's popover shadow (--shadow-popover). */
  background: #fff; box-shadow: 0 8px 24px rgba(10, 11, 13, 0.12);
}
.menu { display: grid; padding: 4px; max-height: 240px; overflow-y: auto; }
.item {
  display: grid; gap: 2px; padding: 8px 10px; border: 0; border-radius: 6px;
  background: none; color: inherit; text-align: left;
}
.item:hover { background: #eef0f3; }
.label { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.detail { font: 400 12px/1.4 "JetBrains Mono", ui-monospace, monospace; color: #5b616e; }
.note { padding: 8px 10px; font-weight: 400; }
.note[data-tone="danger"] { color: #c62a0f; }
[hidden] { display: none !important; }
@media (prefers-color-scheme: dark) {
  .wrap { color: #fff; }
  .menu, .note { border-color: #2b2e36; background: #111318; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5); }
  .item:hover { background: #17191f; }
  .detail { color: #b1b7c3; }
  .note[data-tone="danger"] { color: #ff6a4d; }
}
@media (prefers-reduced-motion: no-preference) {
  .main, .item { transition: background-color 120ms cubic-bezier(0.2, 0, 0, 1); }
}
`;

const SVG = "http://www.w3.org/2000/svg";

/** A download arrow, drawn with DOM calls so no HTML parsing is involved. */
function icon(): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  for (const [k, v] of Object.entries({
    width: "14",
    height: "14",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    "stroke-width": "2",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    "aria-hidden": "true",
  })) {
    svg.setAttribute(k, v);
  }
  for (const d of ["M12 4v12", "m6 10 6 6 6-6", "M5 20h14"]) {
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

interface Ui {
  host: HTMLElement;
  wrap: HTMLElement;
  main: HTMLButtonElement;
  menu: HTMLElement;
  note: HTMLElement;
}

function build(): Ui {
  const host = document.createElement("div");
  const root = host.attachShadow({ mode: "closed" });
  const style = document.createElement("style");
  style.textContent = STYLE;
  const wrap = document.createElement("div");
  wrap.className = "wrap";
  const main = document.createElement("button");
  main.type = "button";
  main.className = "main";
  main.setAttribute("aria-haspopup", "menu");
  main.title = "Download this video with Downpipe";
  main.append(icon(), document.createTextNode("Download"));
  const menu = document.createElement("div");
  menu.className = "menu";
  menu.setAttribute("role", "menu");
  menu.hidden = true;
  const note = document.createElement("div");
  note.className = "note";
  note.setAttribute("role", "status");
  note.hidden = true;
  wrap.append(main, menu, note);
  root.append(style, wrap);
  return { host, wrap, main, menu, note };
}

/** Big enough to be the thing being watched, and not DRM-protected. */
function eligible(video: HTMLVideoElement): boolean {
  if (!video.isConnected) return false;
  // Encrypted Media Extensions are in use: Downpipe refuses protected media,
  // so it does not offer a button that could only fail.
  if (video.mediaKeys) return false;
  const r = video.getBoundingClientRect();
  return r.width >= MIN_WIDTH && r.height >= MIN_HEIGHT;
}

function webSource(video: HTMLVideoElement): string | undefined {
  const src = video.currentSrc;
  return /^https?:\/\//i.test(src) && src.length <= 8192 ? src : undefined;
}

function start(): void {
  const ui = build();
  let video: HTMLVideoElement | null = null;
  let visibleUntil = 0;
  let frame = 0;
  let lastProbe = 0;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

  const shown = () => ui.host.isConnected;
  const busy = () => !ui.menu.hidden || ui.main.disabled || !ui.note.hidden;

  function hide(): void {
    cancelAnimationFrame(frame);
    ui.menu.hidden = true;
    ui.host.remove();
  }

  function place(): boolean {
    if (!video || !eligible(video)) return false;
    const r = video.getBoundingClientRect();
    if (r.bottom <= 0 || r.top >= innerHeight || r.right <= 0 || r.left >= innerWidth) return false;
    ui.wrap.style.top = `${Math.max(8, r.top + 8)}px`;
    ui.wrap.style.left = `${Math.min(innerWidth - 8, r.right - 8)}px`;
    return true;
  }

  function tick(): void {
    if (!place() || (Date.now() > visibleUntil && !busy())) {
      hide();
      return;
    }
    frame = requestAnimationFrame(tick);
  }

  function show(target: HTMLVideoElement, forMs: number): void {
    if (video !== target) {
      ui.menu.hidden = true;
      ui.note.hidden = true;
    }
    video = target;
    visibleUntil = Math.max(visibleUntil, Date.now() + forMs);
    if (!shown()) {
      document.documentElement.append(ui.host);
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(tick);
    }
  }

  function noteText(text: string, tone: "muted" | "danger"): void {
    ui.menu.hidden = true;
    ui.note.textContent = text;
    ui.note.dataset.tone = tone;
    ui.note.hidden = false;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => {
      ui.note.hidden = true;
      visibleUntil = Date.now() + LINGER_MS;
    }, NOTE_MS);
  }

  async function send(item: OverlayItem): Promise<void> {
    ui.menu.hidden = true;
    ui.main.disabled = true;
    try {
      // The page entry (YouTube) sends the page's address, not a detected file.
      const message =
        item.mediaId === PAGE_ITEM_ID ? { kind: "overlay_page" } : { kind: "overlay_send", mediaId: item.mediaId };
      const reply: unknown = await ext.runtime.sendMessage(message);
      const result = overlayResultText(reply);
      noteText(result.text, result.failed ? "danger" : "muted");
    } catch {
      noteText("Downpipe's extension did not answer. Reload the page and try again.", "danger");
    } finally {
      ui.main.disabled = false;
    }
  }

  function openMenu(items: OverlayItem[]): void {
    const rows = items.map((item) => {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "item";
      row.setAttribute("role", "menuitem");
      const label = document.createElement("span");
      label.className = "label";
      label.textContent = item.label;
      label.title = item.label;
      const detail = document.createElement("span");
      detail.className = "detail";
      detail.textContent = item.detail;
      row.append(label, detail);
      row.addEventListener("click", (e) => {
        if (e.isTrusted) void send(item);
      });
      return row;
    });
    ui.note.hidden = true;
    ui.menu.replaceChildren(...rows);
    ui.menu.hidden = false;
    rows[0]?.focus();
  }

  ui.main.addEventListener("click", (e) => {
    // A page script can click elements, but not with a trusted event.
    if (!e.isTrusted || !video) return;
    if (!ui.menu.hidden) {
      ui.menu.hidden = true;
      return;
    }
    const src = webSource(video);
    ui.main.disabled = true;
    void ext.runtime
      .sendMessage(src ? { kind: "overlay_list", src } : { kind: "overlay_list" })
      .then((reply: unknown) => {
        ui.main.disabled = false;
        const items = isOverlayItems(reply) ? reply.items : null;
        if (!items) noteText("Downpipe's extension did not answer. Reload the page and try again.", "danger");
        else if (items.length === 0) noteText("Nothing to download yet. Let the video play a moment, then try again.", "muted");
        else if (items.length === 1 && items[0]) void send(items[0]);
        else openMenu(items);
      })
      .catch(() => {
        ui.main.disabled = false;
        noteText("Downpipe's extension did not answer. Reload the page and try again.", "danger");
      });
  });

  ui.wrap.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !ui.menu.hidden) {
      ui.menu.hidden = true;
      ui.main.focus();
    }
  });

  document.addEventListener(
    "playing",
    (e) => {
      if (e.target instanceof HTMLVideoElement && eligible(e.target)) show(e.target, AFTER_PLAY_MS);
    },
    true,
  );

  document.addEventListener(
    "pointermove",
    (e) => {
      if (video && shown()) {
        const r = video.getBoundingClientRect();
        if (e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom) {
          visibleUntil = Date.now() + LINGER_MS;
          return;
        }
      }
      // A video that was already playing when the page was opened never
      // fires `playing` for this script; find it under the pointer instead.
      const now = Date.now();
      if (now - lastProbe < PROBE_MS) return;
      lastProbe = now;
      const under = document
        .elementsFromPoint(e.clientX, e.clientY)
        .find((el): el is HTMLVideoElement => el instanceof HTMLVideoElement);
      if (under && !under.paused && eligible(under)) show(under, LINGER_MS);
    },
    { capture: true, passive: true },
  );

  document.addEventListener(
    "pointerdown",
    (e) => {
      // A click anywhere else closes the menu; clicks inside the shadow root
      // are retargeted to the host.
      if (!ui.menu.hidden && e.target !== ui.host) ui.menu.hidden = true;
    },
    true,
  );
}

const flag = globalThis as { __downpipeOverlay?: boolean };
if (!flag.__downpipeOverlay) {
  flag.__downpipeOverlay = true;
  start();
}
