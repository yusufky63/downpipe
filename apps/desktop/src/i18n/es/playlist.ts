// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "Lista de reproducción de {site}",
    channel: "Canal de {site}",
  },
  untitled: "Lista de reproducción",
  reading: "Pidiendo los videos a yt-dlp…",
  unavailable: "Esta lista o este canal no está disponible: es privado, se eliminó o está bloqueado en tu región.",
  empty: "yt-dlp no encontró videos en esta lista.",
  truncated: { one: "Solo se muestra el primer video.", many: "Solo se muestran los primeros {count} videos.", other: "Solo se muestran los primeros {count} videos." },
  filter: "Filtrar por título",
  selectAll: "Seleccionar todo",
  selectNone: "No seleccionar nada",
  selected: { one: "{count} de {total} seleccionado", many: "{count} de {total} seleccionados", other: "{count} de {total} seleccionados" },
  rowsLabel: "Videos",
  notAvailable: "No disponible",
  untitledVideo: "Video sin título",
  quality: "Calidad",
  qualityHint: "Una calidad para todos los videos; el que no la tenga recibe la siguiente más baja.",
  presets: {
    best: "Mejor",
    "1080p": "Hasta 1080p",
    "720p": "Hasta 720p",
    audio: "Solo audio",
  },
  autoCaptions: "Usar subtítulos automáticos donde no haya subtítulos",
  noFfmpeg: "No se encontró FFmpeg: solo se pueden descargar calidades con su propio sonido, que en YouTube son como mucho 360p.",
  folder: "Carpeta",
  folderHint: "Se crea dentro de la carpeta de al lado. Vacío: sin carpeta propia.",
  saveIn: "Guardar en",
  submit: { one: "Descargar {count} video", many: "Descargar {count} videos", other: "Descargar {count} videos" },
  addToList: { one: "Añadir {count} video", many: "Añadir {count} videos", other: "Añadir {count} videos" },
  added: { one: "Se añadió {count} video.", many: "Se añadieron {count} videos.", other: "Se añadieron {count} videos." },
  skipped: { one: "{count} ya estaba en la lista.", many: "{count} ya estaban en la lista.", other: "{count} ya estaban en la lista." },
};
