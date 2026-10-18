// The Download button over playing videos. The service worker registers
// this content script only while media detection is on. It sends nothing by
// itself: a click by the user asks the service worker for this tab's media
// (detected from the page's network requests), and sending one is a second
// request, again only after a click. The page cannot see into the button
// (a closed shadow root) and synthetic clicks from page scripts are ignored.
//
// Buttons like this are a common complaint about download managers because
// they cover the video's own controls, so this one is a small icon that the
// user can drag anywhere inside the video (remembered per site), can keep
// to pointer hovers only, and can hide on a site from its own menu.
import { t } from "../i18n/core";
import { ext } from "../lib/browser";
import {
  PAGE_ITEM_ID,
  isOverlayItems,
  isOverlayPrefs,
  overlayResultText,
  type OverlayItem,
  type OverlayPrefs,
} from "../lib/messages";
import { clampCorner, cornerToPlace, type Box, type Place } from "./place";
import { activateOverlayLanguage } from "./texts";

/** Thumbnails and decorative loops stay without a button. */
const MIN_WIDTH = 200;
const MIN_HEIGHT = 112;
/** The button stays this long after the pointer left the video. */
const LINGER_MS = 1500;
/** And this long after a video starts playing, so it is noticed (unless hover only). */
const AFTER_PLAY_MS = 4000;
const NOTE_MS = 4000;
/** How often the pointer may trigger a look for a video under it. */
const PROBE_MS = 250;
/** A press that moves less than this is a click, not a drag. */
const DRAG_THRESHOLD = 4;
/** Room the menu needs beside and below the button before it opens the other way. */
const MENU_WIDTH = 240;
const MENU_HEIGHT = 250;

const STYLE = `
:host { all: initial; }
.wrap {
  position: fixed; top: 0; left: 0; display: flex; flex-direction: column; align-items: flex-end; gap: 6px;
  max-width: 320px; pointer-events: auto;
  font: 500 13px/1.4 "DM Sans", system-ui, sans-serif; color: #0a0b0d;
  -webkit-font-smoothing: antialiased;
}
.wrap[data-side="left"] { align-items: flex-start; }
.wrap[data-up="true"] { flex-direction: column-reverse; }
button { font: inherit; cursor: pointer; }
.bar {
  display: inline-flex; height: 28px; border: 1px solid #075e5a; border-radius: 6px;
  background: #0b7a75; color: #fff; overflow: hidden; touch-action: none;
}
.bar[data-dragging="true"], .bar[data-dragging="true"] button { cursor: grabbing; }
.main, .more {
  display: inline-flex; align-items: center; justify-content: center; height: 100%;
  padding: 0; border: 0; background: none; color: inherit;
}
.main { width: 28px; cursor: grab; }
.more { width: 16px; border-left: 1px solid #075e5a; }
.main:hover, .more:hover, .more[aria-expanded="true"] { background: #075e5a; }
.main:focus-visible, .more:focus-visible, .item:focus-visible { outline: 2px solid #2bb3a6; outline-offset: -2px; }
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
  .main, .more, .item { transition: background-color 120ms cubic-bezier(0.2, 0, 0, 1); }
}
`;

const SVG = "http://www.w3.org/2000/svg";

