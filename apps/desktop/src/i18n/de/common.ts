// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Abbrechen",
  save: "Speichern",
  close: "Schließen",
  browse: "Durchsuchen",
  dismiss: "Ausblenden",
  keep: "Behalten",
  open: "Öffnen",
  pause: "Pausieren",
  resume: "Fortsetzen",
  retry: "Erneut versuchen",
  remove: "Entfernen",
  showInFolder: "Im Ordner anzeigen",
  moreActions: "Weitere Aktionen",
  unknown: "Unbekannt",
  none: "–",
  siteVideo: "{site}-Video",
  downloads: { one: "{count} Download", other: "{count} Downloads" },
  links: { one: "{count} Link", other: "{count} Links" },
};

export const states: Messages["states"] = {
  confirm: "Bestätigen",
  queued: "In Warteschlange",
  starting: "Verbindet",
  downloading: "Wird heruntergeladen",
  waiting: "Wird wiederholt",
  verifying: "Wird geprüft",
  paused: "Pausiert",
  completed: "Fertig",
  failed: "Fehlgeschlagen",
  canceled: "Abgebrochen",
  filters: {
    all: "Alle",
    active: "Aktiv",
    queued: "In Warteschlange",
    completed: "Abgeschlossen",
    failed: "Fehlgeschlagen",
  },
};
