// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  sitePlaylist: "Playlist di {site} · video scelti in Downpipe",
  siteChannel: "Canale di {site} · video scelti in Downpipe",
  thisSitePlaylist: "Questa playlist di {site}",
  thisSiteChannel: "Questo canale di {site}",
  updateForLists: "Aggiorna Downpipe per scaricare le playlist",
  hls: "Flusso HLS",
  dash: "Flusso DASH",
  hlsQualities: { one: "HLS · {count} qualità", many: "HLS · {count} qualità", other: "HLS · {count} qualità" },
  fileAudio: "Audio {format}",
  fileVideo: "Video {format}",
  audio: "Audio",
  video: "Video",
  siteVideo: "Video di {site}",
  thisSiteVideo: "Questo video di {site}",
  siteVideoQuality: "Video di {site} · qualità scelta in Downpipe",
  updateForStreams: "Aggiorna Downpipe per scaricare i flussi",
  updateForPages: "Aggiorna Downpipe per scaricare i video dalle pagine",
  emptyOff:
    "Attiva per elencare i video e l'audio riprodotti dalle pagine. Prima il browser chiede l'accesso a tutti i siti.",
  emptyExcluded: "Non elencato qui: {site} è un sito escluso in Filtri.",
  emptyNothing: "Ancora niente. Riproduci il video; se era già in riproduzione, ricarica la pagina.",
};
