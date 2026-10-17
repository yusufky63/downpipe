import { describe, expect, it } from "vitest";

import { EMPTY_SELECTION, clickSelect, contextSelect, edge, focusOf, prune, selectAll, step, type Selection } from "./selection";

const order = ["a", "b", "c", "d", "e"];
const plain = { ctrl: false, shift: false };
const ctrl = { ctrl: true, shift: false };
const shift = { ctrl: false, shift: true };

describe("clickSelect", () => {
  it("selects one row on a plain click", () => {
    const sel = clickSelect({ ids: ["a", "b"], anchor: "a" }, "c", order, plain);
    expect(sel).toEqual({ ids: ["c"], anchor: "c" });
  });

  it("toggles with Ctrl and moves the anchor", () => {
    let sel = clickSelect(EMPTY_SELECTION, "b", order, plain);
    sel = clickSelect(sel, "d", order, ctrl);
    expect(sel).toEqual({ ids: ["b", "d"], anchor: "d" });
    sel = clickSelect(sel, "b", order, ctrl);
    expect(sel).toEqual({ ids: ["d"], anchor: "b" });
  });

  it("selects a range with Shift, in either direction", () => {
    const start = clickSelect(EMPTY_SELECTION, "b", order, plain);
    expect(clickSelect(start, "d", order, shift).ids).toEqual(["b", "c", "d"]);
    const back = clickSelect(clickSelect(start, "d", order, shift), "a", order, shift);
    expect(back).toEqual({ ids: ["a", "b"], anchor: "b" });
  });

  it("adds a range to the selection with Ctrl+Shift", () => {
    const sel = clickSelect({ ids: ["e"], anchor: "a" }, "b", order, { ctrl: true, shift: true });
    expect(sel.ids).toEqual(["e", "a", "b"]);
  });

  it("treats Shift without a visible anchor as a plain click", () => {
    expect(clickSelect({ ids: ["x"], anchor: "x" }, "c", order, shift)).toEqual({ ids: ["c"], anchor: "c" });
  });
});

describe("keyboard", () => {
  it("moves with the arrows and stops at the ends", () => {
    expect(step(EMPTY_SELECTION, order, 1, false).ids).toEqual(["a"]);
    expect(step(EMPTY_SELECTION, order, -1, false).ids).toEqual(["e"]);
    expect(step({ ids: ["d"], anchor: "d" }, order, 1, false).ids).toEqual(["e"]);
    expect(step({ ids: ["e"], anchor: "e" }, order, 1, false).ids).toEqual(["e"]);
  });

  it("grows and shrinks a range with Shift", () => {
    let sel: Selection = { ids: ["b"], anchor: "b" };
    sel = step(sel, order, 1, true);
    sel = step(sel, order, 1, true);
    expect(sel).toEqual({ ids: ["b", "c", "d"], anchor: "b" });
    expect(focusOf(sel, order)).toBe("d");
    sel = step(sel, order, -1, true);
    expect(sel.ids).toEqual(["b", "c"]);
  });

  it("jumps to the first and last row", () => {
    expect(edge({ ids: ["c"], anchor: "c" }, order, true, false).ids).toEqual(["e"]);
    expect(edge({ ids: ["c"], anchor: "c" }, order, false, true).ids).toEqual(["a", "b", "c"]);
  });

  it("selects everything shown", () => {
    expect(selectAll(order)).toEqual({ ids: order, anchor: "a" });
    expect(selectAll([])).toEqual(EMPTY_SELECTION);
  });
});

describe("context menu and removal", () => {
  it("keeps a selection that contains the clicked row", () => {
    const sel = { ids: ["a", "b"], anchor: "a" };
    expect(contextSelect(sel, "b")).toBe(sel);
    expect(contextSelect(sel, "c")).toEqual({ ids: ["c"], anchor: "c" });
  });

  it("forgets rows that were removed", () => {
    const sel = { ids: ["a", "b", "c"], anchor: "b" };
    expect(prune(sel, new Set(["a", "c"]))).toEqual({ ids: ["a", "c"], anchor: null });
    expect(prune(sel, new Set(order))).toBe(sel);
  });
});
