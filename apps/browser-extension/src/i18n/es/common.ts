// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Descargar",
  sending: "Enviando…",
  checkAgain: "Volver a comprobar",
  setupGuide: "Guía de configuración",
  filters: "Filtros",
  unit: "Unidad",
  extensionNoAnswer: "La extensión no respondió.",
  justNow: "ahora mismo",
  minutesAgo: { one: "hace {count} min", many: "hace {count} min", other: "hace {count} min" },
  hoursAgo: { one: "hace {count} h", many: "hace {count} h", other: "hace {count} h" },
};

export const results: Messages["results"] = {
  added: "Agregada a Downpipe",
  waiting: "Esperando tu respuesta en Downpipe",
  sent: "Enviada a Downpipe",
};
