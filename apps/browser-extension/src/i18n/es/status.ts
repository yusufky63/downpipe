// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Comprobando la conexión",
  connected: "Conectado a Downpipe",
  connectedDetail: "Aplicación {app}, puente del navegador {bridge}",
  appNotRunning: "Downpipe no se está ejecutando",
  /** Instead of appNotRunningDetail while the bridge may start Downpipe. */
  appNotRunningStarts: "Una descarga que envíes lo inicia en la bandeja del sistema.",
  appNotRunningDetail: "Abre la aplicación Downpipe. Los enlaces enviados mientras está cerrada no se ponen en cola.",
  hostMissing: "Puente del navegador no instalado",
  hostMissingDetail:
    "Abre la aplicación Downpipe una vez; registra el puente al iniciarse. Si la compilaste desde el código fuente, ejecuta:",
  hostForbidden: "El puente no permite esta extensión",
  hostForbiddenDetail: "Regístralo para el id de esta extensión y luego reinicia el navegador:",
  error: "Error del puente",
};
