// Keyboard handling that makes the window behave like an app, not a web
// page: browser keys do nothing, and the list has its own keys.

export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

const BROWSER_KEYS = new Set(["f3", "f5", "f7", "browserback", "browserforward", "browserrefresh", "browsersearch", "browserhome"]);
/** With Ctrl: reload, print, save, view source, downloads, history, find, open, bookmark, new window. */
const BROWSER_CTRL_KEYS = new Set(["r", "p", "s", "u", "j", "h", "g", "f", "o", "d", "n"]);
const DEVTOOLS_CTRL_SHIFT_KEYS = new Set(["i", "j", "c"]);

/** True for keys the web view would answer as a browser. `dev` keeps the developer tools reachable. */
export function isBrowserShortcut(e: KeyLike, dev: boolean): boolean {
  const key = e.key.toLowerCase();
  const mod = e.ctrlKey || e.metaKey;
  if (key === "f12" || (mod && e.shiftKey && DEVTOOLS_CTRL_SHIFT_KEYS.has(key))) return !dev;
  if (BROWSER_KEYS.has(key)) return true;
  if (e.altKey && !mod && (key === "arrowleft" || key === "arrowright" || key === "home")) return true;
  return mod && !e.altKey && BROWSER_CTRL_KEYS.has(key);
}

export type ListKey =
  | { kind: "properties" }
  | { kind: "remove" }
  | { kind: "copy_link" }
  | { kind: "select_all" }
  | { kind: "clear" }
  | { kind: "menu" }
  | { kind: "move"; to: "top" | "up" | "down" | "bottom" }
  | { kind: "step"; delta: number; extend: boolean }
  | { kind: "edge"; end: boolean; extend: boolean };

/** Rows a Page Up or Page Down moves. */
const PAGE = 10;

/** What a key does in the download list, if anything. */
export function listKey(e: KeyLike): ListKey | null {
  const mod = e.ctrlKey || e.metaKey;
  if (e.altKey) return null;
  switch (e.key) {
    case "Enter":
      return mod ? null : { kind: "properties" };
    case "Delete":
      return mod ? null : { kind: "remove" };
    case "Escape":
      return { kind: "clear" };
    case "ContextMenu":
      return { kind: "menu" };
    case "F10":
      return e.shiftKey ? { kind: "menu" } : null;
    case "ArrowUp":
      return mod ? { kind: "move", to: "up" } : { kind: "step", delta: -1, extend: e.shiftKey };
    case "ArrowDown":
      return mod ? { kind: "move", to: "down" } : { kind: "step", delta: 1, extend: e.shiftKey };
    case "PageUp":
      return mod ? null : { kind: "step", delta: -PAGE, extend: e.shiftKey };
    case "PageDown":
      return mod ? null : { kind: "step", delta: PAGE, extend: e.shiftKey };
    case "Home":
      return mod ? { kind: "move", to: "top" } : { kind: "edge", end: false, extend: e.shiftKey };
    case "End":
      return mod ? { kind: "move", to: "bottom" } : { kind: "edge", end: true, extend: e.shiftKey };
  }
  if (mod && !e.shiftKey) {
    const key = e.key.toLowerCase();
    if (key === "a") return { kind: "select_all" };
    if (key === "c") return { kind: "copy_link" };
  }
  return null;
}

/** Text fields keep their own keys, menu and selection. */
export function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return target.closest("input, textarea, select, [contenteditable]") !== null;
}
