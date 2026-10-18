// Pure helpers of the Feeds dialog.
import type { Feed, FeedInput } from "./api";
import { MAIN_QUEUE } from "./api";
import { parseExtensions } from "./grabber";

/** Check intervals offered, in minutes. */
export const INTERVALS = [15, 30, 60, 180, 360, 720, 1440] as const;

export const NEW_FEED: FeedInput = {
  url: "",
  filter: { extensions: [], title_pattern: "" },
  interval_minutes: 60,
  queue: MAIN_QUEUE,
  enabled: true,
  download_existing: false,
};

/** The form's fields for an existing feed. */
export function inputOf(feed: Feed): FeedInput {
  return {
    url: feed.url,
    filter: { extensions: [...feed.filter.extensions], title_pattern: feed.filter.title_pattern },
    interval_minutes: feed.interval_minutes,
    queue: feed.queue,
    enabled: feed.enabled,
    download_existing: false,
  };
}

/** The form's file types field as the filter wants them. */
export function withTypes(input: FeedInput, typed: string): FeedInput {
  return { ...input, filter: { ...input.filter, extensions: parseExtensions(typed) } };
}

/** The feed's title, or its host while it has none. */
export function feedName(feed: Feed): string {
  if (feed.title.trim() !== "") return feed.title.trim();
  try {
    return new URL(feed.url).host;
  } catch {
    return feed.url;
  }
}

/** An interval in the largest whole unit: `{ value: 3, unit: "hours" }`. */
export function intervalParts(minutes: number): { value: number; unit: "minutes" | "hours" | "days" } {
  if (minutes % 1440 === 0) return { value: minutes / 1440, unit: "days" };
  if (minutes % 60 === 0) return { value: minutes / 60, unit: "hours" };
  return { value: minutes, unit: "minutes" };
}
