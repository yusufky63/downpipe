// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Neuer Download",
  title: "Link hinzufügen",
  titleMany: { one: "{count} Link hinzufügen", other: "{count} Links hinzufügen" },
  addPaused: "Pausiert hinzufügen",
  submit: {
    next: "Weiter",
    addToList: "Zur Liste hinzufügen",
    download: "Herunterladen",
    downloadMany: { one: "{count} Datei herunterladen", other: "{count} Dateien herunterladen" },
  },
  links: {
    playlist: "Eine {site}-Playlist. Als Nächstes wählen Sie ihre Videos aus.",
    channel: "Ein {site}-Kanal. Als Nächstes wählen Sie seine Videos aus.",
    label: "Links",
    hint: "Ein Link pro Zeile oder ein Muster wie file[001-100].zip. Nur http und https werden unterstützt.",
    found: { one: "{count} Link gefunden. Ein Download pro Link.", other: "{count} Links gefunden. Ein Download pro Link." },
    fromPattern: { one: "{count} Link aus dem Muster. Ein Download pro Link.", other: "{count} Links aus dem Muster. Ein Download pro Link." },
    // German "one" is only ever 1, so that form can say "the first link" without the number.
    patternCapped: { one: "Nur der erste Link des Musters wird verwendet.", other: "Nur die ersten {count} Links des Musters werden verwendet." },
    video: "Ein {site}-Video. Die Qualität wählen Sie im nächsten Schritt.",
    allVideos: "Die Qualität jedes Videos wählen Sie im nächsten Schritt.",
    someVideos: {
      one: "Einer davon ist ein Video, dessen Qualität Sie im nächsten Schritt wählen.",
      other: "{count} davon sind Videos, deren Qualität Sie im nächsten Schritt wählen.",
    },
    importList: "Liste importieren",
    importHint: "Eine .txt-Datei mit einem Link pro Zeile. Sie können sie auch hier ablegen.",
  },
  saveTo: {
    label: "Speichern unter",
    automatic: "Automatisch: der Ordner der Dateikategorie.",
  },
  fileName: {
    label: "Dateiname",
    placeholder: "Automatisch",
    hint: "Leer lassen, um den vom Server gesendeten Namen zu verwenden.",
    videoHint: "Leer lassen, um den Titel des Videos zu verwenden.",
    singleOnly: "Nur für einen einzelnen Link.",
  },
  connections: {
    label: "Verbindungen",
    hint: "Obergrenze",
    videoHint: "Nicht für Videos",
  },
  queue: {
    label: "Warteschlange",
    hint: "Jede Warteschlange führt ihre eigenen Downloads gleichzeitig aus.",
  },
  checksum: {
    label: "Prüfsumme",
    hint: "Optional, z. B. sha256:9f86d0… Die Datei erhält ihren endgültigen Namen nur, wenn sie übereinstimmt.",
    videoHint: "Nur für Dateien: yt-dlp setzt die Datei eines Videos aus den Formaten der Website zusammen.",
  },
  errors: {
    listTooLarge: "Diese Datei ist zu groß für eine Liste von Links.",
    noLinks: "Fügen Sie mindestens einen http- oder https-Link ein.",
    forLink: "{url}: {error}",
  },
};
