import { describe, expect, it } from "vitest";
import { INSET, clampCorner, cornerToPlace } from "./place";

const video = { left: 100, top: 50, width: 640, height: 360 };
const bar = { width: 46, height: 28 };
const viewport = { width: 1280, height: 800 };

describe("button place", () => {
  it("starts in the video's top right corner", () => {
    expect(clampCorner(video, null, bar, viewport)).toEqual({ x: 740 - INSET, y: 50 + INSET });
  });

  it("follows a dragged place and keeps the whole button inside the video", () => {
    expect(clampCorner(video, { x: 0.5, y: 0.5 }, bar, viewport)).toEqual({ x: 420, y: 230 });
    // Dragged past the bottom left: stops at the edges.
    expect(clampCorner(video, { x: 0, y: 1 }, bar, viewport)).toEqual({ x: 100 + bar.width + INSET, y: 410 - bar.height - INSET });
  });

  it("stays in the window when the video does not fit in it", () => {
    const tall = { left: 0, top: -400, width: 1280, height: 2000 };
    expect(clampCorner(tall, null, bar, viewport).y).toBe(INSET);
  });

  it("stores corners as fractions and reads them back", () => {
    const place = cornerToPlace(video, { x: 420, y: 230 });
    expect(place).toEqual({ x: 0.5, y: 0.5 });
    expect(clampCorner(video, place, bar, viewport)).toEqual({ x: 420, y: 230 });
    expect(cornerToPlace(video, { x: -50, y: 9999 })).toEqual({ x: 0, y: 1 });
    expect(cornerToPlace({ left: 0, top: 0, width: 0, height: 0 }, { x: 5, y: 5 })).toEqual({ x: 0, y: 0 });
  });
});
