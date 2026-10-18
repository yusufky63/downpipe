// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "Playlist di {site}",
    channel: "Canale di {site}",
  },
  untitled: "Playlist",
  reading: "Richiesta dei video a yt-dlp…",
  unavailable: "Questa playlist o questo canale non è disponibile: è privato, rimosso o bloccato nella tua zona.",
  empty: "yt-dlp non ha trovato video in questo elenco.",
  truncated: { one: "Viene elencato solo il primo video.", many: "Vengono elencati solo i primi {count} video.", other: "Vengono elencati solo i primi {count} video." },
  filter: "Filtra per titolo",
  selectAll: "Seleziona tutto",
  selectNone: "Deseleziona tutto",
  selected: { one: "{count} di {total} selezionato", many: "{count} di {total} selezionati", other: "{count} di {total} selezionati" },
  rowsLabel: "Video",
  notAvailable: "Non disponibile",
  untitledVideo: "Video senza titolo",
  quality: "Qualità",
  qualityHint: "Una qualità per tutti i video; un video che non la offre riceve quella immediatamente inferiore.",
  presets: {
    best: "Migliore",
    "1080p": "Fino a 1080p",
    "720p": "Fino a 720p",
    audio: "Solo audio",
  },
  autoCaptions: "Usa i sottotitoli automatici dove mancano i sottotitoli",
  noFfmpeg: "FFmpeg non trovato: si possono scaricare solo le qualità con un proprio audio, che su YouTube significa al massimo 360p.",
  folder: "Cartella",
  folderHint: "Viene creata nella cartella accanto. Vuoto: nessuna cartella a parte.",
  saveIn: "Salva in",
  submit: { one: "Scarica {count} video", many: "Scarica {count} video", other: "Scarica {count} video" },
  addToList: { one: "Aggiungi {count} video", many: "Aggiungi {count} video", other: "Aggiungi {count} video" },
  added: { one: "{count} video aggiunto.", many: "{count} video aggiunti.", other: "{count} video aggiunti." },
  skipped: { one: "{count} era già nell'elenco.", many: "{count} erano già nell'elenco.", other: "{count} erano già nell'elenco." },
};
