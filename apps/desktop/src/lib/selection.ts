// Row selection as in a file manager: click selects one, Ctrl toggles,
// Shift selects a range from the anchor, arrow keys move.

export interface Selection {
  ids: string[];
  /** Where a Shift range starts. */
  anchor: string | null;
}

export const EMPTY_SELECTION: Selection = { ids: [], anchor: null };

export interface Modifiers {
  ctrl: boolean;
  shift: boolean;
}

/** `order` is the rows as shown, top to bottom. */
export function clickSelect(sel: Selection, id: string, order: readonly string[], mods: Modifiers): Selection {
  if (mods.shift && sel.anchor !== null && order.includes(sel.anchor)) {
    const range = between(order, sel.anchor, id);
    const ids = mods.ctrl ? union(sel.ids, range) : range;
    return { ids, anchor: sel.anchor };
  }
  if (mods.ctrl) {
    const ids = sel.ids.includes(id) ? sel.ids.filter((x) => x !== id) : [...sel.ids, id];
    return { ids, anchor: id };
  }
  return { ids: [id], anchor: id };
}

/**
 * Right click keeps a selection that contains the row, so the menu acts on
 * all of it; on any other row it selects just that row.
 */
export function contextSelect(sel: Selection, id: string): Selection {
  return sel.ids.includes(id) ? sel : { ids: [id], anchor: id };
}

export function selectAll(order: readonly string[]): Selection {
  return { ids: [...order], anchor: order[0] ?? null };
}

/** Arrow keys: move by `delta` rows; with Shift, grow or shrink the range. */
export function step(sel: Selection, order: readonly string[], delta: number, extend: boolean): Selection {
  if (order.length === 0) return EMPTY_SELECTION;
  const focus = focusOf(sel, order);
  const from = focus === null ? (delta > 0 ? -1 : order.length) : order.indexOf(focus);
  const to = order[Math.min(order.length - 1, Math.max(0, from + delta))] as string;
  if (extend && sel.anchor !== null && order.includes(sel.anchor)) {
    return { ids: between(order, sel.anchor, to), anchor: sel.anchor };
  }
  return { ids: [to], anchor: to };
}

/** Home and End. */
export function edge(sel: Selection, order: readonly string[], end: boolean, extend: boolean): Selection {
  return step(sel, order, end ? order.length : -order.length, extend);
}

/** The row the keyboard is on: the end of the selection away from the anchor. */
export function focusOf(sel: Selection, order: readonly string[]): string | null {
  const shown = sel.ids.filter((id) => order.includes(id));
  if (shown.length === 0) return null;
  if (shown.length === 1 || sel.anchor === null) return shown[shown.length - 1] ?? null;
  const at = shown.map((id) => order.indexOf(id));
  const anchorAt = order.indexOf(sel.anchor);
  const lo = Math.min(...at);
  const hi = Math.max(...at);
  return order[anchorAt === lo ? hi : lo] ?? null;
}

/** Drop rows that no longer exist, keeping the order the user picked them in. */
export function prune(sel: Selection, existing: ReadonlySet<string>): Selection {
  const ids = sel.ids.filter((id) => existing.has(id));
  const anchor = sel.anchor !== null && existing.has(sel.anchor) ? sel.anchor : null;
  return ids.length === sel.ids.length && anchor === sel.anchor ? sel : { ids, anchor };
}

function between(order: readonly string[], a: string, b: string): string[] {
  const i = order.indexOf(a);
  const j = order.indexOf(b);
  if (i === -1 || j === -1) return [b];
  return order.slice(Math.min(i, j), Math.max(i, j) + 1);
}

function union(a: readonly string[], b: readonly string[]): string[] {
  const seen = new Set(a);
  return [...a, ...b.filter((x) => !seen.has(x))];
}
