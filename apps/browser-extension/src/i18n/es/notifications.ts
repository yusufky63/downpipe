// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip. The download's name comes first in each message.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Esperando tu respuesta en Downpipe",
  waitingBody: "Confirma su nombre y carpeta en Downpipe para iniciar la descarga.",
  downloading: "Descargando en Downpipe",
  duplicate: "Ya está en Downpipe",
  kept: "Descargada por el navegador",
  failed: "No se envió a Downpipe",
  toolbarTitle: "Downpipe: {label}. {message}",
};
