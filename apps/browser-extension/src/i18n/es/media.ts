// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  sitePlaylist: "Lista de {site} · videos elegidos en Downpipe",
  siteChannel: "Canal de {site} · videos elegidos en Downpipe",
  thisSitePlaylist: "Esta lista de reproducción de {site}",
  thisSiteChannel: "Este canal de {site}",
  updateForLists: "Actualiza Downpipe para descargar listas de reproducción",
  hls: "Transmisión HLS",
  dash: "Transmisión DASH",
  hlsQualities: { one: "HLS · {count} calidad", many: "HLS · {count} calidades", other: "HLS · {count} calidades" },
  fileAudio: "Audio {format}",
  fileVideo: "Video {format}",
  audio: "Audio",
  video: "Video",
  siteVideo: "Video de {site}",
  thisSiteVideo: "Este video de {site}",
  siteVideoQuality: "Video de {site} · calidad elegida en Downpipe",
  updateForStreams: "Actualiza Downpipe para descargar transmisiones",
  updateForPages: "Actualiza Downpipe para descargar videos de páginas",
  emptyOff: "Actívalo para ver los videos y audios que reproducen las páginas. Antes, el navegador pide acceso a todos los sitios.",
  emptyExcluded: "No se muestra aquí: {site} es un sitio excluido en Filtros.",
  emptyNothing: "Todavía nada. Reproduce el video; si ya se estaba reproduciendo, vuelve a cargar la página.",
};
