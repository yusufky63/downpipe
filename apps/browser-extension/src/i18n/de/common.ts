// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Herunterladen",
  sending: "Wird gesendet…",
  checkAgain: "Erneut prüfen",
  setupGuide: "Einrichtungsanleitung",
  filters: "Filter",
  unit: "Einheit",
  extensionNoAnswer: "Die Erweiterung hat nicht geantwortet.",
  justNow: "gerade eben",
  minutesAgo: { one: "vor {count} Min.", other: "vor {count} Min." },
  hoursAgo: { one: "vor {count} Std.", other: "vor {count} Std." },
};

export const results: Messages["results"] = {
  listOpened: "In Downpipe geöffnet: Wählen Sie dort die Videos aus.",
  added: "Zu Downpipe hinzugefügt",
  waiting: "Wartet in Downpipe auf Ihre Antwort",
  sent: "An Downpipe gesendet",
};
