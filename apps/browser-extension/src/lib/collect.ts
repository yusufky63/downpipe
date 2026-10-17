// Runs inside the web page through `scripting.executeScript`, which
// serializes the function's source and nothing else. It must therefore not
// use imports, module-level constants or helpers: everything it needs is
// written out inside it, and src/lib/collect.test.ts runs it from its
// source text to make sure.

/** What the page hands back. The service worker validates it again (`parseCollection`). */
export interface CollectedPage {
  pageUrl: string;
  title: string;
  links: { url: string; text: string }[];
  truncated: boolean;
}

/**
 * http(s) links from `a[href]` and `area[href]` with their visible text,
 * one entry per address (the fragment is ignored), capped at `max`. With
 * `onlySelection`, only links inside the current text selection, when
 * there is one.
 */
export function collectPageLinks(onlySelection: boolean, max: number): CollectedPage {
  // Characters kept in total, so a page full of 8 KiB links stays well
  // inside the 10 MB of `storage.session`.
  let budget = 4_000_000;
  const selection = onlySelection ? document.getSelection() : null;
  const inSelection = selection !== null && !selection.isCollapsed && selection.rangeCount > 0;
  let page = "";
  try {
    const here = new URL(location.href);
    here.hash = "";
    page = here.href;
  } catch {
    // An opaque page URL: nothing to compare against.
  }
  const index = new Map<string, number>();
  const links: { url: string; text: string }[] = [];
  let truncated = false;

  const textOf = (el: Element): string => {
    const own = "innerText" in el ? String((el as HTMLElement).innerText ?? "") : String(el.textContent ?? "");
    const alt = el.querySelector?.("img[alt]")?.getAttribute("alt") ?? "";
    const label = own.trim() || el.getAttribute("title") || el.getAttribute("aria-label") || alt || "";
    return label.replace(/\s+/g, " ").trim().slice(0, 200);
  };

  for (const el of Array.from(document.querySelectorAll("a[href], area[href]"))) {
    if (inSelection && !selection.containsNode(el, true)) continue;
    const raw = (el.getAttribute("href") ?? "").trim();
    // In-page anchors, not downloads.
    if (!raw || raw.startsWith("#")) continue;
    let url: URL;
    try {
      url = new URL(raw, document.baseURI);
    } catch {
      continue;
    }
    if (url.protocol !== "http:" && url.protocol !== "https:") continue;
    url.hash = "";
    const href = url.href;
    if (href === page) continue;
    const text = textOf(el);
    const seen = index.get(href);
    if (seen !== undefined) {
      const first = links[seen];
      if (first && !first.text && text) first.text = text;
      continue;
    }
    if (links.length >= max || href.length + text.length > budget) {
      truncated = true;
      break;
    }
    budget -= href.length + text.length;
    index.set(href, links.length);
    links.push({ url: href, text });
  }
  return { pageUrl: location.href, title: String(document.title ?? "").slice(0, 300), links, truncated };
}
