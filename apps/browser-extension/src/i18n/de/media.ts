// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  hls: "HLS-Stream",
  dash: "DASH-Stream",
  hlsQualities: { one: "HLS · {count} Qualität", other: "HLS · {count} Qualitäten" },
  fileAudio: "{format}-Audio",
  fileVideo: "{format}-Video",
  audio: "Audio",
  video: "Video",
  siteVideo: "{site}-Video",
  thisSiteVideo: "Dieses {site}-Video",
  siteVideoQuality: "{site}-Video · Qualität wird in Downpipe gewählt",
  updateForStreams: "Aktualisieren Sie Downpipe, um Streams herunterzuladen",
  updateForPages: "Aktualisieren Sie Downpipe, um Videos von Webseiten herunterzuladen",
  emptyOff:
    "Einschalten, um die Videos und Audiodateien aufzulisten, die Seiten abspielen. Der Browser fragt zuerst nach Zugriff auf alle Websites.",
  emptyExcluded: "Hier nicht aufgeführt: {site} ist unter „Filter“ als ausgeschlossene Website eingetragen.",
  emptyNothing: "Noch nichts. Spielen Sie das Video ab; falls es bereits lief, laden Sie die Seite neu.",
};
