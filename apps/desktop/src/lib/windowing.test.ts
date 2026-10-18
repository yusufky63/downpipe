import { describe, expect, it } from "vitest";

import { scrollToRow, visibleRange, type Viewport } from "./windowing";

const view = (scrollTop: number): Viewport => ({ scrollTop, height: 600, offset: 40, rowHeight: 60 });

describe("visibleRange", () => {
  it("draws the rows in view and a few around them", () => {
    expect(visibleRange(view(0), 5000, 2)).toEqual({ start: 0, end: 12 });
    // Row 100 starts at 40 + 6000.
    expect(visibleRange(view(6040), 5000, 2)).toEqual({ start: 98, end: 112 });
    expect(visibleRange(view(5000 * 60), 5000, 2)).toEqual({ start: 4997, end: 5000 });
  });

  it("copes with short lists and rows not measured yet", () => {
    expect(visibleRange(view(0), 3, 2)).toEqual({ start: 0, end: 3 });
    expect(visibleRange({ ...view(0), rowHeight: 0 }, 5000, 8)).toEqual({ start: 0, end: 8 });
    expect(visibleRange(view(0), 0)).toEqual({ start: 0, end: 0 });
  });
});

describe("scrollToRow", () => {
  it("scrolls a row above the view to just below the header", () => {
    // Row 10 starts at 640; the header is 40 tall.
    expect(scrollToRow(view(2000), 10, 40)).toBe(600);
  });

  it("scrolls a row below the view up to the bottom edge", () => {
    // Row 30 ends at 40 + 31 * 60 = 1900.
    expect(scrollToRow(view(0), 30, 40)).toBe(1300);
  });

  it("leaves a row that is already in view", () => {
    expect(scrollToRow(view(0), 2, 40)).toBeNull();
    expect(scrollToRow(view(0), 0, 40)).toBeNull();
  });
});
