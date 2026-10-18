import { describe, expect, it } from "vitest";

import type { Feed } from "./api";
import { NEW_FEED, feedName, inputOf, intervalParts, withTypes } from "./feeds";

const feed = (over: Partial<Feed> = {}): Feed => ({
  id: "f",
  url: "https://pod.test/feed.xml",
  title: "",
  filter: { extensions: ["mp3"], title_pattern: "ep" },
  interval_minutes: 180,
  queue: "main",
  enabled: true,
  created_at: 1,
  last_checked: null,
  last_error: null,
  last_added: 0,
  added_total: 0,
  ...over,
});

describe("feed helpers", () => {
  it("names a feed by its title, else its host", () => {
    expect(feedName(feed({ title: "  Cast " }))).toBe("Cast");
    expect(feedName(feed())).toBe("pod.test");
    expect(feedName(feed({ url: "not a url" }))).toBe("not a url");
  });

  it("turns a feed into form fields without the backlog switch", () => {
    const input = inputOf(feed());
    expect(input).toEqual({
      url: "https://pod.test/feed.xml",
      filter: { extensions: ["mp3"], title_pattern: "ep" },
      interval_minutes: 180,
      queue: "main",
      enabled: true,
      download_existing: false,
    });
  });

  it("reads the typed file types", () => {
    expect(withTypes(NEW_FEED, ".MP3, m4a").filter.extensions).toEqual(["mp3", "m4a"]);
    expect(NEW_FEED.filter.extensions).toEqual([]);
  });

  it("states intervals in whole units", () => {
    expect(intervalParts(15)).toEqual({ value: 15, unit: "minutes" });
    expect(intervalParts(180)).toEqual({ value: 3, unit: "hours" });
    expect(intervalParts(1440)).toEqual({ value: 1, unit: "days" });
  });
});
