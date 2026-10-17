// Video pages Downpipe downloads through its site provider (yt-dlp). The app
// recognizes them itself when a link is added (crates/providers/src/pages.rs);
// this copy of the rules, checked against the same cases
// (crates/providers/tests/fixtures/pages.json), only decides which links the
// Add dialog sends on to the quality chooser instead of starting them in
// yt-dlp's best quality, which can be a very large 4K file.

export interface SitePage {
  /** The site as people name it. */
  site: "YouTube";
  id: string;
  /** The page without tracking, timestamps or playlists. */
  url: string;
}

const YOUTUBE_ID = /^[A-Za-z0-9_-]{11}$/;

/** The video page behind a link, if Downpipe's site provider takes it. */
export function sitePage(raw: string): SitePage | null {
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return null;
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") return null;
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
