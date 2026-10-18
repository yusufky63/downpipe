// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Nur Audio",
  defaultQuality: "Standard",
  aboutSize: "etwa {size}",
  audio: "Audio",
  tools: {
    found: "Gefunden",
    notFound: "Nicht gefunden",
    problem: "Problem",
    version: "Version {version}",
    sources: {
      settings: "in den Einstellungen festgelegt",
      managed: "von Downpipe heruntergeladen",
      path: "im PATH",
    },
    missing: {
      ytDlp: "Erforderlich für Videos auf Webseiten.",
      runtime: "Ohne Deno (oder Node) findet yt-dlp einige YouTube-Qualitäten nicht.",
      ffmpeg: "Ohne FFmpeg funktionieren nur „Nur Audio“ und Qualitäten mit eigenem Ton.",
    },
    download: "{tools} herunterladen",
    update: "{tools} aktualisieren",
  },
};
