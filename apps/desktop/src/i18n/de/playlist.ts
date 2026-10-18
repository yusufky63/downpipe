// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "{site}-Playlist",
    channel: "{site}-Kanal",
  },
  untitled: "Playlist",
  reading: "yt-dlp wird nach den Videos gefragt…",
  unavailable: "Diese Playlist oder dieser Kanal ist nicht verfügbar: privat, entfernt oder in Ihrer Region gesperrt.",
  empty: "yt-dlp hat in dieser Liste keine Videos gefunden.",
  truncated: { one: "Nur das erste Video wird aufgeführt.", other: "Nur die ersten {count} Videos werden aufgeführt." },
  filter: "Nach Titel filtern",
  selectAll: "Alle auswählen",
  selectNone: "Keine auswählen",
  selected: { one: "{count} von {total} ausgewählt", other: "{count} von {total} ausgewählt" },
  rowsLabel: "Videos",
  notAvailable: "Nicht verfügbar",
  untitledVideo: "Video ohne Titel",
  quality: "Qualität",
  qualityHint: "Eine Qualität für alle Videos; ein Video ohne sie bekommt die nächstniedrigere.",
  presets: {
    best: "Beste",
    "1080p": "Bis 1080p",
    "720p": "Bis 720p",
    audio: "Nur Audio",
  },
  autoCaptions: "Automatische Untertitel verwenden, wo es keine Untertitel gibt",
  noFfmpeg: "Kein FFmpeg gefunden: Nur Qualitäten mit eigenem Ton lassen sich herunterladen, bei YouTube also höchstens 360p.",
  folder: "Ordner",
  folderHint: "Wird im Ordner daneben angelegt. Leer: kein eigener Ordner.",
  saveIn: "Speichern in",
  submit: { one: "{count} Video herunterladen", other: "{count} Videos herunterladen" },
  addToList: { one: "{count} Video hinzufügen", other: "{count} Videos hinzufügen" },
  added: { one: "{count} Video hinzugefügt.", other: "{count} Videos hinzugefügt." },
  skipped: { one: "{count} war schon in der Liste.", other: "{count} waren schon in der Liste." },
};
