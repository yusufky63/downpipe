// Video pages Downpipe downloads through its site provider (yt-dlp) instead
// of from the media requests the page makes. The same rules, and the same
// test cases (crates/providers/tests/fixtures/pages.json), are in
// crates/providers/src/pages.rs; the app refuses a `page` message for
// anything this does not recognize.
import { webUrl } from "./url";

export interface SitePage {
  /** The site as people name it. */
  site: "YouTube";
  id: string;
  /** The page without tracking, timestamps or playlists. */
  url: string;
}

const YOUTUBE_ID = /^[A-Za-z0-9_-]{11}$/;

/** The video page behind a link, if Downpipe's site provider takes it. */
export function sitePage(raw: string | undefined): SitePage | null {
  const url = webUrl(raw);
  if (!url) return null;
  const host = url.hostname.toLowerCase().replace(/^www\./, "");
  const path = url.pathname.split("/").filter(Boolean);
  let id: string | null | undefined;
  if (host === "youtube.com" || host === "m.youtube.com" || host === "music.youtube.com") {
    if (path[0] === "watch") id = url.searchParams.get("v");
    else if (path[0] === "shorts" || path[0] === "embed" || path[0] === "live" || path[0] === "v") id = path[1];
  } else if (host === "youtube-nocookie.com") {
    if (path[0] === "embed") id = path[1];
  } else if (host === "youtu.be") {
    id = path[0];
  }
  if (!id || !YOUTUBE_ID.test(id)) return null;
  return { site: "YouTube", id, url: `https://www.youtube.com/watch?v=${id}` };
}

export interface SiteList {
  site: "YouTube";
  kind: "playlist" | "channel";
  /** The list's own page: a playlist, or a channel's videos (Shorts, live) tab. */
  url: string;
}

const LIST_ID = /^[A-Za-z0-9_-]{2,64}$/;
const CHANNEL_ID = /^UC[A-Za-z0-9_-]{22}$/;
const CHANNEL_NAME = /^[A-Za-z0-9._%-]{1,300}$/;
// Watch later and liked videos: empty without the signed-in browser.
const PRIVATE_LISTS = new Set(["WL", "LL", "LM"]);

/**
 * A playlist or channel, whose videos the app lists for the user to choose
 * from (crates/providers/src/pages.rs `recognize_list`, checked against
 * crates/providers/tests/fixtures/lists.json). A video link that names a
 * playlist is the video.
 */
export function siteList(raw: string | undefined): SiteList | null {
  const url = webUrl(raw);
  if (!url || sitePage(raw)) return null;
  const host = url.hostname.toLowerCase().replace(/^www\./, "");
  if (host !== "youtube.com" && host !== "m.youtube.com" && host !== "music.youtube.com") return null;
  const [first, ...rest] = url.pathname.split("/").filter(Boolean);
  if (first === undefined) return null;
  const list = (kind: SiteList["kind"], at: string): SiteList => ({ site: "YouTube", kind, url: `https://www.youtube.com/${at}` });
  if (first === "playlist") {
    const id = url.searchParams.get("list");
    if (!id || !LIST_ID.test(id) || PRIVATE_LISTS.has(id)) return null;
    return list("playlist", `playlist?list=${id}`);
  }
  let channel: string;
  let tab: string | undefined;
  if (first.startsWith("@")) {
    if (!CHANNEL_NAME.test(first.slice(1))) return null;
    channel = first;
    tab = rest[0];
  } else {
    const name = rest[0];
    if (name === undefined) return null;
    const ok = first === "channel" ? CHANNEL_ID.test(name) : (first === "c" || first === "user") && CHANNEL_NAME.test(name);
    if (!ok) return null;
    channel = `${first}/${name}`;
    tab = rest[1];
  }
  return list("channel", `${channel}/${tab === "shorts" || tab === "streams" ? tab : "videos"}`);
}

/**
 * Hosts whose media requests are pieces of a site page's player (YouTube's
 * video servers). They cannot be downloaded on their own, so media
 * detection leaves them out and the page itself is offered instead.
 */
export function isSiteMedia(raw: string): boolean {
  const host = webUrl(raw)?.hostname.toLowerCase();
  return host !== undefined && (host === "googlevideo.com" || host.endsWith(".googlevideo.com"));
}

/**
 * "Me at the zoo" from YouTube's tab title "(3) Me at the zoo - YouTube".
 * Undefined while the tab is still titled after the site alone (YouTube
 * sets the video's title a moment after the page loads): the app then
 * takes the title from yt-dlp.
 */
export function pageTitle(raw: string | undefined): string | undefined {
  const title = raw
    ?.replace(/^\(\d+\)\s+/, "")
    .replace(/\s+-\s+YouTube( Music)?$/, "")
    .trim();
  if (!title || /^YouTube( Music)?$/.test(title)) return undefined;
  return title;
}