/** A stroked icon, drawn with DOM calls so no HTML parsing is involved. */
function icon(size: number, paths: readonly string[]): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  for (const [k, v] of Object.entries({
    width: String(size),
    height: String(size),
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
  for (const d of paths) {
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

const DOWNLOAD_ICON = ["M12 4v12", "m6 10 6 6 6-6", "M5 20h14"];
const CHEVRON_ICON = ["m6 9 6 6 6-6"];

interface Ui {
  host: HTMLElement;
  wrap: HTMLElement;
  bar: HTMLElement;
  main: HTMLButtonElement;
  more: HTMLButtonElement;
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
  const bar = document.createElement("div");
  bar.className = "bar";
  const main = document.createElement("button");
  main.type = "button";
  main.className = "main";
  main.setAttribute("aria-haspopup", "menu");
  main.setAttribute("aria-label", t("overlay.buttonTitle"));
  main.title = `${t("overlay.buttonTitle")}\n${t("overlay.dragHint")}`;
  main.append(icon(14, DOWNLOAD_ICON));
  const more = document.createElement("button");
  more.type = "button";
  more.className = "more";
  more.setAttribute("aria-haspopup", "menu");
  more.setAttribute("aria-expanded", "false");
  more.setAttribute("aria-label", t("overlay.more"));
  more.title = t("overlay.more");
  more.append(icon(12, CHEVRON_ICON));
  bar.append(main, more);
  const menu = document.createElement("div");
  menu.className = "menu";
  menu.setAttribute("role", "menu");
  menu.hidden = true;
  const note = document.createElement("div");
  note.className = "note";
  note.setAttribute("role", "status");
  note.hidden = true;
  wrap.append(bar, menu, note);
  root.append(style, wrap);
  return { host, wrap, bar, main, more, menu, note };
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

/** The poster image's address, which tells this video's streams from the page's others. */
function webPoster(video: HTMLVideoElement): string | undefined {
  const poster = video.poster;
  return /^https?:\/\//i.test(poster) && poster.length <= 8192 ? poster : undefined;
}

function boxOf(r: DOMRect): Box {
  return { left: r.left, top: r.top, width: r.width, height: r.height };
}

function start(prefs: OverlayPrefs): void {
  const ui = build();
  let video: HTMLVideoElement | null = null;
  let visibleUntil = 0;
  let frame = 0;
  let lastProbe = 0;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  let place: Place | null = prefs.place ?? null;
  /** "Hide on this site" was chosen: nothing shows on this page any more. */
  let retired = false;
  /** A press on the bar, while it may still turn into a drag. */
  let press: { id: number; x: number; y: number; corner: { x: number; y: number }; moved: boolean } | null = null;
  /** The click that ends a drag is not a click on the button. */
  let swallowClick = false;
  let corner = { x: 0, y: 0 };

  const shown = () => ui.host.isConnected;
  const busy = () => !ui.menu.hidden || ui.main.disabled || !ui.note.hidden || press !== null;

  function closeMenu(): void {
    ui.menu.hidden = true;
    ui.more.setAttribute("aria-expanded", "false");
  }

  function hide(): void {
    cancelAnimationFrame(frame);
    closeMenu();
    ui.host.remove();
  }

  function barSize(): { width: number; height: number } {
    return { width: ui.bar.offsetWidth || 46, height: ui.bar.offsetHeight || 28 };
  }

  function position(): boolean {
    if (!video || !eligible(video)) return false;
    const r = video.getBoundingClientRect();
    if (r.bottom <= 0 || r.top >= innerHeight || r.right <= 0 || r.left >= innerWidth) return false;
    const bar = barSize();
    if (!press?.moved) corner = clampCorner(boxOf(r), place, bar, { width: innerWidth, height: innerHeight });
    // The menu opens toward the side with room.
    const left = corner.x - MENU_WIDTH < 0;
    const up = corner.y + bar.height + MENU_HEIGHT > innerHeight;
    ui.wrap.dataset.side = left ? "left" : "right";
    ui.wrap.dataset.up = String(up);
    ui.wrap.style.left = `${left ? corner.x - bar.width : corner.x}px`;
    ui.wrap.style.top = `${up ? corner.y + bar.height : corner.y}px`;
    ui.wrap.style.transform = `${left ? "" : "translateX(-100%)"} ${up ? "translateY(-100%)" : ""}`.trim() || "none";
    return true;
  }

  function tick(): void {
    if (retired || !position() || (Date.now() > visibleUntil && !busy())) {
      hide();
      return;
    }
    frame = requestAnimationFrame(tick);
  }

  function show(target: HTMLVideoElement, forMs: number): void {
    if (retired) return;
    if (video !== target) {
      closeMenu();
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

  function noteText(text: string, tone: "muted" | "danger", then?: () => void): void {
    closeMenu();
    ui.note.textContent = text;
    ui.note.dataset.tone = tone;
    ui.note.hidden = false;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => {
      ui.note.hidden = true;
      visibleUntil = Date.now() + LINGER_MS;
      then?.();
    }, NOTE_MS);
  }

  async function send(item: OverlayItem): Promise<void> {
    closeMenu();
    ui.main.disabled = true;
    try {
      // The page entry (YouTube) sends the page's address, not a detected file.
      const message =
        item.mediaId === PAGE_ITEM_ID ? { kind: "overlay_page" } : { kind: "overlay_send", mediaId: item.mediaId };
      const reply: unknown = await ext.runtime.sendMessage(message);
      const result = overlayResultText(reply);
      noteText(result.text, result.failed ? "danger" : "muted");
    } catch {
      noteText(t("overlay.noAnswer"), "danger");
    } finally {
      ui.main.disabled = false;
    }
  }

  function menuRow(label: string, detail: string | null, onChoose: () => void): HTMLButtonElement {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "item";
    row.setAttribute("role", "menuitem");
    const text = document.createElement("span");
    text.className = "label";
    text.textContent = label;
    text.title = label;
    row.append(text);
    if (detail) {
      const small = document.createElement("span");
      small.className = "detail";
      small.textContent = detail;
      row.append(small);
    }
    row.addEventListener("click", (e) => {
      if (e.isTrusted) onChoose();
    });
    return row;
  }

  function openMenu(rows: HTMLButtonElement[]): void {
    ui.note.hidden = true;
    ui.menu.replaceChildren(...rows);
    ui.menu.hidden = false;
    rows[0]?.focus();
  }

  async function hideOnSite(): Promise<void> {
    closeMenu();
    const reply: unknown = await ext.runtime.sendMessage({ kind: "overlay_hide_site" }).catch(() => null);
    const ok = typeof reply === "object" && reply !== null && (reply as { ok?: unknown }).ok === true;
    if (!ok) {
      noteText(t("overlay.noAnswer"), "danger");
      return;
    }
    // Say where it can be undone, then leave the page alone for good.
    noteText(t("overlay.hidden"), "muted", () => {
      retired = true;
      hide();
    });
  }

  ui.main.addEventListener("click", (e) => {
    // A page script can click elements, but not with a trusted event.
    if (!e.isTrusted || !video || swallowClick) return;
    if (!ui.menu.hidden) {
      closeMenu();
      return;
    }
    const src = webSource(video);
    const poster = webPoster(video);
    ui.main.disabled = true;
    void ext.runtime
      .sendMessage({ kind: "overlay_list", ...(src ? { src } : {}), ...(poster ? { poster } : {}) })
      .then((reply: unknown) => {
        ui.main.disabled = false;
        const items = isOverlayItems(reply) ? reply.items : null;
        if (!items) noteText(t("overlay.noAnswer"), "danger");
        else if (items.length === 0) noteText(t("overlay.nothingYet"), "muted");
        else if (items.length === 1 && items[0]) void send(items[0]);
        else openMenu(items.map((item) => menuRow(item.label, item.detail, () => void send(item))));
      })
      .catch(() => {
        ui.main.disabled = false;
        noteText(t("overlay.noAnswer"), "danger");
      });
  });

  ui.more.addEventListener("click", (e) => {
    if (!e.isTrusted || swallowClick) return;
    if (!ui.menu.hidden) {
      closeMenu();
      return;
    }
    openMenu([menuRow(t("overlay.hideSite"), t("overlay.hideSiteDetail"), () => void hideOnSite())]);
    ui.more.setAttribute("aria-expanded", "true");
  });

  // Dragging: a press on the bar that moves past the threshold carries the
  // button with the pointer, inside the video; releasing it keeps the place
  // for this site. Untrusted presses are ignored like untrusted clicks.
  ui.bar.addEventListener("pointerdown", (e) => {
    if (!e.isTrusted || e.button !== 0 || !video) return;
    press = { id: e.pointerId, x: e.clientX, y: e.clientY, corner: { ...corner }, moved: false };
    (e.target as Element).setPointerCapture?.(e.pointerId);
  });

  ui.bar.addEventListener("pointermove", (e) => {
    if (!press || e.pointerId !== press.id || !video) return;
    const dx = e.clientX - press.x;
    const dy = e.clientY - press.y;
    if (!press.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    if (!press.moved) {
      press.moved = true;
      closeMenu();
      ui.bar.dataset.dragging = "true";
    }
    const r = boxOf(video.getBoundingClientRect());
    const wanted = cornerToPlace(r, { x: press.corner.x + dx, y: press.corner.y + dy });
    corner = clampCorner(r, wanted, barSize(), { width: innerWidth, height: innerHeight });
    visibleUntil = Date.now() + LINGER_MS;
  });

  const endPress = (e: PointerEvent) => {
    if (!press || e.pointerId !== press.id) return;
    const moved = press.moved;
    press = null;
    delete ui.bar.dataset.dragging;
    if (!moved || !video) return;
    place = cornerToPlace(boxOf(video.getBoundingClientRect()), corner);
    swallowClick = true;
    setTimeout(() => {
      swallowClick = false;
    }, 0);
    void ext.runtime.sendMessage({ kind: "overlay_place", ...place }).catch(() => undefined);
  };
  ui.bar.addEventListener("pointerup", endPress);
  ui.bar.addEventListener("pointercancel", endPress);

  ui.wrap.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !ui.menu.hidden) {
      const fromMore = ui.more.getAttribute("aria-expanded") === "true";
      closeMenu();
      (fromMore ? ui.more : ui.main).focus();
    }
  });

  document.addEventListener(
    "playing",
    (e) => {
      // Hover only: a video starting is no reason to cover it.
      if (prefs.hoverOnly) return;
      if (e.target instanceof HTMLVideoElement && eligible(e.target)) show(e.target, AFTER_PLAY_MS);
    },
    true,
  );

  document.addEventListener(
    "pointermove",
    (e) => {
      if (retired) return;
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
      if (!ui.menu.hidden && e.target !== ui.host) closeMenu();
    },
    true,
  );
}

const DEFAULT_PREFS: OverlayPrefs = { hoverOnly: false, hidden: false };

const flag = globalThis as { __downpipeOverlay?: boolean };
if (!flag.__downpipeOverlay) {
  flag.__downpipeOverlay = true;
  // The page's own `lang` is left alone: the button speaks the browser's language.
  activateOverlayLanguage();
  // The answer decides whether the button runs here at all, so nothing is
  // shown before it. A service worker that does not answer leaves the defaults.
  void ext.runtime
    .sendMessage({ kind: "overlay_prefs" })
    .then((reply: unknown) => (isOverlayPrefs(reply) ? reply : DEFAULT_PREFS))
    .catch(() => DEFAULT_PREFS)
    .then((prefs) => {
      if (!prefs.hidden) start(prefs);
    });
}
