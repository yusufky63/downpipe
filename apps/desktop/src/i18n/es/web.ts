// Buscar archivos en la web: el recolector de sitios, las suscripciones a feeds y los alojamientos de archivos.
import type { Messages } from "../core";

export const grabber: Messages["grabber"] = {
  open: "Recolector de sitios",
  eyebrow: "Recolector de sitios",
  title: "Buscar archivos en un sitio",
  address: {
    label: "Dirección inicial",
    hint: "Downpipe lee esta página y sigue sus enlaces. No envía cookies, así que ve lo mismo que un visitante sin sesión iniciada.",
  },
  depth: {
    label: "Profundidad",
    hint: "1 lee solo esta página; cada nivel sigue un nivel más de enlaces.",
  },
  pages: {
    label: "Límite de páginas",
    hint: "Páginas leídas como máximo",
  },
  types: {
    label: "Tipos de archivo",
    custom: "Otros tipos",
    customPlaceholder: "p. ej. iso, torrent",
  },
  sameSite: {
    label: "Quedarse en este sitio",
    hint: "Solo se leen páginas del sitio de la dirección inicial. Los archivos enlazados desde otros sitios se listan igualmente.",
  },
  robots: {
    label: "Respetar robots.txt",
    hint: "Omite las páginas que el sitio pide a los rastreadores que no visiten.",
  },
  sizes: {
    label: "Consultar tamaños",
    hint: "Una pequeña petición por archivo, para los primeros {count} archivos.",
  },
  delay: {
    label: "Espera",
    hint: "Milisegundos entre dos peticiones al mismo sitio. Los sitios pueden bloquear a quien va demasiado rápido.",
  },
  start: "Buscar archivos",
  stop: "Detener",
  again: "Nueva búsqueda",
  progress: {
    pages: { one: "{done} de {count} página leídas", many: "{done} de {count} páginas leídas", other: "{done} de {count} páginas leídas" },
    sizes: {
      one: "Consultando tamaños: {done} de {count} archivo",
      many: "Consultando tamaños: {done} de {count} archivos",
      other: "Consultando tamaños: {done} de {count} archivos",
    },
    files: { one: "{count} archivo encontrado", many: "{count} archivos encontrados", other: "{count} archivos encontrados" },
  },
  results: {
    title: { one: "{count} archivo encontrado", many: "{count} archivos encontrados", other: "{count} archivos encontrados" },
    none: "No se encontraron archivos de estos tipos.",
    canceled: "Se detuvo antes de terminar: estos son los archivos encontrados hasta entonces.",
    pageLimit: "Se alcanzó el límite de páginas antes de leerlas todas.",
    blocked: {
      one: "Se omitió {count} página o archivo porque robots.txt no lo permite.",
      many: "Se omitieron {count} páginas o archivos porque robots.txt no lo permite.",
      other: "Se omitieron {count} páginas o archivos porque robots.txt no lo permite.",
    },
    failed: { one: "No se pudo leer {count} página.", many: "No se pudieron leer {count} páginas.", other: "No se pudieron leer {count} páginas." },
    filter: "Filtrar archivos",
    selectAll: "Seleccionar todos los archivos mostrados",
    name: "Nombre",
    size: "Tamaño",
    foundOn: "Encontrado en",
    selected: { one: "{count} seleccionado, {size}", many: "{count} seleccionados, {size}", other: "{count} seleccionados, {size}" },
    unknownSizes: { one: "{count} de tamaño desconocido", many: "{count} de tamaño desconocido", other: "{count} de tamaño desconocido" },
    add: { one: "Añadir {count} archivo", many: "Añadir {count} archivos", other: "Añadir {count} archivos" },
    added: { one: "{count} archivo añadido.", many: "{count} archivos añadidos.", other: "{count} archivos añadidos." },
    alreadyListed: { one: "{count} ya estaba en la lista.", many: "{count} ya estaban en la lista.", other: "{count} ya estaban en la lista." },
  },
  errors: {
    invalid_url: "Escribe una dirección http o https.",
    no_types: "Elige al menos un tipo de archivo.",
    robots_blocked: "El robots.txt de este sitio pide a los rastreadores que no lean esta página. Desactiva Respetar robots.txt para leerla de todos modos.",
    start_page: "No se pudo leer la página inicial: {error}",
    not_html: "La dirección inicial no es una página web. Añádela como descarga.",
  },
};

export const feeds: Messages["feeds"] = {
  open: "Feeds",
  eyebrow: "Feeds",
  title: "Feeds RSS y Atom",
  intro: "Downpipe revisa cada feed en su intervalo y añade los archivos de las entradas nuevas a la cola que elijas. Cada entrada se añade una sola vez, aunque quites su descarga.",
  empty: "Aún no hay feeds. Añade un pódcast, un feed de versiones o cualquier otra dirección RSS o Atom.",
  add: "Añadir un feed",
  checkNow: "Revisar ahora",
  edit: "Editar",
  off: "Desactivado",
  checked: "Revisado {date}",
  notChecked: "Aún sin revisar",
  error: "La última revisión falló: {error}",
  addedTotal: {
    one: "{count} descarga añadida en total",
    many: "{count} descargas añadidas en total",
    other: "{count} descargas añadidas en total",
  },
  checkResult: {
    none: "No hay entradas nuevas en {name}.",
    added: {
      one: "{count} descarga añadida desde {name}.",
      many: "{count} descargas añadidas desde {name}.",
      other: "{count} descargas añadidas desde {name}.",
    },
  },
  confirmRemove: "¿Quitar {name}? Las descargas que añadió se quedan en la lista.",
  form: {
    newTitle: "Nuevo feed",
    editTitle: "Editar feed",
    url: "Dirección del feed",
    types: "Tipos de archivo",
    typesHint: "Vacío: cualquier archivo que ofrezcan las entradas. Por ejemplo mp3, m4a.",
    pattern: "Filtro de título",
    patternHint: "Una expresión regular que el título debe contener, sin distinguir mayúsculas, por ejemplo episodio|extra. Vacío: cualquier título.",
    interval: "Revisar cada",
    queue: "Cola",
    enabled: "Revisar este feed",
    enabledHint: "Desactivado: el feed sigue en la lista pero no se revisa.",
    existing: "Descargar lo que contiene ahora",
    existingHint: "Desactivado: solo se descargan las entradas publicadas a partir de ahora.",
    save: "Guardar",
    subscribe: "Añadir feed",
    subscribed: {
      one: "Feed añadido. {count} descarga añadida.",
      many: "Feed añadido. {count} descargas añadidas.",
      other: "Feed añadido. {count} descargas añadidas.",
    },
    subscribedNone: "Feed añadido. A partir de ahora se descargarán las entradas nuevas.",
  },
  interval: {
    minutes: { one: "{count} minuto", many: "{count} minutos", other: "{count} minutos" },
    hours: { one: "{count} hora", many: "{count} horas", other: "{count} horas" },
    days: { one: "{count} día", many: "{count} días", other: "{count} días" },
  },
  errors: {
    duplicate_feed: "Este feed ya está en la lista.",
    not_a_feed: "Esa dirección es una página web, no un feed RSS o Atom.",
    invalid_url: "Escribe una dirección http o https.",
  },
};

export const hosts: Messages["hosts"] = {
  privateFile: "El archivo no se ha compartido públicamente. Pide a su propietario que lo comparta con cualquiera que tenga el enlace, o descárgalo en el navegador con tu sesión iniciada.",
};
