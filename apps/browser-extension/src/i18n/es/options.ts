// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filtros · Downpipe",
  headLabel: "Extensión del navegador",
  lede: "Elige qué descargas pasa el navegador a Downpipe mientras Capturar descargas del navegador está activado, y qué contenido multimedia muestra la extensión mientras Multimedia en esta página está activado. Todo lo demás se queda en el navegador como siempre.",
  captureOff:
    "La captura está desactivada. Estos filtros se aplican cuando activas Capturar descargas del navegador en la ventana emergente de Downpipe de la barra de herramientas.",
  loadError: "No se pudo cargar la configuración. Vuelve a cargar esta página para intentarlo de nuevo.",
  typesHeading: "01 — Tipos de archivo",
  captureAll: "Capturar todos los tipos de archivo",
  captureAllDetail: "Toda descarga que inicia el navegador va a Downpipe, sea del tipo que sea.",
  typesLabel: "Solo estos tipos de archivo",
  typesHelp:
    "Extensiones separadas por espacios o comas, como {example}. Una descarga se captura cuando su nombre de archivo termina en una de ellas; {tarGz} también funciona.",
  restoreDefault: "Restaurar la lista predeterminada",
  sitesHeading: "02 — Sitios excluidos",
  sitesLabel: "No capturar ni mostrar multimedia nunca en estos sitios",
  sitesHelp:
    "Un sitio por línea. {wildcard} también abarca todos los subdominios. Una descarga se queda en el navegador cuando proviene de uno de estos sitios o se inicia en una de sus páginas, y el contenido multimedia que reproducen no se muestra. Los puertos y las rutas se ignoran.",
  sizeHeading: "03 — Tamaño mínimo",
  sizeLabel: "Dejar al navegador los archivos más pequeños",
  sizeHelp: "0 lo desactiva. Cuando el servidor no indica el tamaño de un archivo, va a Downpipe.",
  mediaHeading: "04 — Multimedia en las páginas",
  mediaHelp:
    "Se usa mientras Multimedia en esta página está activado en la ventana emergente de la barra de herramientas. Las transmisiones HLS y DASH siempre se muestran; los sitios excluidos de arriba también se aplican aquí.",
  overlay: "Mostrar un botón Descargar en los videos",
  overlayDetail: "En un video en reproducción, durante unos segundos y mientras el puntero está encima.",
  mediaTypesLabel: "Mostrar archivos multimedia de estos tipos",
  mediaTypesHelp:
    "Extensiones separadas por espacios o comas. Un archivo sin extensión se evalúa por el tipo que envía el servidor. Con la lista vacía, solo se muestran transmisiones.",
  mediaSizeLabel: "Omitir archivos multimedia de menos de",
  mediaSizeHelp:
    "Deja fuera los sonidos de notificación y las vistas previas cortas. 0 muestra todos los tamaños; los archivos cuyo tamaño no indica el servidor siempre se muestran.",
  bypassHeading: "05 — Descargar en el navegador",
  bypassLabel: "Dejar que el navegador descargue un enlace pulsado con",
  bypassAlt: "Alt (Opción en un Mac)",
  bypassShift: "Mayús",
  bypassOff: "Desactivado",
  bypassHelp: "Con la captura activada, al pulsar un enlace con esta tecla, esa descarga se queda en el navegador. Los demás clics y los atajos del propio navegador no cambian.",
  bypassAccess: "Para notar el clic hace falta acceso a los sitios que visitas. No se lee ni se envía nada hasta que pulses un enlace de esta forma.",
  bypassAllow: "Permitir el acceso",
  save: "Guardar",
  moreErrors: { one: "Y {count} más.", many: "Y {count} más.", other: "Y {count} más." },
  unsaved: "Cambios sin guardar",
  notSavedFix: "No se guardó. Corrige los campos marcados en rojo.",
  saving: "Guardando…",
  notSavedNoAnswer: "No se guardó. La extensión no respondió; vuelve a intentarlo.",
  saved: "Guardado",
  errors: {
    notExtension: "“{value}” no es una extensión de archivo.",
    tooManyTypes: {
      one: "Indica como máximo {count} tipo de archivo.",
      many: "Indica como máximo {count} tipos de archivo.",
      other: "Indica como máximo {count} tipos de archivo.",
    },
    notSite: "Línea {line}: “{value}” no es un sitio. Usa un host como example.com o *.example.com.",
    tooManySites: {
      one: "Indica como máximo {count} sitio.",
      many: "Indica como máximo {count} sitios.",
      other: "Indica como máximo {count} sitios.",
    },
    sizeNotNumber: "Escribe un número como 2 o 1.5, o 0 para desactivarlo.",
    sizeTooBig: "Escribe un tamaño inferior a 1 TiB.",
    noTypes: "Agrega al menos un tipo de archivo o activa Capturar todos los tipos de archivo.",
  },
};
