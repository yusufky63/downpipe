// Problems the extension words itself. Messages the app sends for other codes
// are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "El puente del navegador de Downpipe no está instalado.",
    host_forbidden: "El puente del navegador está instalado, pero no permite esta extensión.",
    host_error: "El puente del navegador se detuvo inesperadamente.",
    timeout: "Downpipe no respondió a tiempo.",
    bad_response: "El puente del navegador envió una respuesta que esta extensión no entiende.",
  },
  code: {
    app_not_running: "Downpipe no se está ejecutando. Abre la aplicación Downpipe y vuelve a intentarlo.",
    app_unavailable: "Downpipe no respondió. Reinicia la aplicación y vuelve a intentarlo.",
    duplicate: "Ya está en Downpipe.",
    invalid_url: "Este enlace no se puede descargar. Solo funcionan los enlaces http y https.",
    version_mismatch:
      "Downpipe y su puente del navegador son de versiones distintas. Reinicia Downpipe para que registre su propio puente.",
    unsupported_version: "Esta extensión es más reciente que el Downpipe instalado. Actualiza Downpipe.",
    message_too_large: "El enlace es demasiado largo para enviarlo.",
    unsupported_page: "Downpipe no descarga videos de este sitio.",
  },
  refused: "Downpipe rechazó la descarga ({code}).",
  streamsUnsupported: "Esta versión de Downpipe no puede descargar transmisiones. Actualiza Downpipe.",
  pagesUnsupported: "Esta versión de Downpipe no puede descargar videos de páginas. Actualiza Downpipe.",
  keptClosed: "Downpipe no se estaba ejecutando, así que el navegador la descargó.",
  keptInstead: "{reason} El navegador la descargó en su lugar.",
  notVideoPage: "Downpipe no descarga videos de esta página.",
  unknownVideoPage: "Esta no es una página de video que Downpipe conozca.",
  mediaGone: "Este contenido multimedia ya no aparece en la lista. Vuelve a reproducirlo.",
  mediaInvalid: "Este contenido multimedia no se puede descargar.",
  streamInvalid: "Esta transmisión no se puede descargar.",
};
