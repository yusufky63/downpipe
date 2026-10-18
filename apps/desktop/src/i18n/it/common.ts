// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Annulla",
  save: "Salva",
  close: "Chiudi",
  browse: "Sfoglia",
  dismiss: "Ignora",
  keep: "Mantieni",
  open: "Apri",
  pause: "Metti in pausa",
  resume: "Riprendi",
  retry: "Riprova",
  remove: "Rimuovi",
  showInFolder: "Mostra nella cartella",
  moreActions: "Altre azioni",
  unknown: "Sconosciuto",
  none: "–",
  siteVideo: "Video di {site}",
  downloads: { one: "{count} download", many: "{count} download", other: "{count} download" },
  links: { one: "{count} link", many: "{count} link", other: "{count} link" },
};

export const states: Messages["states"] = {
  confirm: "Da confermare",
  queued: "In coda",
  starting: "In connessione",
  downloading: "In download",
  waiting: "Nuovo tentativo",
  verifying: "In verifica",
  paused: "In pausa",
  completed: "Completato",
  failed: "Non riuscito",
  canceled: "Annullato",
  filters: {
    all: "Tutti",
    active: "Attivi",
    queued: "In coda",
    completed: "Completati",
    failed: "Non riusciti",
  },
};
