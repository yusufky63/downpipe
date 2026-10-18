// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Nueva descarga",
  title: "Agregar un enlace",
  titleMany: { one: "Agregar {count} enlace", many: "Agregar {count} enlaces", other: "Agregar {count} enlaces" },
  addPaused: "Agregar en pausa",
  submit: {
    next: "Siguiente",
    addToList: "Agregar a la lista",
    download: "Descargar",
    downloadMany: { one: "Descargar {count} archivo", many: "Descargar {count} archivos", other: "Descargar {count} archivos" },
  },
  links: {
    label: "Enlaces",
    hint: "Un enlace por línea, o un patrón como file[001-100].zip. Solo se admiten http y https.",
    found: {
      one: "{count} enlace encontrado. Una descarga por enlace.",
      many: "{count} enlaces encontrados. Una descarga por enlace.",
      other: "{count} enlaces encontrados. Una descarga por enlace.",
    },
    fromPattern: {
      one: "{count} enlace del patrón. Una descarga por enlace.",
      many: "{count} enlaces del patrón. Una descarga por enlace.",
      other: "{count} enlaces del patrón. Una descarga por enlace.",
    },
    patternCapped: {
      one: "Solo se usa el primer enlace del patrón.",
      many: "Solo se usan los primeros {count} enlaces del patrón.",
      other: "Solo se usan los primeros {count} enlaces del patrón.",
    },
    video: "Un video de {site}. A continuación eliges su calidad.",
    allVideos: "A continuación eliges la calidad de cada video.",
    someVideos: {
      one: "Uno de ellos es un video: a continuación eliges su calidad.",
      many: "{count} de ellos son videos: a continuación eliges su calidad.",
      other: "{count} de ellos son videos: a continuación eliges su calidad.",
    },
    importList: "Importar una lista",
    importHint: "Un archivo .txt con un enlace por línea. También puedes soltarlo aquí.",
  },
  saveTo: {
    label: "Guardar en",
    automatic: "Automático: la carpeta de la categoría del archivo.",
  },
  fileName: {
    label: "Nombre de archivo",
    placeholder: "Automático",
    hint: "Déjalo vacío para usar el nombre que envía el servidor.",
    videoHint: "Déjalo vacío para usar el título del video.",
    singleOnly: "Solo para un único enlace.",
  },
  connections: {
    label: "Conexiones",
    hint: "Límite máximo",
    videoHint: "No para videos",
  },
  queue: {
    label: "Cola",
    hint: "Cada cola ejecuta sus propias descargas simultáneas.",
  },
  checksum: {
    label: "Suma de comprobación",
    hint: "Opcional, p. ej., sha256:9f86d0… El archivo solo recibe su nombre final si coincide.",
    videoHint: "Solo para archivos: yt-dlp compone el archivo de un video a partir de los formatos del sitio.",
  },
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "Encabezados de la solicitud",
    hint: "Opcional. Uno por línea, como Nombre: valor. Se envían con cada solicitud y solo se guardan en memoria, nunca en disco.",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "Pegar como cURL",
    label: "Comando cURL",
    hint: "En las herramientas para desarrolladores del navegador: Red, clic derecho en la solicitud, Copiar como cURL. Solo se pueden descargar solicitudes GET.",
    apply: "Rellenar",
    filled: "Rellenado a partir del comando: el enlace y sus encabezados.",
    ignored: "Se omitió: {list}.",
    notCurl: "Esto no es un comando cURL. Tiene que empezar por curl.",
    quote: "Una comilla del comando nunca se cierra. Copia de nuevo el comando completo.",
    noUrl: "El comando no tiene ningún enlace http o https.",
    method: "Esta solicitud usa {method}. Downpipe solo descarga con GET, así que no puede repetirla.",
    data: "Esta solicitud envía datos ({option}), lo que la convierte en un POST. Downpipe solo descarga con GET.",
  },
  errors: {
    listTooLarge: "Ese archivo es demasiado grande para una lista de enlaces.",
    noLinks: "Pega al menos un enlace http o https.",
    forLink: "{url}: {error}",
    headerFormat: "Línea {line}: escríbelo como Nombre: valor.",
    headerName: "Línea {line}: {name} no es un nombre de encabezado válido.",
    headerManaged: "Línea {line}: Downpipe pone {name} por su cuenta.",
  },
};
