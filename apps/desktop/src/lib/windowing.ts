// Which rows of the download list to draw. Only the rows in view (and a few
// around them) are in the page, so a list of thousands of downloads costs
// what a screenful does. Rows have one height (they never wrap), measured
// from a drawn row, so positions follow from the index alone.

export interface Viewport {
  /** The scrolling element's `scrollTop`. */
  scrollTop: number;
  /** Its visible height. */
  height: number;
  /** Where the first row starts inside the scrolled content (below the header). */
  offset: number;
  rowHeight: number;
}

/** Rows drawn beyond the visible ones on each side, so fast scrolling does not show gaps. */
export const OVERSCAN = 8;

/** The rows to draw, as `[start, end)`. */
export function visibleRange(view: Viewport, count: number, overscan = OVERSCAN): { start: number; end: number } {
  if (count <= 0 || view.rowHeight <= 0) return { start: 0, end: Math.min(count, overscan) };
  const top = view.scrollTop - view.offset;
  const first = Math.floor(top / view.rowHeight) - overscan;
  const last = Math.ceil((top + view.height) / view.rowHeight) + overscan;
  const start = Math.min(Math.max(first, 0), count);
  const end = Math.min(Math.max(last, start), count);
  return { start, end };
}

/**
 * The `scrollTop` that brings a row fully into view below the sticky header
 * (`header` pixels tall), or null when it already is. It moves as little as
 * it can, like the browser does when a focused element is scrolled into view.
 */
export function scrollToRow(view: Viewport, index: number, header: number): number | null {
  const rowTop = view.offset + index * view.rowHeight;
  const rowBottom = rowTop + view.rowHeight;
  if (rowTop < view.scrollTop + header) return Math.max(0, rowTop - header);
  if (rowBottom > view.scrollTop + view.height) return rowBottom - view.height;
  return null;
}
