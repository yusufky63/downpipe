// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Télécharger",
  sending: "Envoi…",
  checkAgain: "Vérifier à nouveau",
  setupGuide: "Guide d'installation",
  filters: "Filtres",
  unit: "Unité",
  extensionNoAnswer: "L'extension n'a pas répondu.",
  justNow: "à l'instant",
  minutesAgo: { one: "il y a {count} min", many: "il y a {count} min", other: "il y a {count} min" },
  hoursAgo: { one: "il y a {count} h", many: "il y a {count} h", other: "il y a {count} h" },
};

export const results: Messages["results"] = {
  added: "Ajouté à Downpipe",
  waiting: "En attente de votre réponse dans Downpipe",
  sent: "Envoyé à Downpipe",
};
