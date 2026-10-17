// Small DOM helpers shared by the popup, the options page and the links
// page. Text always goes in through `textContent`, never as HTML: link
// texts and host messages are untrusted.
import { ext } from "../lib/browser";
import type { StatusView } from "../lib/view";

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
