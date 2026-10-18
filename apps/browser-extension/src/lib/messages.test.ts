import { describe, expect, it } from "vitest";
import { DEFAULT_CAPTURE_TYPES, defaultFilters } from "./filters";
import { defaultMediaFilters } from "./media";
import {
  MAX_ACTIVITY,
  PAGE_ITEM_ID,
  addActivity,
  isLinkResult,
  isMediaReply,
  isOverlayItems,
  isSettings,
  overlayResultText,
  parseActivity,
  parseOverlayRequest,
  parsePopupRequest,
  parseSettings,
  relativeTime,
  taskIdsOf,
  type Activity,
} from "./messages";

const entry = (at: number): Activity => ({ at, label: `f${at}.zip`, source: "link", outcome: "added", message: "Added" });

describe("parsePopupRequest", () => {
  it("accepts only its own requests, exactly", () => {
    expect(parsePopupRequest({ kind: "status" })).toEqual({ kind: "status" });
    expect(parsePopupRequest({ kind: "tasks" })).toEqual({ kind: "tasks" });
    expect(parsePopupRequest({ kind: "set_notify", enabled: false })).toEqual({ kind: "set_notify", enabled: false });
    expect(parsePopupRequest({ kind: "set_notify", enabled: "no" })).toBeNull();
    expect(parsePopupRequest({ kind: "set_notify", enabled: true, extra: 1 })).toBeNull();
    expect(parsePopupRequest({ kind: "tasks", task_ids: ["x"] })).toBeNull();
    expect(parsePopupRequest({ kind: "status", url: "https://x" })).toBeNull();
    expect(parsePopupRequest({ kind: "download", url: "https://x" })).toBeNull();
    expect(parsePopupRequest("status")).toBeNull();
    expect(parsePopupRequest(null)).toBeNull();
  });
});

describe("new requests", () => {
  const filters = { captureAll: false, captureTypes: ["zip"], excludedSites: ["*.example.com"], minSizeBytes: 0 };

  it("accepts the settings and bridge requests exactly", () => {
    expect(parsePopupRequest({ kind: "ping" })).toEqual({ kind: "ping" });
    expect(parsePopupRequest({ kind: "get_settings" })).toEqual({ kind: "get_settings" });
    expect(parsePopupRequest({ kind: "ping", x: 1 })).toBeNull();
    expect(parsePopupRequest({ kind: "get_settings", key: "settings" })).toBeNull();
  });

  it("saves only normalized filters", () => {
    expect(parsePopupRequest({ kind: "save_filters", filters })).toEqual({ kind: "save_filters", filters });
    expect(parsePopupRequest({ kind: "save_filters", filters, notify: false })).toBeNull();
    expect(parsePopupRequest({ kind: "save_filters", filters: { ...filters, captureTypes: ["ZIP"] } })).toBeNull();
    expect(parsePopupRequest({ kind: "save_filters", filters: { ...filters, notify: true } })).toBeNull();
    expect(parsePopupRequest({ kind: "save_filters" })).toBeNull();
  });

  it("pauses one site given in stored form", () => {
    expect(parsePopupRequest({ kind: "set_site_paused", site: "www.example.com", paused: true })).toEqual({
      kind: "set_site_paused",
      site: "www.example.com",
      paused: true,
    });
    expect(parsePopupRequest({ kind: "set_site_paused", site: "WWW.example.com", paused: true })).toBeNull();
    expect(parsePopupRequest({ kind: "set_site_paused", site: "https://example.com/", paused: true })).toBeNull();
    expect(parsePopupRequest({ kind: "set_site_paused", site: "a b", paused: true })).toBeNull();
    expect(parsePopupRequest({ kind: "set_site_paused", site: "example.com", paused: "yes" })).toBeNull();
    expect(parsePopupRequest({ kind: "set_site_paused", site: "example.com" })).toBeNull();
  });

  it("collects links of one tab by id", () => {
    expect(parsePopupRequest({ kind: "collect_links", tabId: 12 })).toEqual({ kind: "collect_links", tabId: 12 });
    for (const tabId of [-1, 1.5, "12", null, Number.MAX_SAFE_INTEGER + 1]) {
      expect(parsePopupRequest({ kind: "collect_links", tabId }), String(tabId)).toBeNull();
    }
    expect(parsePopupRequest({ kind: "collect_links", tabId: 1, frameId: 0 })).toBeNull();
  });

  it("sends only http(s) links, with an optional http(s) page", () => {
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip" })).toEqual({
      kind: "send_link",
      url: "https://x.org/a.zip",
    });
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip", pageUrl: "https://x.org/" })).toEqual({
      kind: "send_link",
      url: "https://x.org/a.zip",
      pageUrl: "https://x.org/",
    });
    expect(parsePopupRequest({ kind: "send_link", url: "javascript:alert(1)" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a\nb" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a\u0000" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip", pageUrl: "file:///C:/" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip", pageUrl: undefined })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip", referrer: "https://x.org/" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_link", url: "https://x.org/a.zip", cookies: "a=b" })).toBeNull();
    expect(parsePopupRequest([{ kind: "ping" }])).toBeNull();
  });
});

