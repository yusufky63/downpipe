// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Untertitel",
    none: "Keine",
    uploaded: "Vom Uploader",
    auto: "Automatische Untertitel",
    original: "{language} (Original)",
    embedded: "Die Untertitel kommen in die Videodatei.",
    separate: "Die Untertitel werden als Datei neben dem Video gespeichert.",
  },
  siteErrors: {
    forbidden: "YouTube hat den Download abgelehnt. Ein neueres yt-dlp behebt das meist: Aktualisieren Sie es unter Einstellungen, Medien, und versuchen Sie es erneut.",
    signIn: "YouTube verlangt für diesen Download einen angemeldeten Browser. Downpipe verwendet keine Browser-Cookies: Warten Sie eine Weile oder nutzen Sie ein anderes Netzwerk, und versuchen Sie es erneut.",
    ageRestricted: "YouTube zeigt dieses Video nur angemeldeten Erwachsenen. Downpipe verwendet weder Konten noch Browser-Cookies und kann es daher nicht herunterladen.",
    unavailable: "Dieses Video ist nicht verfügbar: Es ist privat, entfernt, nur für Mitglieder oder in Ihrer Region gesperrt.",
    jsRuntime: "yt-dlp konnte die JavaScript-Abfragen von YouTube nicht beantworten. Laden Sie Deno unter Einstellungen, Medien herunter, und versuchen Sie es erneut.",
    poToken: "YouTube hat für dieses Video ein Herkunftsnachweis-Token (PO-Token) verlangt. Aktualisieren Sie yt-dlp unter Einstellungen, Medien, oder wählen Sie eine andere Qualität.",
    outdated: "yt-dlp versteht diese Seite nicht mehr; YouTube hat sie wahrscheinlich geändert. Aktualisieren Sie yt-dlp unter Einstellungen, Medien, und versuchen Sie es erneut.",
  },
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
