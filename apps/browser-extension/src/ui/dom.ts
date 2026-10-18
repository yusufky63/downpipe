// Small DOM helpers shared by the popup, the options page and the links
// page. Text always goes in through `textContent`, never as HTML: link
// texts, host messages and translations are untrusted.
import { activateBrowserLanguage, t, type MessageKey } from "../i18n";
import { ext } from "../lib/browser";
import type { StatusView } from "../lib/view";

/** `data-i18n-*` attributes and the element attribute each one fills. */
const ATTRIBUTES = [
  ["data-i18n-title", "title"],
  ["data-i18n-aria-label", "aria-label"],
  ["data-i18n-placeholder", "placeholder"],
] as const;

/**
 * Fill an element from its text. Children marked `data-var="name"` (code
 * samples) stay in the markup and are put where the text says `{name}`, so
 * a translation can move them.
 */
function fillText(el: HTMLElement, key: string): void {
  const text = t(key as MessageKey);
  const slots = new Map<string, Element>();
  for (const child of el.querySelectorAll<HTMLElement>("[data-var]")) {
    if (child.dataset.var) slots.set(child.dataset.var, child);
  }
  if (!slots.size) {
    el.textContent = text;
    return;
  }
  // split() with a capture group: names land at the odd indexes.
  const parts = text.split(/\{(\w+)\}/);
  el.replaceChildren(
    ...parts.map((part, i) => (i % 2 ? (slots.get(part) ?? document.createTextNode(`{${part}}`)) : document.createTextNode(part))),
  );
}

/**
 * Start an extension page in the browser's language: the texts of the
 * page's `data-i18n` elements and attributes, and its `lang`. The pages'
 * HTML holds keys, not English, so every language goes through here.
 */
export function startPage(): void {
  document.documentElement.lang = activateBrowserLanguage();
  for (const el of document.querySelectorAll<HTMLElement>("[data-i18n]")) {
    if (el.dataset.i18n) fillText(el, el.dataset.i18n);
  }
  for (const [data, attribute] of ATTRIBUTES) {
    for (const el of document.querySelectorAll(`[${data}]`)) {
      const key = el.getAttribute(data);
      if (key) el.setAttribute(attribute, t(key as MessageKey));
    }
  }
}

export function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} is missing from the page`);
  return el as T;
}

export function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

/** The bridge status box (`.status` in base.css) and its actions. */
export interface StatusElements {
  box: HTMLElement;
  title: HTMLElement;
  detail: HTMLElement;
  command: HTMLElement;
  guide: HTMLAnchorElement;
}

export function paintStatus(els: StatusElements, view: StatusView): void {
  els.box.dataset.tone = view.tone;
  els.title.textContent = view.title;
  els.detail.textContent = view.detail;
  els.detail.hidden = !view.detail;
  els.command.textContent = view.command ?? "";
  els.command.hidden = !view.command;
  els.guide.hidden = !view.guide;
}

/** What `statusView` needs to word its commands for this browser. */
export function extensionInfo(): { id: string; firefox: boolean } {
  return { id: ext.runtime.id, firefox: ext.runtime.getURL("").startsWith("moz-extension:") };
}