describe("set_start_app", () => {
  it("takes exactly one boolean", () => {
    expect(parsePopupRequest({ kind: "set_start_app", enabled: false })).toEqual({ kind: "set_start_app", enabled: false });
    expect(parsePopupRequest({ kind: "set_start_app", enabled: "no" })).toBeNull();
    expect(parsePopupRequest({ kind: "set_start_app" })).toBeNull();
    expect(parsePopupRequest({ kind: "set_start_app", enabled: true, path: "/usr/bin/x"})).toBeNull();
  });

  it("is on unless switched off", () => {
    expect(parseSettings({ startApp: false }).startApp).toBe(false);
    expect(parseSettings({ startApp: "off" }).startApp).toBe(true);
  });
});

describe("parseSettings", () => {
  it("notifies unless switched off", () => {
    expect(parseSettings(undefined)).toEqual({
      notify: true,
      startApp: true,
      ...defaultFilters(),
      ...defaultMediaFilters(),
    });
    expect(parseSettings({ notify: false }).notify).toBe(false);
    expect(parseSettings({ notify: "off" }).notify).toBe(true);
  });

  it("gives settings from before the filters the default filters", () => {
    const old = parseSettings({ notify: false });
    expect(old).toEqual({
      notify: false,
      startApp: true,
      captureAll: false,
      captureTypes: [...DEFAULT_CAPTURE_TYPES],
      excludedSites: [],
      minSizeBytes: 0,
      ...defaultMediaFilters(),
    });
    expect(isSettings(old)).toBe(true);
  });

  it("keeps saved filters", () => {
    const saved = {
      notify: true,
      startApp: false,
      captureAll: true,
      captureTypes: ["zip"],
      excludedSites: ["a.org"],
      minSizeBytes: 2048,
      mediaOverlay: false,
      mediaTypes: ["mp4"],
      mediaMinBytes: 0,
    };
    expect(parseSettings(saved)).toEqual(saved);
    expect(parseSettings(JSON.parse(JSON.stringify(saved)))).toEqual(saved);
  });

  it("checks replies that claim to be settings", () => {
    expect(isSettings(parseSettings(undefined))).toBe(true);
    expect(isSettings({ ...parseSettings(undefined), notify: "yes" })).toBe(false);
    expect(isSettings({ ...parseSettings(undefined), startApp: undefined })).toBe(false);
    expect(isSettings({ notify: true })).toBe(false);
    expect(isSettings({ ...parseSettings(undefined), mediaTypes: ["MP4"] })).toBe(false);
    expect(isSettings(null)).toBe(false);
  });
});

