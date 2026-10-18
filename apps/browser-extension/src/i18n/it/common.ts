// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Scarica",
  sending: "Invio…",
  checkAgain: "Controlla di nuovo",
  setupGuide: "Guida all'installazione",
  filters: "Filtri",
  unit: "Unità",
  extensionNoAnswer: "L'estensione non ha risposto.",
  justNow: "adesso",
  minutesAgo: { one: "{count} min fa", many: "{count} min fa", other: "{count} min fa" },
  hoursAgo: { one: "{count} h fa", many: "{count} h fa", other: "{count} h fa" },
};

export const results: Messages["results"] = {
  listOpened: "Aperta in Downpipe: scegli lì i video.",
  added: "Aggiunto a Downpipe",
  waiting: "In attesa della tua risposta in Downpipe",
  sent: "Inviato a Downpipe",
};
