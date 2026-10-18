// Where the Download button sits over a video. Pure geometry, so it can be
// tested without a page: the overlay script measures, these decide.

/** A rectangle in viewport pixels, like `getBoundingClientRect()`. */
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * The button's top right corner as fractions of the video's width and
 * height. Fractions rather than pixels, so the place carries over to the
 * same site's other videos and to a resized or full-screen player.
 */
export interface Place {
  x: number;
  y: number;
}

/** Gap kept between the button and the video's (and the window's) edges. */
export const INSET = 8;

const clamp = (value: number, low: number, high: number) => Math.min(Math.max(value, low), high);

/**
 * The button's top right corner in viewport pixels: the dragged place, or
 * the video's top right corner, kept inside the video and the window.
 */
export function clampCorner(
  video: Box,
  place: Place | null,
  bar: { width: number; height: number },
  viewport: { width: number; height: number },
): { x: number; y: number } {
  const right = video.left + video.width;
  const bottom = video.top + video.height;
  let x = place ? video.left + place.x * video.width : right - INSET;
  let y = place ? video.top + place.y * video.height : video.top + INSET;
  x = clamp(x, video.left + bar.width + INSET, right - INSET);
  y = clamp(y, video.top + INSET, bottom - bar.height - INSET);
  // A video taller or wider than the window still shows the button.
  x = clamp(x, bar.width + INSET, viewport.width - INSET);
  y = clamp(y, INSET, viewport.height - bar.height - INSET);
  return { x, y };
}

/** A corner in viewport pixels as the fractions `Place` stores. */
export function cornerToPlace(video: Box, corner: { x: number; y: number }): Place {
  const fraction = (offset: number, size: number) => (size > 0 ? clamp(offset / size, 0, 1) : 0);
  return { x: fraction(corner.x - video.left, video.width), y: fraction(corner.y - video.top, video.height) };
}
