// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
export const media = {
  hls: "HLS stream",
  dash: "DASH stream",
  hlsQualities: { one: "HLS · {count} quality", other: "HLS · {count} qualities" },
  fileAudio: "{format} audio",
  fileVideo: "{format} video",
  audio: "Audio",
  video: "Video",
  /** A video page before its title is known, e.g. "YouTube video". */
  siteVideo: "{site} video",
  thisSiteVideo: "This {site} video",
  siteVideoQuality: "{site} video · quality chosen in Downpipe",
  updateForStreams: "Update Downpipe to download streams",
  updateForPages: "Update Downpipe to download videos from pages",
  emptyOff: "Turn on to list the videos and audio pages play. The browser asks for access to all sites first.",
  emptyExcluded: "Not listed here: {site} is an excluded site in Filters.",
  emptyNothing: "Nothing yet. Play the video; if it was already playing, reload the page.",
};