describe("isLinkResult", () => {
  it("accepts the service worker's answer to send_link", () => {
    expect(isLinkResult({ outcome: "added", message: "Added to Downpipe", taskId: "t", stop: false })).toBe(true);
    expect(isLinkResult({ outcome: "failed", message: "Downpipe is not running.", stop: true })).toBe(true);
    expect(isLinkResult({ outcome: "kept", message: "x", stop: false })).toBe(false);
    expect(isLinkResult({ outcome: "added", message: "x" })).toBe(false);
    expect(isLinkResult({ outcome: "added", message: "x", stop: false, taskId: 3 })).toBe(false);
    expect(isLinkResult(null)).toBe(false);
  });
});

describe("activity", () => {
  it("keeps the newest few", () => {
    let list: Activity[] = [];
    for (let i = 0; i < MAX_ACTIVITY + 3; i++) list = addActivity(list, entry(i));
    expect(list).toHaveLength(MAX_ACTIVITY);
    expect(list[0]?.at).toBe(MAX_ACTIVITY + 2);
  });

  it("drops stored entries that do not have the right shape", () => {
    expect(parseActivity([entry(1), { at: "x" }, null, { ...entry(2), outcome: "hacked" }])).toEqual([entry(1)]);
    expect(parseActivity([{ ...entry(3), taskId: "t" }, { ...entry(4), taskId: 7 }])).toEqual([{ ...entry(3), taskId: "t" }]);
    expect(
      parseActivity([
        { ...entry(5), awaitingConfirmation: true },
        { ...entry(6), awaitingConfirmation: "yes" },
      ]),
    ).toEqual([{ ...entry(5), awaitingConfirmation: true }]);
    expect(parseActivity(undefined)).toEqual([]);
  });

  it("collects the app's ids once each", () => {
    expect(taskIdsOf([{ ...entry(1), taskId: "a" }, entry(2), { ...entry(3), taskId: "a" }, { ...entry(4), taskId: "b" }])).toEqual([
      "a",
      "b",
    ]);
  });

  it("describes times briefly", () => {
    const now = Date.UTC(2026, 9, 17, 12);
    expect(relativeTime(now - 5_000, now)).toBe("just now");
    expect(relativeTime(now - 4 * 60_000, now)).toBe("4 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 3 * 86_400_000, now)).toBe("2026-10-14");
    expect(relativeTime(now + 10_000, now)).toBe("just now");
  });
});

describe("media requests", () => {
  it("accepts the popup's media requests exactly", () => {
    expect(parsePopupRequest({ kind: "media_list", tabId: 3 })).toEqual({ kind: "media_list", tabId: 3 });
    expect(parsePopupRequest({ kind: "send_media", tabId: 3, mediaId: "m-1" })).toEqual({
      kind: "send_media",
      tabId: 3,
      mediaId: "m-1",
    });
    expect(parsePopupRequest({ kind: "media_list", tabId: -1 })).toBeNull();
    expect(parsePopupRequest({ kind: "media_list", tabId: 1.5 })).toBeNull();
    expect(parsePopupRequest({ kind: "send_media", tabId: 3, mediaId: "../x" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_media", tabId: 3, mediaId: "m", url: "https://a.org/" })).toBeNull();
    expect(parsePopupRequest({ kind: "send_page", tabId: 3 })).toEqual({ kind: "send_page", tabId: 3 });
    // The page is the tab's own; no URL is taken from the message.
    expect(parsePopupRequest({ kind: "send_page", tabId: 3, url: "https://youtu.be/jNQXAC9IVRw" })).toBeNull();
  });

  it("lets the overlay ask about its own tab only", () => {
    expect(parseOverlayRequest({ kind: "overlay_list" })).toEqual({ kind: "overlay_list" });
    expect(parseOverlayRequest({ kind: "overlay_list", src: "https://a.org/v.mp4" })).toEqual({
      kind: "overlay_list",
      src: "https://a.org/v.mp4",
    });
    expect(parseOverlayRequest({ kind: "overlay_send", mediaId: "m-1" })).toEqual({ kind: "overlay_send", mediaId: "m-1" });
    expect(parseOverlayRequest({ kind: "overlay_page" })).toEqual({ kind: "overlay_page" });
    expect(parseOverlayRequest({ kind: "overlay_list", poster: "https://pbs.twimg.com/a/1.jpg" })).toEqual({
      kind: "overlay_list",
      poster: "https://pbs.twimg.com/a/1.jpg",
    });
    expect(
      parseOverlayRequest({ kind: "overlay_list", src: "https://a.org/v.mp4", poster: "https://a.org/p.jpg" }),
    ).toEqual({ kind: "overlay_list", src: "https://a.org/v.mp4", poster: "https://a.org/p.jpg" });
    for (const bad of [
      { kind: "overlay_page", pageUrl: "https://www.youtube.com/watch?v=jNQXAC9IVRw" },
      { kind: "send_page", tabId: 4 },
      { kind: "overlay_list", src: "blob:https://a.org/1" },
      { kind: "overlay_list", poster: "data:image/png;base64,AA" },
      { kind: "overlay_list", poster: 5 },
      { kind: "overlay_list", tabId: 4 },
      { kind: "overlay_send", mediaId: "m-1", tabId: 4 },
      { kind: "overlay_send" },
      // Popup requests are not for pages.
      { kind: "media_list", tabId: 4 },
      { kind: "send_link", url: "https://a.org/" },
      { kind: "status" },
      "overlay_list",
    ]) {
      expect(parseOverlayRequest(bad), JSON.stringify(bad)).toBeNull();
    }
  });

  it("checks what the overlay gets back", () => {
    expect(isOverlayItems({ items: [{ mediaId: "a", label: "x.m3u8", detail: "HLS stream" }] })).toBe(true);
    expect(isOverlayItems({ items: [{ mediaId: "a b", label: "x", detail: "y" }] })).toBe(false);
    expect(isOverlayItems({ items: [{ mediaId: PAGE_ITEM_ID, label: "Me at the zoo", detail: "YouTube" }] })).toBe(true);
    expect(isOverlayItems(null)).toBe(false);
    expect(overlayResultText({ outcome: "added", message: "Added to Downpipe", stop: false })).toEqual({
      text: "Sent to Downpipe",
      failed: false,
    });
    expect(overlayResultText({ outcome: "added", message: "Waiting for your answer in Downpipe", stop: false }).text).toBe(
      "Waiting for your answer in Downpipe",
    );
    expect(overlayResultText({ outcome: "failed", message: "Downpipe is not running.", stop: true })).toEqual({
      text: "Downpipe is not running.",
      failed: true,
    });
    expect(overlayResultText("nope").failed).toBe(true);
  });

  it("checks the page entry of a media list", () => {
    expect(isMediaReply({ enabled: false, items: [], page: { site: "YouTube", title: "Me at the zoo" } })).toBe(true);
    expect(isMediaReply({ enabled: false, items: [], page: { site: "YouTube" } })).toBe(true);
    expect(isMediaReply({ enabled: false, items: [], page: { title: "x" } })).toBe(false);
    expect(isMediaReply({ enabled: true, items: [], title: "Home / X" })).toBe(true);
    expect(isMediaReply({ enabled: true, items: [], title: 3 })).toBe(false);
  });

  it("keeps media hand-overs in the activity list", () => {
    const media: Activity = { at: 1, label: "Clip", source: "media", outcome: "added", message: "Added", taskId: "t" };
    expect(parseActivity([media])).toEqual([media]);
  });
});

describe("media settings requests", () => {
  it("accept the options page's media settings exactly", () => {
    const media = { mediaOverlay: false, mediaTypes: ["mp4", "mp3"], mediaMinBytes: 1024 };
    expect(parsePopupRequest({ kind: "save_media", media })).toEqual({ kind: "save_media", media });
    expect(parsePopupRequest({ kind: "save_media", media: { ...media, mediaTypes: ["MP4"] } })).toBeNull();
    expect(parsePopupRequest({ kind: "save_media", media, extra: 1 })).toBeNull();
    expect(parseOverlayRequest({ kind: "save_media", media })).toBeNull();
  });
});
